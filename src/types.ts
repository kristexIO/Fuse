export type ThemeName =
  | "obsidian"
  | "porcelain"
  | "oled"
  | "boreal"
  | "ember"
  | "violet"
  | "rose";

export type Density = "compact" | "comfortable" | "spacious";

export type ModuleId =
  | "library"
  | "now"
  | "collection"
  | "player"
  | "queue"
  | "mixer"
  | "playlists"
  | "stats";

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
  hasArtwork: boolean;
  lyrics?: string | null;
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

export interface LibrarySnapshot {
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  playlists: Playlist[];
}
