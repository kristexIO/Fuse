import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  Album,
  AppDiagnostics,
  AppSettings,
  Artwork,
  BrokenTrackIssue,
  DuplicateTrackGroup,
  FuseShareTicket,
  Artist,
  LayoutProfile,
  LibraryFolder,
  LibrarySnapshot,
  LocalSearchResult,
  P2pSettings,
  P2pStatus,
  Playlist,
  PlaybackState,
  RecommendedTrack,
  ScanJob,
  ScanOptions,
  ScanSummary,
  SharedItem,
  SmartPlaylist,
  Track,
  TrackQuery,
  TransferTask,
  WorkspaceExport,
} from "../types";

let mockTracks: Track[] = [
  {
    id: 1,
    path: "C:/Music/Northline Archive/Signal Bloom.flac",
    title: "Signal Bloom",
    artist: "Northline Archive",
    album: "Late Focus",
    durationMs: 228000,
    format: "FLAC",
    sizeBytes: 44100000,
    modifiedAt: 1780830000,
    missingTags: false,
    artworkId: null,
    artworkUri: "mock://artwork/1",
    hasArtwork: true,
    dateAdded: 1780800000,
    playCount: 12,
    lastPlayedAt: null,
    isMissing: false,
    lyrics: `[00:00.00] (Инструментальное вступление)
[00:05.00] Запусти сигнал в эфир
[00:10.00] Дай ему раскрыться в белом шуме
[00:16.00] Потерянный во времени и пространстве
[00:22.00] Ищи свой путь во тьме
[00:28.00] (Электронное соло)
[00:45.00] Мы ловим отголоски прошлого
[00:52.00] На этой забытой частоте
[00:58.00] Держи волну крепче
[01:04.00] Не позволяй ей угаснуть
[01:10.00] (Ритмический переход)
[01:30.00] Космос говорит с нами
[01:36.00] На языке аналоговых сигналов
[01:42.00] Почувствуй это тепло
[01:48.00] На стыке цифровых миров
[01:55.00] (Кульминация)
[02:20.00] Держи сигнал близко
[02:26.00] Пусть он расцветет в статике
[02:32.00] В самом сердце шума
[02:40.00] Навсегда с тобой`,
  },
  {
    id: 2,
    path: "C:/Music/Lenora Map/Concrete Night Drive.mp3",
    title: "Concrete Night Drive",
    artist: "Lenora Map",
    album: "Road Cache",
    durationMs: 252000,
    format: "MP3",
    sizeBytes: 12300000,
    modifiedAt: 1780829000,
    missingTags: false,
    artworkId: null,
    artworkUri: null,
    hasArtwork: false,
    dateAdded: 1780800000,
    playCount: 4,
    lastPlayedAt: null,
    isMissing: false,
    lyrics: null,
  },
  {
    id: 3,
    path: "C:/Music/Imports/Glass Relay.wav",
    title: "Glass Relay",
    artist: "Paper Harbor",
    album: null,
    durationMs: 176000,
    format: "WAV",
    sizeBytes: 51200000,
    modifiedAt: 1780828000,
    missingTags: true,
    artworkId: null,
    artworkUri: null,
    hasArtwork: false,
    dateAdded: 1780800000,
    playCount: 0,
    lastPlayedAt: null,
    isMissing: false,
    lyrics: null,
  },
];

const mockArtworkDataUrls = new Map<number, string>([
  [
    1,
    "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 512 512'%3E%3Cdefs%3E%3ClinearGradient id='g' x1='0' y1='0' x2='1' y2='1'%3E%3Cstop stop-color='%23ff715b'/%3E%3Cstop offset='0.55' stop-color='%234fd8c6'/%3E%3Cstop offset='1' stop-color='%23201c2b'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='512' height='512' rx='36' fill='url(%23g)'/%3E%3Ccircle cx='256' cy='256' r='136' fill='none' stroke='rgba(255,255,255,.42)' stroke-width='18'/%3E%3Ccircle cx='256' cy='256' r='34' fill='rgba(0,0,0,.46)'/%3E%3C/svg%3E",
  ],
]);

let mockPlaylists: Playlist[] = [
  { id: 1, name: "Late Focus", trackCount: 86, createdAt: 1780800000, description: null, artworkUri: null, updatedAt: 1780800000, sortOrder: 1 },
  { id: 2, name: "Road Cache", trackCount: 142, createdAt: 1780800000, description: null, artworkUri: null, updatedAt: 1780800000, sortOrder: 2 },
  { id: 3, name: "Lossless Picks", trackCount: 39, createdAt: 1780800000, description: null, artworkUri: null, updatedAt: 1780800000, sortOrder: 3 },
];

