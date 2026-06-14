use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub format: String,
    pub size_bytes: i64,
    pub modified_at: i64,
    pub missing_tags: bool,
    pub artwork_id: Option<String>,
    pub artwork_uri: Option<String>,
    pub has_artwork: bool,
    pub lyrics: Option<String>,
    pub date_added: i64,
    pub play_count: i64,
    pub last_played_at: Option<i64>,
    pub is_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artwork {
    pub track_id: i64,
    pub mime: String,
    pub data_url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackQuery {
    pub search: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub name: String,
    pub artist: Option<String>,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub name: String,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
    pub created_at: i64,
    pub description: Option<String>,
    pub artwork_uri: Option<String>,
    pub updated_at: i64,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFolder {
    pub id: i64,
    pub path: String,
    pub added_at: i64,
    pub last_scanned_at: Option<i64>,
    pub ignored_patterns: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub register_folders: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanJob {
    pub id: i64,
    pub state: String,
    pub total_files: Option<usize>,
    pub scanned_files: usize,
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<ScanError>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventLog {
    pub id: i64,
    pub level: String,
    pub message: String,
    pub path: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDiagnostics {
    pub app_data_dir: Option<String>,
    pub log_path: Option<String>,
    pub recent_events: Vec<EventLog>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub first_run_complete: bool,
    pub reduced_motion: bool,
    pub active_layout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PeerSource {
    pub node_id: String,
    pub addr: serde_json::Value,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShareTicketItem {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub format: String,
    pub file_hash: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShareTicketDisplay {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub item_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FuseShareTicket {
    pub version: u8,
    pub scope: String,
    pub manifest_hash: String,
    pub swarm_topic: String,
    pub providers: Vec<PeerSource>,
    pub display: ShareTicketDisplay,
    pub items: Vec<ShareTicketItem>,
    pub size_bytes: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SharedItem {
    pub id: i64,
    pub scope: String,
    pub track_id: Option<i64>,
    pub playlist_id: Option<i64>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub manifest_hash: String,
    pub swarm_topic: String,
    pub size_bytes: i64,
    pub item_count: i64,
    pub ticket: String,
    pub state: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferTask {
    pub id: i64,
    pub direction: String,
    pub status: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub manifest_hash: String,
    pub swarm_topic: String,
    pub size_bytes: i64,
    pub downloaded_bytes: i64,
    pub peer_count: i64,
    pub ticket: String,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct P2pSettings {
    pub enabled: bool,
    pub auto_seed_downloads: bool,
    pub import_dir: Option<String>,
    pub upload_limit_kbps: Option<i64>,
    pub download_limit_kbps: Option<i64>,
}

impl Default for P2pSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_seed_downloads: true,
            import_dir: None,
            upload_limit_kbps: None,
            download_limit_kbps: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct P2pStatus {
    pub enabled: bool,
    pub running: bool,
    pub node_id: Option<String>,
    pub node_addr: Option<serde_json::Value>,
    pub active_shares: i64,
    pub active_downloads: i64,
    pub auto_seed_downloads: bool,
    pub import_dir: Option<String>,
    pub upload_limit_kbps: Option<i64>,
    pub download_limit_kbps: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SharedProviderFile {
    pub file_hash: String,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub format: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct ShareFileDraft {
    pub track_id: Option<i64>,
    pub file_hash: String,
    pub local_path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub format: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct P2pShareDraft {
    pub scope: String,
    pub track_id: Option<i64>,
    pub playlist_id: Option<i64>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub manifest_hash: String,
    pub swarm_topic: String,
    pub size_bytes: i64,
    pub item_count: i64,
    pub ticket: String,
    pub files: Vec<ShareFileDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackQueueItem {
    pub track_id: i64,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    pub engine: String,
    pub status: String,
    pub track_id: Option<i64>,
    pub position_ms: i64,
    pub duration_ms: Option<i64>,
    pub volume: f32,
    pub queue: Vec<i64>,
    pub queue_index: Option<usize>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutBlock {
    pub id: String,
    pub cols: u8,
    pub rows: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutProfile {
    pub name: String,
    pub theme: String,
    pub density: String,
    pub order: Vec<String>,
    pub hidden: Vec<String>,
    pub blocks: Vec<LayoutBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scanned_files: usize,
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: Vec<ScanError>,
}

impl ScanSummary {
    pub fn from_job(job: &ScanJob) -> Self {
        Self {
            scanned_files: job.scanned_files,
            added: job.added,
            updated: job.updated,
            skipped: job.skipped,
            errors: job.errors.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackDraft {
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub format: String,
    pub size_bytes: i64,
    pub modified_at: i64,
    pub missing_tags: bool,
    pub artwork_id: Option<String>,
    pub artwork_mime: Option<String>,
    pub artwork_data: Option<Vec<u8>>,
    pub lyrics: Option<String>,
}
