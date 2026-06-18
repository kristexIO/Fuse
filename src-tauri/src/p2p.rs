use crate::error::{FuseError, FuseResult};
use crate::models::{
    FuseShareTicket, PeerSource, ShareTicketDisplay, ShareTicketItem, SharedProviderFile,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use bytes::Bytes;
use futures_lite::StreamExt;
use iroh::{Endpoint, NodeAddr};
use iroh_gossip::{
    net::{Event, Gossip, GossipEvent, GOSSIP_ALPN},
    proto::TopicId,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferControl {
    Continue,
    Pause,
    Cancel,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferRequest {
    file_hash: String,
    offset_bytes: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderAnnouncement {
    kind: String,
    manifest_hash: String,
    provider: PeerSource,
    file_hashes: Vec<String>,
    announced_at: i64,
}

#[derive(Debug, Clone)]
struct TopicAnnouncement {
    topic: TopicId,
    announcement: ProviderAnnouncement,
}

struct DownloadControl<'a, P, C>
where
    P: FnMut(i64, i64) -> FuseResult<()>,
    C: FnMut() -> FuseResult<TransferControl>,
{
    peer_count: i64,
    download_limit_kbps: Option<i64>,
    on_progress: &'a mut P,
    should_cancel: &'a mut C,
}

pub struct P2pService {
    runtime: Runtime,
    endpoint: Option<Endpoint>,
    gossip: Option<Gossip>,
    node_addr: Option<NodeAddr>,
    accept_task: Option<JoinHandle<()>>,
    announcement_tasks: HashMap<String, JoinHandle<()>>,
    shared_files: Arc<Mutex<HashMap<String, SharedProviderFile>>>,
    app_data_dir: PathBuf,
    last_error: Option<String>,
    upload_limit_kbps: Arc<std::sync::atomic::AtomicI64>,
}

impl P2pService {
    pub fn new(app_data_dir: PathBuf) -> FuseResult<Self> {
        let runtime = Runtime::new().map_err(|error| FuseError::P2p(error.to_string()))?;
        Ok(Self {
            runtime,
            endpoint: None,
            gossip: None,
            node_addr: None,
            accept_task: None,
            announcement_tasks: HashMap::new(),
            shared_files: Arc::new(Mutex::new(HashMap::new())),
            app_data_dir,
            last_error: None,
            upload_limit_kbps: Arc::new(std::sync::atomic::AtomicI64::new(0)),
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

    pub fn start(
        &mut self,
        files: Vec<SharedProviderFile>,
        upload_limit_kbps: Option<i64>,
    ) -> FuseResult<P2pRuntimeStatus> {
        self.replace_shared_files(files)?;
        self.set_upload_limit(upload_limit_kbps);

        if self.endpoint.is_none() {
            let shared_files = self.shared_files.clone();
            let upload_limit = self.upload_limit_kbps.clone();
            let (endpoint, gossip, node_addr) = self.runtime.block_on(async {
                let endpoint = Endpoint::builder()
                    .alpns(vec![ALPN.to_vec(), GOSSIP_ALPN.to_vec()])
                    .bind()
                    .await
                    .map_err(|error| FuseError::P2p(error.to_string()))?;
                let gossip = Gossip::builder()
                    .spawn(endpoint.clone())
                    .await
                    .map_err(|error| FuseError::P2p(error.to_string()))?;
                let node_addr = addressable_node_addr(&endpoint).await?;
                Ok::<_, FuseError>((endpoint, gossip, node_addr))
            })?;

            let accept_endpoint = endpoint.clone();
            let accept_gossip = gossip.clone();
            self.accept_task = Some(self.runtime.spawn(async move {
                accept_loop(accept_endpoint, accept_gossip, shared_files, upload_limit).await;
            }));
            self.node_addr = Some(node_addr);
            self.gossip = Some(gossip);
            self.endpoint = Some(endpoint);
        }

        self.last_error = None;
        Ok(self.status())
    }

    pub fn stop(&mut self) -> FuseResult<P2pRuntimeStatus> {
        for (_, task) in self.announcement_tasks.drain() {
            task.abort();
        }

        if let Some(task) = self.accept_task.take() {
            task.abort();
        }

        if let Some(endpoint) = self.endpoint.take() {
            self.runtime.block_on(async {
                endpoint.close().await;
            });
        }

        self.gossip = None;
        self.node_addr = None;
        Ok(self.status())
    }

    pub fn set_upload_limit(&self, upload_limit_kbps: Option<i64>) {
        self.upload_limit_kbps.store(
            upload_limit_kbps.unwrap_or_default().max(0),
            std::sync::atomic::Ordering::Relaxed,
        );
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

    pub fn sync_provider_announcements(
        &mut self,
        tickets: Vec<FuseShareTicket>,
    ) -> FuseResult<()> {
        let Some(gossip) = self.gossip.clone() else {
            return Ok(());
        };
        let provider = self.provider()?;
        let mut active_topics = HashSet::new();
        let mut announcements = Vec::new();

        for ticket in tickets {
            let topic_key = ticket.swarm_topic.clone();
            active_topics.insert(topic_key.clone());
            if self.announcement_tasks.contains_key(&topic_key) {
                continue;
            }

            let topic = topic_from_hex(&ticket.swarm_topic)?;
            let announcement = ProviderAnnouncement {
                kind: "provider".to_string(),
                manifest_hash: ticket.manifest_hash,
                provider: provider.clone(),
                file_hashes: ticket.items.iter().map(|item| item.file_hash.clone()).collect(),
                announced_at: unix_now(),
            };
            announcements.push((topic_key, TopicAnnouncement { topic, announcement }));
        }

        let stale_topics = self
            .announcement_tasks
            .keys()
            .filter(|topic| !active_topics.contains(*topic))
            .cloned()
            .collect::<Vec<_>>();
        for topic in stale_topics {
            if let Some(task) = self.announcement_tasks.remove(&topic) {
                task.abort();
            }
        }

        for (topic_key, announcement) in announcements {
            let gossip = gossip.clone();
            let task = self.runtime.spawn(async move {
                announce_provider_loop(gossip, announcement).await;
            });
            self.announcement_tasks.insert(topic_key, task);
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

    fn discover_providers(&self, ticket: &FuseShareTicket) -> FuseResult<Vec<PeerSource>> {
        let Some(endpoint) = self.endpoint.clone() else {
            return Ok(ticket.providers.clone());
        };
        let Some(gossip) = self.gossip.clone() else {
            return Ok(ticket.providers.clone());
        };

        let topic = topic_from_hex(&ticket.swarm_topic)?;
        let manifest_hash = ticket.manifest_hash.clone();
        let bootstrap = ticket
            .providers
            .iter()
            .filter_map(|provider| serde_json::from_value::<NodeAddr>(provider.addr.clone()).ok())
            .collect::<Vec<_>>();
        let original = ticket.providers.clone();

        self.runtime.block_on(async move {
            for addr in &bootstrap {
                endpoint
                    .add_node_addr(addr.clone())
                    .map_err(|error| FuseError::P2p(error.to_string()))?;
            }

            let bootstrap_ids = bootstrap.iter().map(|addr| addr.node_id).collect::<Vec<_>>();
            let mut topic = gossip
                .subscribe(topic, bootstrap_ids)
                .map_err(|error| FuseError::P2p(error.to_string()))?;
            let mut providers = original;
            let mut seen = providers
                .iter()
                .map(|provider| provider.node_id.clone())
                .collect::<HashSet<_>>();
            let deadline = tokio::time::sleep(Duration::from_secs(2));
            tokio::pin!(deadline);

            loop {
                tokio::select! {
                    _ = &mut deadline => break,
                    event = topic.next() => {
                        let Some(event) = event else {
                            break;
                        };
                        if let Ok(Event::Gossip(GossipEvent::Received(message))) = event {
                            if let Ok(announcement) = serde_json::from_slice::<ProviderAnnouncement>(&message.content) {
                                if announcement.kind == "provider"
                                    && announcement.manifest_hash == manifest_hash
                                    && seen.insert(announcement.provider.node_id.clone())
                                {
                                    if let Ok(addr) = serde_json::from_value::<NodeAddr>(announcement.provider.addr.clone()) {
                                        let _ = endpoint.add_node_addr(addr);
                                    }
                                    providers.push(announcement.provider);
                                }
                            }
                        }
                    }
                }
            }

            Ok::<_, FuseError>(providers)
        })
    }

    pub fn download_ticket(
        &mut self,
        encoded_ticket: &str,
        import_dir: &Path,
        download_limit_kbps: Option<i64>,
        mut on_progress: impl FnMut(i64, i64) -> FuseResult<()>,
        mut should_cancel: impl FnMut() -> FuseResult<TransferControl>,
    ) -> FuseResult<DownloadOutcome> {
        let ticket = decode_ticket(encoded_ticket)?;
        let endpoint = self.endpoint.as_ref().ok_or_else(|| {
            FuseError::P2p("P2P must be running before downloading".to_string())
        })?;
        let providers = self.discover_providers(&ticket)?;
        fs::create_dir_all(import_dir)?;

        let mut output_paths = Vec::new();
        let mut downloaded_bytes = 0_i64;
        let mut seeded_files = Vec::new();

        for item in &ticket.items {
            let output_path = unique_output_path(import_dir, item)?;
            if verified_existing_file(&output_path, &item.file_hash, item.size_bytes) {
                downloaded_bytes += item.size_bytes;
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
                continue;
            }

            let part_path = part_path_for(&output_path);
            if verified_existing_file(&part_path, &item.file_hash, item.size_bytes) {
                if output_path.exists() {
                    fs::remove_file(&output_path)?;
                }
                fs::rename(&part_path, &output_path)?;
                downloaded_bytes += item.size_bytes;
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
                continue;
            }

            let mut control = DownloadControl {
                peer_count: providers.len() as i64,
                download_limit_kbps,
                on_progress: &mut on_progress,
                should_cancel: &mut should_cancel,
            };
            let copied = self.runtime.block_on(download_one(
                endpoint.clone(),
                &providers,
                item,
                &part_path,
                &mut control,
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

fn topic_from_hex(value: &str) -> FuseResult<TopicId> {
    let bytes = hex::decode(value)
        .map_err(|error| FuseError::Validation(format!("Invalid swarm topic: {error}")))?;
    let topic: [u8; 32] = bytes
        .try_into()
        .map_err(|_| FuseError::Validation("Swarm topic must be a 32-byte BLAKE3 hash".to_string()))?;
    Ok(TopicId::from_bytes(topic))
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or_default()
}

async fn addressable_node_addr(endpoint: &Endpoint) -> FuseResult<NodeAddr> {
    let mut direct_addresses = endpoint.direct_addresses();
    let mut home_relay = endpoint.home_relay();
    tokio::select! {
        _ = direct_addresses.initialized() => {}
        _ = home_relay.initialized() => {}
    }
    let direct_addresses = direct_addresses
        .get()
        .map_err(|error| FuseError::P2p(error.to_string()))?
        .unwrap_or_default()
        .into_iter()
        .map(|address| address.addr);
    let home_relay = home_relay
        .get()
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    Ok(NodeAddr::from_parts(endpoint.node_id(), home_relay, direct_addresses))
}

async fn announce_provider_loop(gossip: Gossip, announcement: TopicAnnouncement) {
    let mut topic = match gossip.subscribe(announcement.topic, Vec::new()) {
        Ok(topic) => topic,
        Err(_) => return,
    };
    let payload = match serde_json::to_vec(&announcement.announcement) {
        Ok(payload) => Bytes::from(payload),
        Err(_) => return,
    };
    let mut interval = tokio::time::interval(Duration::from_secs(15));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let _ = topic.broadcast(payload.clone()).await;
            }
            event = topic.next() => {
                if event.is_none() {
                    break;
                }
            }
        }
    }
}

async fn accept_loop(
    endpoint: Endpoint,
    gossip: Gossip,
    shared_files: Arc<Mutex<HashMap<String, SharedProviderFile>>>,
    upload_limit_kbps: Arc<std::sync::atomic::AtomicI64>,
) {
    while let Some(incoming) = endpoint.accept().await {
        let shared_files = shared_files.clone();
        let gossip = gossip.clone();
        let upload_limit_kbps = upload_limit_kbps.clone();
        tokio::spawn(async move {
            let Ok(mut connecting) = incoming.accept() else {
                return;
            };
            let Ok(alpn) = connecting.alpn().await else {
                return;
            };
            let Ok(connection) = connecting.await else {
                return;
            };

            if alpn == ALPN {
                let _ = handle_connection(connection, shared_files, upload_limit_kbps).await;
            } else if alpn == GOSSIP_ALPN {
                let _ = gossip.handle_connection(connection).await;
            }
        });
    }
}

async fn handle_connection(
    connection: iroh::endpoint::Connection,
    shared_files: Arc<Mutex<HashMap<String, SharedProviderFile>>>,
    upload_limit_kbps: Arc<std::sync::atomic::AtomicI64>,
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
    if request.offset_bytes > shared_file.size_bytes.max(0) as u64 {
        write_header(
            &mut send,
            &TransferResponseHeader {
                ok: false,
                error: Some("Resume offset is past the end of the seeded file".to_string()),
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
    if request.offset_bytes > 0 {
        file.seek(SeekFrom::Start(request.offset_bytes))
            .await
            .map_err(|error| FuseError::P2p(error.to_string()))?;
    }
    throttled_copy_upload(&mut file, &mut send, upload_limit_kbps).await?;
    send.finish()
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    Ok(())
}

async fn download_one<P, C>(
    endpoint: Endpoint,
    providers: &[PeerSource],
    item: &ShareTicketItem,
    part_path: &Path,
    control: &mut DownloadControl<'_, P, C>,
) -> FuseResult<u64>
where
    P: FnMut(i64, i64) -> FuseResult<()>,
    C: FnMut() -> FuseResult<TransferControl>,
{
    let mut last_error = "no providers available".to_string();

    for provider in providers {
        let node_addr: NodeAddr = serde_json::from_value(provider.addr.clone())?;
        for attempt in 0..3 {
            match try_download_from_provider(
                endpoint.clone(),
                node_addr.clone(),
                item,
                part_path,
                control,
            )
            .await
            {
                Ok(copied) => return Ok(copied),
                Err(error) => {
                    if is_pause_error(&error) {
                        return Err(error);
                    }
                    last_error = error.to_string();
                    let _ = tokio::fs::remove_file(part_path).await;
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                    }
                }
            }
        }
    }

    Err(FuseError::P2p(last_error))
}

async fn try_download_from_provider<P, C>(
    endpoint: Endpoint,
    provider: NodeAddr,
    item: &ShareTicketItem,
    part_path: &Path,
    control: &mut DownloadControl<'_, P, C>,
) -> FuseResult<u64>
where
    P: FnMut(i64, i64) -> FuseResult<()>,
    C: FnMut() -> FuseResult<TransferControl>,
{
    let connection = endpoint
        .connect(provider, ALPN)
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    let resume_from = resume_offset(part_path, item.size_bytes).await?;
    write_header(
        &mut send,
        &TransferRequest {
            file_hash: item.file_hash.clone(),
            offset_bytes: resume_from,
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

    let mut open_options = tokio::fs::OpenOptions::new();
    open_options.create(true).write(true);
    if resume_from > 0 {
        open_options.append(true);
    } else {
        open_options.truncate(true);
    }
    let mut file = open_options
        .open(part_path)
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    let copied = throttled_copy_download(
        &mut recv,
        &mut file,
        control,
        resume_from,
    )
    .await?;
    connection.close(0_u8.into(), b"done");
    Ok(copied)
}

async fn throttled_copy_download<R, W, P, C>(
    reader: &mut R,
    writer: &mut W,
    control: &mut DownloadControl<'_, P, C>,
    resume_from: u64,
) -> FuseResult<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    P: FnMut(i64, i64) -> FuseResult<()>,
    C: FnMut() -> FuseResult<TransferControl>,
{
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut copied = resume_from;
    let limit = control.download_limit_kbps.unwrap_or_default().max(0);

    loop {
        match (control.should_cancel)()? {
            TransferControl::Continue => {}
            TransferControl::Pause => return Err(FuseError::P2p("download paused".to_string())),
            TransferControl::Cancel => return Err(FuseError::P2p("download cancelled".to_string())),
        }

        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|error| FuseError::P2p(error.to_string()))?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buffer[..read])
            .await
            .map_err(|error| FuseError::P2p(error.to_string()))?;
        copied = copied.saturating_add(read as u64);
        (control.on_progress)(copied.min(i64::MAX as u64) as i64, control.peer_count)?;
        throttle_bytes(read, limit).await;
    }

    writer
        .flush()
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    Ok(copied)
}

async fn throttled_copy_upload<R, W>(
    reader: &mut R,
    writer: &mut W,
    upload_limit_kbps: Arc<std::sync::atomic::AtomicI64>,
) -> FuseResult<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut copied = 0_u64;

    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|error| FuseError::P2p(error.to_string()))?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buffer[..read])
            .await
            .map_err(|error| FuseError::P2p(error.to_string()))?;
        copied = copied.saturating_add(read as u64);
        let limit = upload_limit_kbps.load(std::sync::atomic::Ordering::Relaxed);
        throttle_bytes(read, limit).await;
    }

    writer
        .flush()
        .await
        .map_err(|error| FuseError::P2p(error.to_string()))?;
    Ok(copied)
}

async fn throttle_bytes(bytes: usize, limit_kbps: i64) {
    if limit_kbps <= 0 || bytes == 0 {
        return;
    }

    let bytes_per_second = (limit_kbps as u64).saturating_mul(1024);
    if bytes_per_second == 0 {
        return;
    }
    let millis = ((bytes as u64).saturating_mul(1000) / bytes_per_second).max(1);
    tokio::time::sleep(Duration::from_millis(millis)).await;
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
    let suffix = item.file_hash.chars().take(8).collect::<String>();

    for index in 0..1000 {
        let candidate = if index == 0 {
            import_dir.join(format!("{base}.{extension}"))
        } else if index == 1 {
            import_dir.join(format!("{base}-{suffix}.{extension}"))
        } else {
            import_dir.join(format!("{base}-{suffix}-{index}.{extension}"))
        };

        if !candidate.exists()
            || verified_existing_file(&candidate, &item.file_hash, item.size_bytes)
        {
            return Ok(candidate);
        }
    }

    Err(FuseError::P2p("Could not choose a unique output path".to_string()))
}

fn part_path_for(output_path: &Path) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    output_path.with_file_name(format!("{file_name}.part"))
}

fn verified_existing_file(path: &Path, file_hash: &str, size_bytes: i64) -> bool {
    if size_bytes < 0 {
        return false;
    }
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    metadata.is_file()
        && metadata.len() == size_bytes as u64
        && hash_file(path)
            .map(|actual| actual == file_hash)
            .unwrap_or(false)
}

async fn resume_offset(path: &Path, size_bytes: i64) -> FuseResult<u64> {
    let Ok(metadata) = tokio::fs::metadata(path).await else {
        return Ok(0);
    };
    let expected_size = size_bytes.max(0) as u64;
    let current = metadata.len();
    if !metadata.is_file() || current >= expected_size {
        let _ = tokio::fs::remove_file(path).await;
        return Ok(0);
    }
    Ok(current)
}

fn is_pause_error(error: &FuseError) -> bool {
    error.to_string().contains("download paused")
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
    use tempfile::tempdir;

    static P2P_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    #[test]
    fn two_local_nodes_share_and_download_one_track() {
        let _guard = P2P_TEST_LOCK.lock().unwrap();
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();
        let source_path = source_dir.path().join("signal.flac");
        fs::write(&source_path, b"fuse p2p integration track").unwrap();
        let file_hash = hash_file(&source_path).unwrap();

        let mut provider = P2pService::new(source_dir.path().join("provider")).unwrap();
        provider
            .start(
                vec![SharedProviderFile {
                    file_hash: file_hash.clone(),
                    path: source_path.to_string_lossy().to_string(),
                    title: "Signal Bloom".to_string(),
                    artist: Some("Northline Archive".to_string()),
                    album: Some("Late Focus".to_string()),
                    format: "flac".to_string(),
                    size_bytes: 26,
                }],
                None,
            )
            .unwrap();
        let ticket = build_ticket(
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
                format: "flac".to_string(),
                file_hash,
                size_bytes: 26,
            }],
            provider.provider().unwrap(),
            unix_now(),
        )
        .unwrap();
        provider
            .sync_provider_announcements(vec![ticket.clone()])
            .unwrap();

        let encoded = encode_ticket(&ticket).unwrap();
        let mut downloader = P2pService::new(target_dir.path().join("downloader")).unwrap();
        downloader.start(Vec::new(), None).unwrap();
        let outcome = downloader
            .download_ticket(&encoded, target_dir.path(), None, |_, _| Ok(()), || Ok(TransferControl::Continue))
            .unwrap();

        assert_eq!(outcome.downloaded_bytes, 26);
        assert_eq!(outcome.seeded_files.len(), 1);
        assert_eq!(fs::read(&outcome.output_paths[0]).unwrap(), b"fuse p2p integration track");

        downloader.stop().unwrap();
        provider.stop().unwrap();
    }

    #[test]
    fn downloaded_node_can_seed_after_original_provider_stops() {
        let _guard = P2P_TEST_LOCK.lock().unwrap();
        let source_dir = tempdir().unwrap();
        let middle_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();
        let source_path = source_dir.path().join("relay.flac");
        fs::write(&source_path, b"fuse p2p reshare track").unwrap();
        let file_hash = hash_file(&source_path).unwrap();
        let item = ShareTicketItem {
            title: "Relay".to_string(),
            artist: None,
            album: None,
            format: "flac".to_string(),
            file_hash: file_hash.clone(),
            size_bytes: 22,
        };

        let mut a = P2pService::new(source_dir.path().join("a")).unwrap();
        a.start(
            vec![SharedProviderFile {
                file_hash,
                path: source_path.to_string_lossy().to_string(),
                title: "Relay".to_string(),
                artist: None,
                album: None,
                format: "flac".to_string(),
                size_bytes: 22,
            }],
            None,
        )
        .unwrap();
        let mut ticket = build_ticket(
            "track",
            ShareTicketDisplay {
                title: "Relay".to_string(),
                artist: None,
                album: None,
                item_count: 1,
            },
            vec![item],
            a.provider().unwrap(),
            unix_now(),
        )
        .unwrap();
        let encoded_a = encode_ticket(&ticket).unwrap();

        let mut b = P2pService::new(middle_dir.path().join("b")).unwrap();
        b.start(Vec::new(), None).unwrap();
        let b_outcome = b
            .download_ticket(&encoded_a, middle_dir.path(), None, |_, _| Ok(()), || Ok(TransferControl::Continue))
            .unwrap();
        b.add_shared_files(b_outcome.seeded_files).unwrap();
        ticket.providers = vec![b.provider().unwrap()];
        b.sync_provider_announcements(vec![ticket.clone()]).unwrap();
        a.stop().unwrap();

        let mut c = P2pService::new(target_dir.path().join("c")).unwrap();
        c.start(Vec::new(), None).unwrap();
        let encoded_b = encode_ticket(&ticket).unwrap();
        let c_outcome = c
            .download_ticket(&encoded_b, target_dir.path(), None, |_, _| Ok(()), || Ok(TransferControl::Continue))
            .unwrap();

        assert_eq!(fs::read(&c_outcome.output_paths[0]).unwrap(), b"fuse p2p reshare track");

        c.stop().unwrap();
        b.stop().unwrap();
    }
}