let mockFolders: LibraryFolder[] = [];
let mockP2pRunning = false;
let mockP2pSettings: P2pSettings = {
  enabled: false,
  autoSeedDownloads: true,
  importDir: null,
  uploadLimitKbps: null,
  downloadLimitKbps: null,
};
let mockP2pShares: SharedItem[] = [];
let mockP2pTransfers: TransferTask[] = [];

const mockPlaylistTrackIds = new Map<number, number[]>([
  [1, [1, 3]],
  [2, [2]],
  [3, [1, 2, 3]],
]);

export function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export function audioSourceForTrack(track: Track): string | null {
  if (!isTauriRuntime() || track.isMissing) {
    return null;
  }

  return convertFileSrc(track.path);
}

export async function getLibrarySnapshot(query?: TrackQuery): Promise<LibrarySnapshot> {
  if (!isTauriRuntime()) {
    const search = query?.search?.trim().toLowerCase();
    const tracks = search
      ? mockTracks.filter((track) =>
          [track.title, track.artist, track.album, track.path]
            .filter(Boolean)
            .some((value) => value?.toLowerCase().includes(search)),
        )
      : mockTracks;

    return {
      tracks,
      albums: toAlbums(tracks),
      artists: toArtists(tracks),
      playlists: readMockPlaylists(),
    };
  }

  const [tracks, albums, artists, playlists] = await Promise.all([
    invoke<Track[]>("get_tracks", { query: query ?? null }),
    invoke<Album[]>("get_albums"),
    invoke<Artist[]>("get_artists"),
    invoke<Playlist[]>("get_playlists"),
  ]);

  return { tracks, albums, artists, playlists };
}

export async function scanLibrary(paths: string[]): Promise<ScanSummary> {
  const job = await startScan(paths);
  return {
    scannedFiles: job.scannedFiles,
    added: job.added,
    updated: job.updated,
    skipped: job.skipped,
    errors: job.errors,
  };
}

export async function startScan(paths: string[], options?: ScanOptions): Promise<ScanJob> {
  if (!isTauriRuntime()) {
    return {
      id: Date.now(),
      state: "completed_with_errors",
      totalFiles: null,
      scannedFiles: 0,
      added: 0,
      updated: 0,
      skipped: 0,
      errors: [
        {
          path: paths.join("; "),
          message: "Folder scanning is available in the Tauri desktop window.",
        },
      ],
      startedAt: Math.floor(Date.now() / 1000),
      finishedAt: Math.floor(Date.now() / 1000),
    };
  }

  return invoke<ScanJob>("start_scan", { paths, options: options ?? null });
}

export async function cancelScan(jobId: number): Promise<boolean> {
  if (!isTauriRuntime()) {
    return false;
  }

  return invoke<boolean>("cancel_scan", { jobId });
}

export async function getLibraryFolders(): Promise<LibraryFolder[]> {
  if (!isTauriRuntime()) {
    return mockFolders;
  }

  return invoke<LibraryFolder[]>("get_library_folders");
}

export async function addLibraryFolder(path: string): Promise<LibraryFolder> {
  if (!isTauriRuntime()) {
    const existing = mockFolders.find((folder) => folder.path === path);
    if (existing) {
      return existing;
    }

    const folder: LibraryFolder = {
      id: Math.max(0, ...mockFolders.map((item) => item.id)) + 1,
      path,
      addedAt: Math.floor(Date.now() / 1000),
      lastScannedAt: null,
      ignoredPatterns: null,
    };
    mockFolders = [...mockFolders, folder];
    return folder;
  }

  return invoke<LibraryFolder>("add_library_folder", { path });
}

export async function removeLibraryFolder(folderId: number): Promise<void> {
  if (!isTauriRuntime()) {
    mockFolders = mockFolders.filter((folder) => folder.id !== folderId);
    return;
  }

  await invoke("remove_library_folder", { folderId });
}

export async function getDiagnostics(): Promise<AppDiagnostics> {
  if (!isTauriRuntime()) {
    return { appDataDir: null, logPath: null, recentEvents: [] };
  }

  return invoke<AppDiagnostics>("get_diagnostics");
}

