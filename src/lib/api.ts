import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  Album,
  Artwork,
  Artist,
  LayoutProfile,
  LibrarySnapshot,
  Playlist,
  ScanSummary,
  Track,
  TrackQuery,
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
    hasArtwork: true,
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
    hasArtwork: false,
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
    hasArtwork: false,
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
  { id: 1, name: "Late Focus", trackCount: 86, createdAt: 1780800000 },
  { id: 2, name: "Road Cache", trackCount: 142, createdAt: 1780800000 },
  { id: 3, name: "Lossless Picks", trackCount: 39, createdAt: 1780800000 },
];

const mockPlaylistTrackIds = new Map<number, number[]>([
  [1, [1, 3]],
  [2, [2]],
  [3, [1, 2, 3]],
]);

export function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export function audioSourceForTrack(track: Track): string | null {
  if (!isTauriRuntime()) {
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
  if (!isTauriRuntime()) {
    return {
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
    };
  }

  return invoke<ScanSummary>("scan_library", { paths });
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
    };
    mockPlaylists = [playlist, ...mockPlaylists];
    mockPlaylistTrackIds.set(playlist.id, []);
    return playlist;
  }

  return invoke<Playlist>("create_playlist", { name });
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
    return readMockPlaylists().find((playlist) => playlist.id === playlistId) ?? createPlaylist("Quick Mix");
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

function readMockPlaylists(): Playlist[] {
  return mockPlaylists.map((playlist) => ({
    ...playlist,
    trackCount: mockPlaylistTrackIds.get(playlist.id)?.length ?? playlist.trackCount,
  }));
}

function toAlbums(tracks: Track[]): Album[] {
  const byAlbum = new Map<string, Album>();

  tracks.forEach((track) => {
    const name = track.album || "Unknown Album";
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
    const name = track.artist || "Unknown Artist";
    const current = byArtist.get(name) || { name, trackCount: 0 };
    current.trackCount += 1;
    byArtist.set(name, current);
  });

  return [...byArtist.values()];
}
