use crate::error::{FuseError, FuseResult};
use crate::models::{
    FuseShareTicket, PeerSource, ShareTicketDisplay, ShareTicketItem, SharedProviderFile,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use iroh::{Endpoint, NodeAddr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

const ALPN: &[u8] = b"fuse-swarm/1";
const TICKET_PREFIX: &str = "fuse-share:v1:";
const HEADER_LIMIT: usize = 32 * 1024;

#[derive(Debug, Clone)]
pub struct P2pRuntimeStatus {
    pub running: bool,
    pub node_id: Option<String>,
    pub node_addr: Option<serde_json::Value>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub output_paths: Vec<String>,
    pub downloaded_bytes: i64,
    pub seeded_files: Vec<SharedProviderFile>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferRequest {
    file_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferResponseHeader {
    ok: bool,
    error: Option<String>,
    file_hash: Option<String>,
    size_bytes: Option<i64>,
    title: Option<String>,
    format: Option<String>,
}

pub struct P2pService {
    runtime: Runtime,
    endpoint: Option<Endpoint>,
    node_addr: Option<NodeAddr>,
    accept_task: Option<JoinHandle<()>>,
    shared_files: Arc<Mutex<HashMap<String, SharedProviderFile>>>,
    app_data_dir: PathBuf,
    last_error: Option<String>,
}

impl P2pService {
    pub fn new(app_data_dir: PathBuf) -> FuseResult<Self> {
        let runtime = Runtime::new().map_err(|error| FuseError::P2p(error.to_string()))?;
        Ok(Self {
            runtime,
            endpoint: None,
            node_addr: None,
            accept_task: None,
            shared_files: Arc::new(Mutex::new(HashMap::new())),
            app_data_dir,
            last_error: None,
        })
    }

    pub fn default_import_dir(&self) -> PathBuf {
        self.app_data_dir.join("swarm-imports")
    }

    pub fn status(&self) -> P2pRuntimeStatus {
        P2pRuntimeStatus {
            running: self.endpoint.is_some(),
            node_id: self.node_addr.as_ref().map(|addr| addr.node_id.to_string()),
            node_addr: self
                .node_addr
                .as_ref()
                .and_then(|addr| serde_json::to_value(addr).ok()),
            last_error: self.last_error.clone(),
        }
    }

    pub fn start(&mut self, files: Vec<SharedProviderFile>) -> FuseResult<P2pRuntimeStatus> {
        self.replace_shared_files(files)?;

        if self.endpoint.is_none() {
            let shared_files = self.shared_files.clone();
            let (endpoint, node_addr) = self.runtime.block_on(async {
                let endpoint = Endpoint::builder()
                    .alpns(vec![ALPN.to_vec()])
                    .bind()
                    .await
                    .map_err(|error| FuseError::P2p(error.to_string()))?;
                let node_addr = match tokio::time::timeout(Duration::from_secs(3), endpoint.node_addr()).await {
                    Ok(Ok(addr)) => addr,
                    Ok(Err(error)) => {
                        return Err(FuseError::P2p(error.to_string()));
                    }
                    Err(_) => NodeAddr::new(endpoint.node_id()),
                };
                Ok::<_, FuseError>((endpoint, node_addr))
            })?;

            let accept_endpoint = endpoint.clone();
            self.accept_task = Some(self.runtime.spawn(async move {
                accept_loop(accept_endpoint, shared_files).await;
            }));
            self.node_addr = Some(node_addr);
            self.endpoint = Some(endpoint);
        }

        self.last_error = None;
        Ok(self.status())
    }

    pub fn stop(&mut self) -> FuseResult<P2pRuntimeStatus> {
        if let Some(task) = self.accept_task.take() {
            task.abort();
        }

        if let Some(endpoint) = self.endpoint.take() {
            self.runtime.block_on(async {
                endpoint.close().await;
            });
        }

        self.node_addr = None;
        Ok(self.status())
    }

    pub fn replace_shared_files(&mut self, files: Vec<SharedProviderFile>) -> FuseResult<()> {
        let mut shared = self
            .shared_files
            .lock()
            .map_err(|_| FuseError::P2p("shared file registry lock failed".to_string()))?;
        shared.clear();
        for file in files {
            shared.insert(file.file_hash.clone(), file);
        }
        Ok(())
    }

    pub fn add_shared_files(&mut self, files: Vec<SharedProviderFile>) -> FuseResult<()> {
        let mut shared = self
            .shared_files
            .lock()
            .map_err(|_| FuseError::P2p("shared file registry lock failed".to_string()))?;
        for file in files {
            shared.insert(file.file_hash.clone(), file);
        }
        Ok(())
    }

    pub fn provider(&self) -> FuseResult<PeerSource> {
        let addr = self.node_addr.as_ref().ok_or_else(|| {
            FuseError::P2p("P2P must be running before creating a share ticket".to_string())
        })?;
        Ok(PeerSource {
            node_id: addr.node_id.to_string(),
            addr: serde_json::to_value(addr)?,
            label: None,
        })
    }

    pub fn download_ticket(
        &mut self,
        encoded_ticket: &str,
        import_dir: &Path,
    ) -> FuseResult<DownloadOutcome> {
        let ticket = decode_ticket(encoded_ticket)?;
        let endpoint = self.endpoint.as_ref().ok_or_else(|| {
            FuseError::P2p("P2P must be running before downloading".to_string())
        })?;
        fs::create_dir_all(import_dir)?;

        let mut output_paths = Vec::new();
        let mut downloaded_bytes = 0_i64;
        let mut seeded_files = Vec::new();

        for item in &ticket.items {
            let output_path = unique_output_path(import_dir, item)?;
            let part_path = part_path_for(&output_path);
            let copied = self.runtime.block_on(download_one(
                endpoint.clone(),
                &ticket.providers,
                item,
                &part_path,
            ))?;
            let actual_hash = hash_file(&part_path)?;
            if actual_hash != item.file_hash {
                let _ = fs::remove_file(&part_path);
                return Err(FuseError::P2p(format!(
                    "hash mismatch for {}: expected {}, got {}",
                    item.title, item.file_hash, actual_hash
                )));
            }

            if output_path.exists() {
                fs::remove_file(&output_path)?;
            }
            fs::rename(&part_path, &output_path)?;
            downloaded_bytes += copied.min(i64::MAX as u64) as i64;
            let output = output_path.to_string_lossy().to_string();
            output_paths.push(output.clone());
            seeded_files.push(SharedProviderFile {
                file_hash: item.file_hash.clone(),
                path: output,
                title: item.title.clone(),
                artist: item.artist.clone(),
                album: item.album.clone(),
                format: item.format.clone(),
                size_bytes: item.size_bytes,
            });
        }

        self.add_shared_files(seeded_files.clone())?;

        Ok(DownloadOutcome {
            output_paths,
            downloaded_bytes,
            seeded_files,
        })
    }
}

pub fn encode_ticket(ticket: &FuseShareTicket) -> FuseResult<String> {
    let payload = serde_json::to_vec(ticket)?;
    Ok(format!("{TICKET_PREFIX}{}", URL_SAFE_NO_PAD.encode(payload)))
}

pub fn decode_ticket(value: &str) -> FuseResult<FuseShareTicket> {
    let payload = value
        .trim()
        .strip_prefix(TICKET_PREFIX)
        .ok_or_else(|| FuseError::Validation("Share ticket must start with fuse-share:v1:".to_string()))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|error| FuseError::Validation(format!("Malformed share ticket: {error}")))?;
    let ticket: FuseShareTicket = serde_json::from_slice(&bytes)?;

    if ticket.version != 1 {
        return Err(FuseError::Validation("Unsupported share ticket version".to_string()));
    }

    if ticket.items.is_empty() {
        return Err(FuseError::Validation("Share ticket has no items".to_string()));
    }

    if ticket.providers.is_empty() {
        return Err(FuseError::Validation("Share ticket has no providers".to_string()));
    }

    validate_ticket(&ticket)?;

    Ok(ticket)
}

pub fn build_ticket(
    scope: &str,
    display: ShareTicketDisplay,
    items: Vec<ShareTicketItem>,
    provider: PeerSource,
    created_at: i64,
) -> FuseResult<FuseShareTicket> {
    if items.is_empty() {
        return Err(FuseError::Validation("Cannot share an empty item list".to_string()));
    }

    let manifest_hash = manifest_hash(scope, &display, &items)?;
    let swarm_topic = hex::encode(blake3::hash(
        format!("fuse-swarm:{scope}:{manifest_hash}:{created_at}").as_bytes(),
    ).as_bytes());
    let size_bytes = items.iter().map(|item| item.size_bytes).sum();

    Ok(FuseShareTicket {
        version: 1,
        scope: scope.to_string(),
        manifest_hash,
        swarm_topic,
        providers: vec![provider],
        display,
        items,
        size_bytes,
        created_at,
    })
}

pub fn hash_file(path: &Path) -> FuseResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn manifest_hash(
    scope: &str,
    display: &ShareTicketDisplay,
    items: &[ShareTicketItem],
) -> FuseResult<String> {
    let payload = serde_json::to_vec(&(scope, display, items))?;
    Ok(blake3::hash(&payload).to_hex().to_string())
}

fn validate_ticket(ticket: &FuseShareTicket) -> FuseResult<()> {
    if !matches!(ticket.scope.as_str(), "track" | "playlist") {
        return Err(FuseError::Validation("Unsupported share scope".to_string()));
    }
    if !is_hex_hash(&ticket.manifest_hash) || !is_hex_hash(&ticket.swarm_topic) {
        return Err(FuseError::Validation("Share ticket contains an invalid hash".to_string()));
    }
    if ticket.display.item_count != ticket.items.len() as i64 {
        return Err(FuseError::Validation("Share ticket item count does not match manifest".to_string()));
    }
    if ticket.size_bytes <= 0 {
        return Err(FuseError::Validation("Share ticket has invalid size".to_string()));
    }

    let item_size = ticket.items.iter().map(|item| item.size_bytes).sum::<i64>();
    if item_size != ticket.size_bytes {
        return Err(FuseError::Validation("Share ticket size does not match manifest".to_string()));
    }

    for item in &ticket.items {
        if item.title.trim().is_empty() {
            return Err(FuseError::Validation("Share ticket contains an unnamed item".to_string()));
        }
        if !is_hex_hash(&item.file_hash) {
            return Err(FuseError::Validation("Share ticket contains an invalid file hash".to_string()));
        }
        if item.size_bytes <= 0 {
            return Err(FuseError::Validation("Share ticket contains an invalid file size".to_string()));
        }
        if !is_supported_audio_format(&item.format) {
            return Err(FuseError::Validation(format!(
                "Unsupported audio format in share ticket: {}",
                item.format
            )));
        }
    }

    Ok(())
}

fn is_hex_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_supported_audio_format(value: &str) -> bool {
    matches!(
        value.trim_start_matches('.').to_ascii_lowercase().as_str(),
        "flac" | "mp3" | "wav" | "m4a" | "aac" | "ogg" | "opus" | "alac" | "aiff" | "aif"
    )
}

async fn accept_loop(
    endpoint: Endpoint,
    shared_files: Arc<Mutex<HashMap<String, SharedProviderFile>>>,
) {
    while let Some(connecting) = endpoint.accept().await {
        let shared_files = shared_files.clone();
        tokio::spawn(async move {
            if let Ok(connection) = connecting.await {
                let _ = handle_connection(connection, shared_files).await;
            }
        });
    }
}

async fn handle_connection(
    connection: iroh::endpoint::Connection,
    shared_files: Arc<Mutex<HashMap<String, SharedProviderFile>>>,
) -> FuseResult<()> {
    let (mut send, mut recv) = connection
        .accept_bi()
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    let header = read_json_header(&mut recv, HEADER_LIMIT).await?;
    let request: TransferRequest = serde_json::from_str(&header)?;
    let shared_file = {
        let shared = shared_files
            .lock()
            .map_err(|_| FuseError::P2p("shared file registry lock failed".to_string()))?;
        shared.get(&request.file_hash).cloned()
    };

    let Some(shared_file) = shared_file else {
        write_header(
            &mut send,
            &TransferResponseHeader {
                ok: false,
                error: Some("Requested file is not being seeded".to_string()),
                file_hash: None,
                size_bytes: None,
                title: None,
                format: None,
            },
        )
        .await?;
        send.finish()
            .map_err(|error| FuseError::P2p(error.to_string()))?;
        return Ok(());
    };

    let path = PathBuf::from(&shared_file.path);
    if !path.exists() || !path.is_file() {
        write_header(
            &mut send,
            &TransferResponseHeader {
                ok: false,
                error: Some("Seeded file is no longer available".to_string()),
                file_hash: None,
                size_bytes: None,
                title: None,
                format: None,
            },
        )
        .await?;
        send.finish()
            .map_err(|error| FuseError::P2p(error.to_string()))?;
        return Ok(());
    }

    write_header(
        &mut send,
        &TransferResponseHeader {
            ok: true,
            error: None,
            file_hash: Some(shared_file.file_hash),
            size_bytes: Some(shared_file.size_bytes),
            title: Some(shared_file.title),
            format: Some(shared_file.format),
        },
    )
    .await?;
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    tokio::io::copy(&mut file, &mut send)
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    send.finish()
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    Ok(())
}

async fn download_one(
    endpoint: Endpoint,
    providers: &[PeerSource],
    item: &ShareTicketItem,
    part_path: &Path,
) -> FuseResult<u64> {
    let mut last_error = "no providers available".to_string();

    for provider in providers {
        let node_addr: NodeAddr = serde_json::from_value(provider.addr.clone())?;
        match try_download_from_provider(endpoint.clone(), node_addr, item, part_path).await {
            Ok(copied) => return Ok(copied),
            Err(error) => {
                last_error = error.to_string();
                let _ = tokio::fs::remove_file(part_path).await;
            }
        }
    }

    Err(FuseError::P2p(last_error))
}

async fn try_download_from_provider(
    endpoint: Endpoint,
    provider: NodeAddr,
    item: &ShareTicketItem,
    part_path: &Path,
) -> FuseResult<u64> {
    let connection = endpoint
        .connect(provider, ALPN)
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    write_header(
        &mut send,
        &TransferRequest {
            file_hash: item.file_hash.clone(),
        },
    )
    .await?;
    send.finish()
        .map_err(|error| FuseError::P2p(error.to_string()))?;

    let header = read_json_header(&mut recv, HEADER_LIMIT).await?;
    let response: TransferResponseHeader = serde_json::from_str(&header)?;
    if !response.ok {
        return Err(FuseError::P2p(
            response
                .error
                .unwrap_or_else(|| "provider refused the transfer".to_string()),
        ));
    }

    let mut file = tokio::fs::File::create(part_path)
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    let copied = tokio::io::copy(&mut recv, &mut file)
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    connection.close(0_u8.into(), b"done");
    Ok(copied)
}

async fn write_header<W, T>(writer: &mut W, value: &T) -> FuseResult<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let mut data = serde_json::to_vec(value)?;
    data.push(b'\n');
    writer
        .write_all(&data)
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))
}