export async function recordClientError(message: string, source?: string): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("record_client_error", { message, source: source ?? null });
}

export async function getSettings(): Promise<AppSettings> {
  if (!isTauriRuntime()) {
    return { firstRunComplete: false, reducedMotion: false, activeLayout: null };
  }

  return invoke<AppSettings>("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("save_settings", { settings });
}

export async function getP2pStatus(): Promise<P2pStatus> {
  if (!isTauriRuntime()) {
    return {
      ...mockP2pSettings,
      running: mockP2pRunning,
      nodeId: mockP2pRunning ? "preview-node" : null,
      nodeAddr: null,
      activeShares: mockP2pShares.filter((share) => share.state === "active").length,
      activeDownloads: mockP2pTransfers.filter((transfer) => ["pending", "downloading"].includes(transfer.status)).length,
      lastError: null,
    };
  }

  return invoke<P2pStatus>("get_p2p_status");
}

export async function getP2pSettings(): Promise<P2pSettings> {
  if (!isTauriRuntime()) {
    return mockP2pSettings;
  }

  return invoke<P2pSettings>("get_p2p_settings");
}

export async function saveP2pSettings(settings: P2pSettings): Promise<P2pSettings> {
  if (!isTauriRuntime()) {
    mockP2pSettings = { ...settings };
    mockP2pRunning = settings.enabled && mockP2pRunning;
    return mockP2pSettings;
  }

  return invoke<P2pSettings>("save_p2p_settings", { settings });
}

export async function startP2p(): Promise<P2pStatus> {
  if (!isTauriRuntime()) {
    mockP2pRunning = true;
    mockP2pSettings = { ...mockP2pSettings, enabled: true };
    return getP2pStatus();
  }

  return invoke<P2pStatus>("start_p2p");
}

export async function stopP2p(): Promise<P2pStatus> {
  if (!isTauriRuntime()) {
    mockP2pRunning = false;
    mockP2pSettings = { ...mockP2pSettings, enabled: false };
    return getP2pStatus();
  }

  return invoke<P2pStatus>("stop_p2p");
}

export async function createTrackShareTicket(trackId: number): Promise<SharedItem> {
  if (!isTauriRuntime()) {
    const track = mockTracks.find((item) => item.id === trackId);
    if (!track) {
      throw new Error("Track not found");
    }
    const share = mockShareFromTrack(track);
    mockP2pShares = [share, ...mockP2pShares];
    mockP2pRunning = true;
    mockP2pSettings = { ...mockP2pSettings, enabled: true };
    return share;
  }

  return invoke<SharedItem>("create_track_share_ticket", { trackId });
}

export async function createPlaylistShareTicket(playlistId: number): Promise<SharedItem> {
  if (!isTauriRuntime()) {
    const playlist = readMockPlaylists().find((item) => item.id === playlistId);
    if (!playlist) {
      throw new Error("Playlist not found");
    }
    const share = mockShareFromPlaylist(playlist);
    mockP2pShares = [share, ...mockP2pShares];
    mockP2pRunning = true;
    mockP2pSettings = { ...mockP2pSettings, enabled: true };
    return share;
  }

  return invoke<SharedItem>("create_playlist_share_ticket", { playlistId });
}

export async function listP2pShares(): Promise<SharedItem[]> {
  if (!isTauriRuntime()) {
    return mockP2pShares;
  }

  return invoke<SharedItem[]>("list_p2p_shares");
}

export async function pauseP2pShare(shareId: number): Promise<SharedItem> {
  if (!isTauriRuntime()) {
    return updateMockShareState(shareId, "paused");
  }

  return invoke<SharedItem>("pause_p2p_share", { shareId });
}

export async function resumeP2pShare(shareId: number): Promise<SharedItem> {
  if (!isTauriRuntime()) {
    return updateMockShareState(shareId, "active");
  }

  return invoke<SharedItem>("resume_p2p_share", { shareId });
}

export async function revokeP2pShare(shareId: number): Promise<SharedItem> {
  if (!isTauriRuntime()) {
    return updateMockShareState(shareId, "revoked", Math.floor(Date.now() / 1000));
  }

  return invoke<SharedItem>("revoke_p2p_share", { shareId });
}

export async function previewShareTicket(ticket: string): Promise<FuseShareTicket> {
  if (!isTauriRuntime()) {
    if (!ticket.trim().startsWith("fuse-share:v1:")) {
      throw new Error("Share ticket must start with fuse-share:v1:");
    }
    return {
      version: 1,
      scope: "track",
      manifestHash: "preview",
      swarmTopic: "preview",
      providers: [{ nodeId: "preview-node", addr: null }],
      display: { title: "Preview ticket", itemCount: 1 },
      items: [{ title: "Preview ticket", format: "MP3", fileHash: "preview", sizeBytes: 0 }],
      sizeBytes: 0,
      createdAt: Math.floor(Date.now() / 1000),
    };
  }

  return invoke<FuseShareTicket>("preview_share_ticket", { ticket });
}

export async function startDownloadFromTicket(ticket: string): Promise<TransferTask> {
  if (!isTauriRuntime()) {
    const task = mockTransferFromTicket(ticket, "completed");
    mockP2pTransfers = [task, ...mockP2pTransfers];
    return task;
  }

  return invoke<TransferTask>("start_download_from_ticket", { ticket });
}

export async function listP2pTransfers(): Promise<TransferTask[]> {
  if (!isTauriRuntime()) {
    return mockP2pTransfers;
  }

  return invoke<TransferTask[]>("list_p2p_transfers");
}

export async function cancelP2pTransfer(transferId: number): Promise<TransferTask> {
  if (!isTauriRuntime()) {
    return updateMockTransferStatus(transferId, "cancelled");
  }

  return invoke<TransferTask>("cancel_p2p_transfer", { transferId });
}

export async function pauseP2pTransfer(transferId: number): Promise<TransferTask> {
  if (!isTauriRuntime()) {
    return updateMockTransferStatus(transferId, "paused");
  }

  return invoke<TransferTask>("pause_p2p_transfer", { transferId });
}

export async function resumeP2pTransfer(transferId: number): Promise<TransferTask> {
  if (!isTauriRuntime()) {
    return updateMockTransferStatus(transferId, "completed");
  }

  return invoke<TransferTask>("resume_p2p_transfer", { transferId });
}

export async function retryP2pTransfer(transferId: number): Promise<TransferTask> {
  if (!isTauriRuntime()) {
    return updateMockTransferStatus(transferId, "completed");
  }

  return invoke<TransferTask>("retry_p2p_transfer", { transferId });
}

export async function pickMusicFolders(): Promise<string[]> {
  if (!isTauriRuntime()) {
    return [];
  }

  const result = await open({
    directory: true,
    multiple: true,
    title: "Choose music folders",
  });

  if (!result) {
    return [];
  }

  return Array.isArray(result) ? result : [result];
}

export async function pickMusicFiles(): Promise<string[]> {
  if (!isTauriRuntime()) {
    return [];
  }

  const result = await open({
    multiple: true,
    title: "Choose music files",
    filters: [
      {
        name: "Audio",
        extensions: ["aac", "aif", "aiff", "ape", "flac", "m4a", "mp3", "mp4", "mpc", "ogg", "opus", "speex", "wav", "wv"],
      },
    ],
  });

  if (!result) {
    return [];
  }

  return Array.isArray(result) ? result : [result];
}

export async function pickArtworkFile(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const result = await open({
    multiple: false,
    title: "Choose cover art",
    filters: [
      {
        name: "Images",
        extensions: ["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff"],
      },
    ],
  });

  return typeof result === "string" ? result : null;
}

export async function createPlaylist(name: string): Promise<Playlist> {
  if (!isTauriRuntime()) {
    const cleanName = name.trim();
    if (!cleanName) {
      throw new Error("Playlist name is empty");
    }

    const existing = readMockPlaylists().find((playlist) => playlist.name === cleanName);
    if (existing) {
      return existing;
    }

    const playlist: Playlist = {
      id: Math.max(0, ...mockPlaylists.map((item) => item.id)) + 1,
      name: cleanName,
      trackCount: 0,
      createdAt: Math.floor(Date.now() / 1000),
      description: null,
      artworkUri: null,
      updatedAt: Math.floor(Date.now() / 1000),
      sortOrder: Math.max(0, ...mockPlaylists.map((item) => item.sortOrder)) + 1,
    };
    mockPlaylists = [playlist, ...mockPlaylists];
    mockPlaylistTrackIds.set(playlist.id, []);
    return playlist;
  }

  return invoke<Playlist>("create_playlist", { name });
}

export async function updatePlaylist(
  playlistId: number,
  details: { name?: string | null; description?: string | null },
): Promise<Playlist> {
  if (!isTauriRuntime()) {
    const playlist = mockPlaylists.find((item) => item.id === playlistId);
    if (!playlist) {
      throw new Error("Playlist not found");
    }

    if (details.name?.trim()) {
      playlist.name = details.name.trim();
    }
    playlist.description = details.description?.trim() || null;
    playlist.updatedAt = Math.floor(Date.now() / 1000);
    return { ...playlist };
  }

  return invoke<Playlist>("update_playlist", {
    playlistId,
    name: details.name ?? null,
    description: details.description ?? null,
  });
}

export async function deletePlaylist(playlistId: number): Promise<void> {
  if (!isTauriRuntime()) {
    mockPlaylists = mockPlaylists.filter((playlist) => playlist.id !== playlistId);
    mockPlaylistTrackIds.delete(playlistId);
    return;
  }

  await invoke("delete_playlist", { playlistId });
}

export async function addTracksToPlaylist(playlistId: number, trackIds: number[]): Promise<Playlist> {
  if (!isTauriRuntime()) {
    const current = mockPlaylistTrackIds.get(playlistId) ?? [];
    const next = [...current];

    trackIds.forEach((trackId) => {
      if (!next.includes(trackId)) {
        next.push(trackId);
      }
    });

    mockPlaylistTrackIds.set(playlistId, next);
    return readMockPlaylists().find((playlist) => playlist.id === playlistId) ?? createPlaylist("Быстрый микс");
  }

  return invoke<Playlist>("add_tracks_to_playlist", { playlistId, trackIds });
}

export async function removeTrackFromPlaylist(playlistId: number, trackId: number): Promise<void> {
  if (!isTauriRuntime()) {
    const current = mockPlaylistTrackIds.get(playlistId) ?? [];
    mockPlaylistTrackIds.set(
      playlistId,
      current.filter((id) => id !== trackId),
    );
    return;
  }

  await invoke("remove_track_from_playlist", { playlistId, trackId });
}

export async function getPlaylistTracks(playlistId: number): Promise<Track[]> {
  if (!isTauriRuntime()) {
    const ids = mockPlaylistTrackIds.get(playlistId) ?? [];
    return ids
      .map((id) => mockTracks.find((track) => track.id === id))
      .filter((track): track is Track => Boolean(track));
  }

  return invoke<Track[]>("get_playlist_tracks", { playlistId });
}

export async function reorderPlaylistTracks(playlistId: number, trackIds: number[]): Promise<Playlist> {
  if (!isTauriRuntime()) {
    mockPlaylistTrackIds.set(playlistId, trackIds);
    const playlist = readMockPlaylists().find((item) => item.id === playlistId);
    if (!playlist) {
      throw new Error("Playlist not found");
    }
    return playlist;
  }

  return invoke<Playlist>("reorder_playlist_tracks", { playlistId, trackIds });
}

export async function getTrackArtwork(trackId: number): Promise<Artwork | null> {
  if (!isTauriRuntime()) {
    const dataUrl = mockArtworkDataUrls.get(trackId);
    return dataUrl ? { trackId, mime: "image/svg+xml", dataUrl } : null;
  }

  return invoke<Artwork | null>("get_track_artwork", { trackId });
}

export async function setTrackArtwork(trackId: number, imagePath: string): Promise<Track> {
  if (!isTauriRuntime()) {
    const track = mockTracks.find((item) => item.id === trackId);
    if (!track) {
      throw new Error("Track not found");
    }

    track.hasArtwork = true;
    track.artworkId = `mock:${Date.now()}`;
    mockArtworkDataUrls.set(trackId, mockArtworkDataUrls.get(1) || "");
    return track;
  }

  return invoke<Track>("set_track_artwork", { trackId, imagePath });
}

export async function updateTrackDetails(
  trackId: number,
  details: Pick<Track, "title" | "artist" | "album" | "lyrics">,
): Promise<Track> {
  if (!isTauriRuntime()) {
    const track = mockTracks.find((item) => item.id === trackId);
    if (!track) {
      throw new Error("Track not found");
    }

    const title = details.title.trim();
    if (!title) {
      throw new Error("Track title is empty");
    }

    track.title = title;
    track.artist = details.artist?.trim() || null;
    track.album = details.album?.trim() || null;
    track.lyrics = details.lyrics?.trim() || null;
    track.missingTags = !track.artist || !track.album;
    mockTracks = [...mockTracks];
    return track;
  }

  return invoke<Track>("update_track_details", {
    trackId,
    title: details.title,
    artist: details.artist ?? null,
    album: details.album ?? null,
    lyrics: details.lyrics ?? null,
  });
}

export async function markTrackPlayed(trackId: number): Promise<Track | null> {
  if (!isTauriRuntime()) {
    const track = mockTracks.find((item) => item.id === trackId);
    if (!track) {
      return null;
    }

    track.playCount += 1;
    track.lastPlayedAt = Math.floor(Date.now() / 1000);
    return { ...track };
  }

  return invoke<Track>("mark_track_played", { trackId });
}

export async function setRustPlaybackQueue(trackIds: number[], startIndex?: number): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("set_queue", { trackIds, startIndex: startIndex ?? null });
}

