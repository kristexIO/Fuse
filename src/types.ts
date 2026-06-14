export type ThemeName =
  | "obsidian"
  | "porcelain"
  | "oled"
  | "boreal"
  | "ember"
  | "violet"
  | "rose"
  | "graphite"
  | "lagoon"
  | "daybreak";

export type Density = "compact" | "comfortable" | "spacious";

export type ModuleId =
  | "library"
  | "now"
  | "collection"
  | "player"
  | "queue"
  | "mixer"
  | "swarm"
  | "playlists"
  | "stats";

export type CollectionView = "tracks" | "albums" | "folders";

export interface Track {
  id: number;
  path: string;
  title: string;
  artist?: string | null;
  album?: string | null;
  durationMs?: number | null;
  format: string;
  sizeBytes: number;
  modifiedAt: number;
  missingTags: boolean;
  artworkId?: string | null;
  artworkUri?: string | null;
  hasArtwork: boolean;
  lyrics?: string | null;
  dateAdded: number;
  playCount: number;
  lastPlayedAt?: number | null;
  isMissing: boolean;
}

export interface Artwork {
  trackId: number;
  mime: string;
  dataUrl: string;
}

export interface TrackQuery {
  search?: string;
  limit?: number;
}

export interface Album {
  name: string;
  artist?: string | null;
  trackCount: number;
}

export interface Artist {
  name: string;
  trackCount: number;
}

export interface Playlist {
  id: number;
  name: string;
  trackCount: number;
  createdAt: number;
  description?: string | null;
  artworkUri?: string | null;
  updatedAt: number;
  sortOrder: number;
}

export interface LibraryFolder {
  id: number;
  path: string;
  addedAt: number;
  lastScannedAt?: number | null;
  ignoredPatterns?: string | null;
}

export interface LayoutBlock {
  id: ModuleId;
  cols: number;
  rows: number;
}

export interface LayoutProfile {
  name: string;
  theme: ThemeName;
  density: Density;
  order: ModuleId[];
  hidden: ModuleId[];
  blocks: LayoutBlock[];
}

export interface ScanError {
  path: string;
  message: string;
}

export interface ScanSummary {
  scannedFiles: number;
  added: number;
  updated: number;
  skipped: number;
  errors: ScanError[];
}

export interface ScanOptions {
  registerFolders?: boolean;
}

export interface ScanJob {
  id: number;
  state: string;
  totalFiles?: number | null;
  scannedFiles: number;
  added: number;
  updated: number;
  skipped: number;
  errors: ScanError[];
  startedAt: number;
  finishedAt?: number | null;
}

export interface EventLog {
  id: number;
  level: string;
  message: string;
  path?: string | null;
  createdAt: number;
}

export interface AppDiagnostics {
  appDataDir?: string | null;
  logPath?: string | null;
  recentEvents: EventLog[];
}

export interface AppSettings {
  firstRunComplete: boolean;
  reducedMotion: boolean;
  activeLayout?: string | null;
}

export interface PeerSource {
  nodeId: string;
  addr: unknown;
  label?: string | null;
}

export interface ShareTicketItem {
  title: string;
  artist?: string | null;
  album?: string | null;
  format: string;
  fileHash: string;
  sizeBytes: number;
}

export interface ShareTicketDisplay {
  title: string;
  artist?: string | null;
  album?: string | null;
  itemCount: number;
}

export interface FuseShareTicket {
  version: number;
  scope: string;
  manifestHash: string;
  swarmTopic: string;
  providers: PeerSource[];
  display: ShareTicketDisplay;
  items: ShareTicketItem[];
  sizeBytes: number;
  createdAt: number;
}

export interface SharedItem {
  id: number;
  scope: string;
  trackId?: number | null;
  playlistId?: number | null;
  title: string;
  artist?: string | null;
  album?: string | null;
  manifestHash: string;
  swarmTopic: string;
  sizeBytes: number;
  itemCount: number;
  ticket: string;
  state: string;
  createdAt: number;
  updatedAt: number;
  revokedAt?: number | null;
}

export interface TransferTask {
  id: number;
  direction: string;
  status: string;
  title: string;
  artist?: string | null;
  album?: string | null;
  manifestHash: string;
  swarmTopic: string;
  sizeBytes: number;
  downloadedBytes: number;
  peerCount: number;
  ticket: string;
  outputPath?: string | null;
  error?: string | null;
  createdAt: number;
  updatedAt: number;
  finishedAt?: number | null;
}

export interface P2pSettings {
  enabled: boolean;
  autoSeedDownloads: boolean;
  importDir?: string | null;
  uploadLimitKbps?: number | null;
  downloadLimitKbps?: number | null;
}

export interface P2pStatus extends P2pSettings {
  running: boolean;
  nodeId?: string | null;
  nodeAddr?: unknown;
  activeShares: number;
  activeDownloads: number;
  lastError?: string | null;
}

export interface PlaybackState {
  engine: string;
  status: string;
  trackId?: number | null;
  positionMs: number;
  durationMs?: number | null;
  volume: number;
  queue: number[];
  queueIndex?: number | null;
  error?: string | null;
}

export interface LibrarySnapshot {
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  playlists: Playlist[];
}