async fn read_json_header<R>(reader: &mut R, max_len: usize) -> FuseResult<String>
where
    R: AsyncRead + Unpin,
{
    let mut data = Vec::new();
    let mut byte = [0_u8; 1];

    loop {
        let read = reader
            .read(&mut byte)
            .await
            .map_err(|error| FuseError::P2p(error.to_string()))?;
        if read == 0 {
            return Err(FuseError::P2p("connection closed before header".to_string()));
        }
        if byte[0] == b'\n' {
            break;
        }
        data.push(byte[0]);
        if data.len() > max_len {
            return Err(FuseError::P2p("transfer header is too large".to_string()));
        }
    }

    String::from_utf8(data).map_err(|error| FuseError::P2p(error.to_string()))
}

fn unique_output_path(import_dir: &Path, item: &ShareTicketItem) -> FuseResult<PathBuf> {
    let extension = item.format.to_ascii_lowercase();
    let extension = extension.trim_start_matches('.');
    let extension = if extension.is_empty() { "audio" } else { extension };
    let stem = sanitize_file_name(&item.title);
    let base = if stem.is_empty() {
        item.file_hash.chars().take(12).collect::<String>()
    } else {
        stem
    };
    let mut candidate = import_dir.join(format!("{base}.{extension}"));

    if candidate.exists() {
        let suffix = item.file_hash.chars().take(8).collect::<String>();
        candidate = import_dir.join(format!("{base}-{suffix}.{extension}"));
    }

    Ok(candidate)
}