export async function playRustQueueIndex(index: number): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("play_queue_index", { index });
}

export async function playRustTrack(trackId: number): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("play_track", { trackId });
}

export async function pauseRustPlayback(): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("pause_playback");
}

export async function stopRustPlayback(): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("stop_playback");
}

export async function resumeRustPlayback(): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("resume_playback");
}

export async function seekRustPlayback(positionMs: number): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("seek_playback", { positionMs: Math.round(positionMs) });
}

export async function setRustPlaybackVolume(volume: number): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("set_volume", { volume });
}

export async function getRustPlaybackState(): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("get_playback_state");
}

export async function loadLayoutProfile(name: string): Promise<LayoutProfile | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<LayoutProfile | null>("load_layout", { name });
}

export async function saveLayoutProfile(profile: LayoutProfile): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("save_layout", { profile });
}

export async function listLayoutProfiles(): Promise<LayoutProfile[]> {
  if (!isTauriRuntime()) {
    return [];
  }

  return invoke<LayoutProfile[]>("list_layout_profiles");
}

export async function exportWorkspace(): Promise<WorkspaceExport> {
  if (!isTauriRuntime()) {
    return {
      version: 1,
      settings: { firstRunComplete: true, reducedMotion: false, activeLayout: "Studio" },
      p2pSettings: mockP2pSettings,
      layouts: [],
      exportedAt: Math.floor(Date.now() / 1000),
    };
  }

  return invoke<WorkspaceExport>("export_workspace");
}

export async function importWorkspace(bundle: WorkspaceExport): Promise<WorkspaceExport> {
  if (!isTauriRuntime()) {
    mockP2pSettings = { ...bundle.p2pSettings };
    return { ...bundle, exportedAt: Math.floor(Date.now() / 1000) };
  }

  return invoke<WorkspaceExport>("import_workspace", { bundle });
}

export async function listSmartPlaylists(): Promise<SmartPlaylist[]> {
  if (!isTauriRuntime()) {
    return mockSmartPlaylists();
  }

  return invoke<SmartPlaylist[]>("list_smart_playlists");
}

export async function getSmartPlaylistTracks(smartId: string, limit = 25): Promise<Track[]> {
  if (!isTauriRuntime()) {
    return mockSmartPlaylistTracks(smartId).slice(0, limit);
  }

  return invoke<Track[]>("get_smart_playlist_tracks", { smartId, limit });
}

export async function localSearch(query: string, limit = 12): Promise<LocalSearchResult> {
  if (!isTauriRuntime()) {
    return mockLocalSearch(query, limit);
  }

  return invoke<LocalSearchResult>("local_search", { query, limit });
}

export async function recommendTracks(seedTrackId: number, limit = 12): Promise<RecommendedTrack[]> {
  if (!isTauriRuntime()) {
    return mockRecommendations(seedTrackId).slice(0, limit);
  }

  return invoke<RecommendedTrack[]>("recommend_tracks", { seedTrackId, limit });
}