fn part_path_for(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    output_path.with_file_name(format!("{file_name}.part"))
}

fn sanitize_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ticket() -> FuseShareTicket {
        build_ticket(
            "track",
            ShareTicketDisplay {
                title: "Signal Bloom".to_string(),
                artist: Some("Northline Archive".to_string()),
                album: Some("Late Focus".to_string()),
                item_count: 1,
            },
            vec![ShareTicketItem {
                title: "Signal Bloom".to_string(),
                artist: Some("Northline Archive".to_string()),
                album: Some("Late Focus".to_string()),
                format: "FLAC".to_string(),
                file_hash: "a".repeat(64),
                size_bytes: 123,
            }],
            PeerSource {
                node_id: "node".to_string(),
                addr: serde_json::json!({ "node_id": "node", "relay_url": null, "direct_addresses": [] }),
                label: None,
            },
            42,
        )
        .unwrap()
    }

    #[test]
    fn ticket_round_trip_uses_fuse_prefix() {
        let ticket = sample_ticket();
        let encoded = encode_ticket(&ticket).unwrap();
        let decoded = decode_ticket(&encoded).unwrap();

        assert!(encoded.starts_with(TICKET_PREFIX));
        assert_eq!(decoded, ticket);
    }

    #[test]
    fn ticket_payload_does_not_contain_local_paths() {
        let ticket = sample_ticket();
        let encoded = encode_ticket(&ticket).unwrap();

        assert!(!encoded.contains("C:/"));
        assert!(!encoded.contains("\\"));
    }

    #[test]
    fn malformed_ticket_is_rejected() {
        let error = decode_ticket("not-a-ticket").unwrap_err();

        assert!(error.to_string().contains("Share ticket"));
    }

    #[test]
    fn unsupported_ticket_audio_format_is_rejected() {
        let mut ticket = sample_ticket();
        ticket.items[0].format = "exe".to_string();
        let encoded = encode_ticket(&ticket).unwrap();
        let error = decode_ticket(&encoded).unwrap_err();

        assert!(error.to_string().contains("Unsupported audio format"));
    }
}