export async function createRadioQueue(seedTrackId: number, limit = 25): Promise<PlaybackState | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<PlaybackState>("create_radio_queue", { seedTrackId, limit });
}

export async function findDuplicateTracks(): Promise<DuplicateTrackGroup[]> {
  if (!isTauriRuntime()) {
    return mockDuplicateGroups();
  }

  return invoke<DuplicateTrackGroup[]>("find_duplicate_tracks");
}

export async function findBrokenTracks(): Promise<BrokenTrackIssue[]> {
  if (!isTauriRuntime()) {
    return mockTracks
      .filter((track) => track.isMissing)
      .map((track) => ({ track, reason: "File is missing from disk" }));
  }

  return invoke<BrokenTrackIssue[]>("find_broken_tracks");
}

export async function repairTrackPath(trackId: number, replacementPath: string): Promise<Track> {
  if (!isTauriRuntime()) {
    const track = mockTracks.find((item) => item.id === trackId);
    if (!track) {
      throw new Error("Track not found");
    }
    track.path = replacementPath;
    track.isMissing = false;
    return { ...track };
  }

  return invoke<Track>("repair_track_path", { trackId, replacementPath });
}

function mockSmartPlaylists(): SmartPlaylist[] {
  const definitions = [
    { id: "lossless", name: "Lossless", description: "FLAC and WAV tracks" },
    { id: "recent", name: "Recently added", description: "Newest local imports first" },
    { id: "missing-tags", name: "Needs tags", description: "Tracks with missing metadata" },
    { id: "favorites", name: "Local favorites", description: "Most played on this device" },
    { id: "focus", name: "Focus radio", description: "Longer complete tracks" },
    { id: "quick", name: "Quick plays", description: "Short local tracks" },
  ];
  return definitions.map((definition) => ({
    ...definition,
    trackCount: mockSmartPlaylistTracks(definition.id).length,
  }));
}

function mockSmartPlaylistTracks(smartId: string): Track[] {
  const tracks = [...mockTracks];
  switch (smartId) {
    case "lossless":
      return tracks.filter((track) => ["FLAC", "WAV", "AIFF", "ALAC"].includes(track.format.toUpperCase()));
    case "recent":
      return tracks.filter((track) => !track.isMissing).sort((a, b) => b.dateAdded - a.dateAdded);
    case "missing-tags":
      return tracks.filter((track) => track.missingTags || !track.artist || !track.album);
    case "favorites":
      return tracks.filter((track) => track.playCount > 0).sort((a, b) => b.playCount - a.playCount);
    case "focus":
      return tracks.filter((track) => (track.durationMs ?? 0) >= 180000 && !track.missingTags && !track.isMissing);
    case "quick":
      return tracks.filter((track) => (track.durationMs ?? Number.MAX_SAFE_INTEGER) <= 180000 && !track.isMissing);
    default:
      return [];
  }
}

function mockLocalSearch(query: string, limit: number): LocalSearchResult {
  const normalized = query.trim().toLowerCase();
  const matches = (value?: string | null) => Boolean(value?.toLowerCase().includes(normalized));
  if (!normalized) {
    return { query, tracks: [], albums: [], artists: [], playlists: [] };
  }
  const tracks = mockTracks
    .filter((track) => [track.title, track.artist, track.album, track.lyrics].some(matches))
    .slice(0, limit);
  return {
    query,
    tracks,
    albums: toAlbums(mockTracks).filter((album) => matches(album.name) || matches(album.artist)).slice(0, limit),
    artists: toArtists(mockTracks).filter((artist) => matches(artist.name)).slice(0, limit),
    playlists: readMockPlaylists().filter((playlist) => matches(playlist.name) || matches(playlist.description)).slice(0, limit),
  };
}

function mockRecommendations(seedTrackId: number): RecommendedTrack[] {
  const seed = mockTracks.find((track) => track.id === seedTrackId) ?? mockTracks[0];
  return mockTracks
    .filter((track) => track.id !== seed.id && !track.isMissing)
    .map((track) => {
      let score = 0;
      const reasons: string[] = [];
      if (track.artist && track.artist === seed.artist) {
        score += 45;
        reasons.push("same artist");
      }
      if (track.album && track.album === seed.album) {
        score += 25;
        reasons.push("same album");
      }
      if (track.format === seed.format) {
        score += 10;
        reasons.push("same format");
      }
      score += Math.max(1, track.playCount);
      return { track, score, reason: reasons.join(", ") || "similar local metadata" };
    })
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score);
}

function mockDuplicateGroups(): DuplicateTrackGroup[] {
  const byKey = new Map<string, Track[]>();
  mockTracks.forEach((track) => {
    const key = `${track.title.toLowerCase()}|${track.artist ?? ""}|${track.album ?? ""}|${track.durationMs ?? 0}|${track.sizeBytes}`;
    byKey.set(key, [...(byKey.get(key) ?? []), track]);
  });
  return [...byKey.entries()]
    .filter(([, tracks]) => tracks.length > 1)
    .map(([signature, tracks]) => ({
      signature,
      tracks,
      sizeBytes: tracks.reduce((sum, track) => sum + track.sizeBytes, 0),
    }));
}

function readMockPlaylists(): Playlist[] {
  return mockPlaylists
    .map((playlist) => ({
      ...playlist,
      trackCount: mockPlaylistTrackIds.get(playlist.id)?.length ?? playlist.trackCount,
    }))
    .sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name));
}

function mockShareFromTrack(track: Track): SharedItem {
  const now = Math.floor(Date.now() / 1000);
  const manifestHash = `mock-${track.id}-${now}`;

  return {
    id: Math.max(0, ...mockP2pShares.map((share) => share.id)) + 1,
    scope: "track",
    trackId: track.id,
    playlistId: null,
    title: track.title,
    artist: track.artist,
    album: track.album,
    manifestHash,
    swarmTopic: `mock-topic-${manifestHash}`,
    sizeBytes: track.sizeBytes,
    itemCount: 1,
    ticket: `fuse-share:v1:preview-${manifestHash}`,
    state: "active",
    createdAt: now,
    updatedAt: now,
    revokedAt: null,
  };
}

function mockShareFromPlaylist(playlist: Playlist): SharedItem {
  const now = Math.floor(Date.now() / 1000);
  const manifestHash = `mock-playlist-${playlist.id}-${now}`;
  const tracks = mockPlaylistTrackIds.get(playlist.id) ?? [];

  return {
    id: Math.max(0, ...mockP2pShares.map((share) => share.id)) + 1,
    scope: "playlist",
    trackId: null,
    playlistId: playlist.id,
    title: playlist.name,
    artist: null,
    album: null,
    manifestHash,
    swarmTopic: `mock-topic-${manifestHash}`,
    sizeBytes: tracks
      .map((trackId) => mockTracks.find((track) => track.id === trackId)?.sizeBytes ?? 0)
      .reduce((total, size) => total + size, 0),
    itemCount: tracks.length,
    ticket: `fuse-share:v1:preview-${manifestHash}`,
    state: "active",
    createdAt: now,
    updatedAt: now,
    revokedAt: null,
  };
}

function updateMockShareState(shareId: number, state: string, revokedAt: number | null = null): SharedItem {
  const share = mockP2pShares.find((item) => item.id === shareId);
  if (!share) {
    throw new Error("Share not found");
  }

  share.state = state;
  share.updatedAt = Math.floor(Date.now() / 1000);
  share.revokedAt = revokedAt;
  mockP2pShares = [...mockP2pShares];
  return { ...share };
}

function mockTransferFromTicket(ticket: string, status: string): TransferTask {
  const now = Math.floor(Date.now() / 1000);
  return {
    id: Math.max(0, ...mockP2pTransfers.map((transfer) => transfer.id)) + 1,
    direction: "download",
    status,
    title: "Preview download",
    artist: null,
    album: null,
    manifestHash: `mock-transfer-${now}`,
    swarmTopic: `mock-topic-${now}`,
    sizeBytes: 0,
    downloadedBytes: 0,
    peerCount: 1,
    ticket,
    outputPath: null,
    error: null,
    createdAt: now,
    updatedAt: now,
    finishedAt: status === "completed" ? now : null,
  };
}

function updateMockTransferStatus(transferId: number, status: string): TransferTask {
  const transfer = mockP2pTransfers.find((item) => item.id === transferId);
  if (!transfer) {
    throw new Error("Transfer not found");
  }

  transfer.status = status;
  transfer.updatedAt = Math.floor(Date.now() / 1000);
  transfer.finishedAt = ["completed", "cancelled", "failed"].includes(status) ? transfer.updatedAt : null;
  mockP2pTransfers = [...mockP2pTransfers];
  return { ...transfer };
}

function toAlbums(tracks: Track[]): Album[] {
  const byAlbum = new Map<string, Album>();

  tracks.forEach((track) => {
    const name = track.album || "Без альбома";
    const key = `${name}:${track.artist || ""}`;
    const current = byAlbum.get(key) || {
      name,
      artist: track.artist,
      trackCount: 0,
    };
    current.trackCount += 1;
    byAlbum.set(key, current);
  });

  return [...byAlbum.values()];
}

function toArtists(tracks: Track[]): Artist[] {
  const byArtist = new Map<string, Artist>();

  tracks.forEach((track) => {
    const name = track.artist || "Неизвестный исполнитель";
    const current = byArtist.get(name) || { name, trackCount: 0 };
    current.trackCount += 1;
    byArtist.set(name, current);
  });

  return [...byArtist.values()];
}
