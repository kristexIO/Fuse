import { CSSProperties, useEffect, useMemo, useRef } from "react";
import type { Album, Artist, Playlist, ScanSummary, Track } from "../types";

interface PanelProps {
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  playlists: Playlist[];
  scanSummary: ScanSummary | null;
}

interface CollectionPanelProps {
  tracks: Track[];
  activePlaylist: Playlist | null;
  activePlaylistTrackIds: Set<number>;
  currentTrackId: number | null;
  onPlayTrack: (track: Track) => void;
  onAddTrackToPlaylist: (trackId: number) => void;
}

interface NowPlayingPanelProps {
  artworkUrl: string | null;
  currentTrack: Track | null;
  isPlaying: boolean;
  currentTimeMs: number;
  showLyrics: boolean;
}

interface PlayerPanelProps {
  artworkUrl: string | null;
  currentTrack: Track | null;
  currentTimeMs: number;
  durationMs: number;
  isPlaying: boolean;
  playbackError: string | null;
  repeat: boolean;
  shuffle: boolean;
  volume: number;
  onNext: () => void;
  onPrevious: () => void;
  onSeek: (percent: number) => void;
  onTogglePlay: () => void;
  onToggleRepeat: () => void;
  onToggleShuffle: () => void;
  onVolumeChange: (volume: number) => void;
}

interface QueuePanelProps {
  artworkUrls: Record<number, string | null>;
  tracks: Track[];
  currentTrackId: number | null;
  onPlayTrack: (track: Track) => void;
}

interface PlaylistsPanelProps {
  artworkUrls: Record<number, string | null>;
  playlists: Playlist[];
  activePlaylistId: number | null;
  activePlaylistTracks: Track[];
  onSelectPlaylist: (playlistId: number) => void;
  onRemoveTrack: (trackId: number) => void;
  onDeletePlaylist: () => void;
  onPlayPlaylist: () => void;
}

export function LibraryPanel({ tracks, albums, artists }: Pick<PanelProps, "tracks" | "albums" | "artists">) {
  const missing = tracks.filter((track) => track.missingTags).length;

  return (
    <nav className="nav-list" aria-label="Разделы">
      <a className="nav-item" href="#collection">
        <strong>Все треки</strong>
        <span className="badge">{compactNumber(tracks.length)}</span>
      </a>
      <a className="nav-item" href="#albums">
        <strong>Альбомы</strong>
        <span className="badge">{compactNumber(albums.length)}</span>
      </a>
      <a className="nav-item" href="#artists">
        <strong>Исполнители</strong>
        <span className="badge">{compactNumber(artists.length)}</span>
      </a>
      <a className="nav-item" href="#missing-tags">
        <strong>Файлы без тегов</strong>
        <span className="badge">{compactNumber(missing)}</span>
      </a>
    </nav>
  );
}

interface LyricLine {
  timeMs: number;
  text: string;
}

function parseLrc(lrcText: string): LyricLine[] | null {
  const lines = lrcText.split("\n");
  const parsed: LyricLine[] = [];
  const lrcRegex = /^\[(\d+):(\d+)(?:[.:](\d+))?\](.*)/;

  let hasTimestamps = false;
  for (const line of lines) {
    const trimmed = line.trim();
    const match = lrcRegex.exec(trimmed);
    if (match) {
      hasTimestamps = true;
      const mins = parseInt(match[1], 10);
      const secs = parseInt(match[2], 10);
      const hundredths = match[3] ? parseInt(match[3], 10) : 0;
      const text = match[4].trim();

      const msFactor = match[3] && match[3].length === 3 ? 1 : 10;
      const timeMs = mins * 60 * 1000 + secs * 1000 + hundredths * msFactor;
      parsed.push({ timeMs, text });
    }
  }

  if (!hasTimestamps) {
    return null;
  }

  return parsed.sort((a, b) => a.timeMs - b.timeMs);
}

export function NowPlayingPanel({
  artworkUrl,
  currentTrack,
  isPlaying,
  currentTimeMs,
  showLyrics,
}: NowPlayingPanelProps) {
  const lyricsText = currentTrack?.lyrics;
  const parsedLyrics = useMemo(() => {
    if (!lyricsText) return null;
    return parseLrc(lyricsText);
  }, [lyricsText]);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const activeLineRef = useRef<HTMLDivElement | null>(null);

  // Find active line
  const activeIndex = useMemo(() => {
    if (!parsedLyrics) return -1;
    let index = -1;
    for (let i = 0; i < parsedLyrics.length; i++) {
      if (parsedLyrics[i].timeMs <= currentTimeMs) {
        index = i;
      } else {
        break;
      }
    }
    return index;
  }, [parsedLyrics, currentTimeMs]);

  // Scroll active line to center
  useEffect(() => {
    if (showLyrics && activeLineRef.current) {
      activeLineRef.current.scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
    }
  }, [activeIndex, showLyrics]);

  return (
    <div className={`cover-wrap ${isPlaying ? "is-playing" : ""}`}>
      {showLyrics ? (
        <div className="lyrics-view" ref={containerRef}>
          {parsedLyrics ? (
            <div className="lyrics-scroll">
              {parsedLyrics.map((line, index) => {
                const isActive = index === activeIndex;
                return (
                  <div
                    key={index}
                    ref={isActive ? activeLineRef : null}
                    className={`lyric-line ${isActive ? "is-active" : ""}`}
                  >
                    {line.text}
                  </div>
                );
              })}
            </div>
          ) : lyricsText ? (
            <div className="lyrics-plain">
              {lyricsText.split("\n").map((line, index) => (
                <div key={index} className="lyric-plain-line">
                  {line}
                </div>
              ))}
            </div>
          ) : (
            <div className="lyrics-empty">
              <span>Текст песни отсутствует</span>
              <p>Добавьте его в панели кастомизации справа</p>
            </div>
          )}
        </div>
      ) : (
        <div className={`cover ${artworkUrl ? "has-artwork" : ""}`} aria-label="Обложка трека" style={artworkStyle(artworkUrl)} />
      )}
      <div className="now-meta">
        <h2>{currentTrack?.title || "Signal Bloom"}</h2>
        <p>{currentTrack?.artist || "Northline Archive"}</p>
      </div>
      <div className="wave" aria-hidden="true">
        {[24, 48, 64, 30, 82, 58, 42, 72, 36, 68, 92, 44, 62, 76, 28, 70, 52, 84].map(
          (height, index) => (
            <i key={index} style={{ "--h": height } as CSSProperties} />
          ),
        )}
      </div>
    </div>
  );
}

export function CollectionPanel({
  tracks,
  activePlaylist,
  activePlaylistTrackIds,
  currentTrackId,
  onPlayTrack,
  onAddTrackToPlaylist,
}: CollectionPanelProps) {
  return (
    <>
      <div className="collection-toolbar">
        <div className="segmented" role="tablist" aria-label="Вид коллекции">
          <button className="is-active" type="button">Треки</button>
          <button type="button">Альбомы</button>
          <button type="button">Папки</button>
        </div>
        <span className="pill">
          {activePlaylist ? `В ${activePlaylist.name}: ${activePlaylist.trackCount}` : qualityLabel(tracks)}
        </span>
      </div>
      <div className="track-table" id="collection">
        {tracks.slice(0, 8).map((track) => {
          const alreadyAdded = activePlaylistTrackIds.has(track.id);

          return (
            <div className={`track-row ${track.id === currentTrackId ? "is-current" : ""} ${track.isMissing ? "is-missing" : ""}`} key={track.id}>
              <button className="track-action play" type="button" disabled={track.isMissing} title={track.isMissing ? "File is missing on disk" : "Играть сейчас"} onClick={() => onPlayTrack(track)}>
                ▶
              </button>
              <strong>{track.title}</strong>
              <span>{track.isMissing ? "Missing file" : track.artist || "Unknown Artist"}</span>
              <time>{formatDuration(track.durationMs)}</time>
              <button
                className="track-action"
                type="button"
                disabled={!activePlaylist || alreadyAdded || track.isMissing}
                title={
                  alreadyAdded
                    ? "Трек уже в активном плейлисте"
                    : activePlaylist
                      ? `Добавить в ${activePlaylist.name}`
                      : "Создай или выбери плейлист"
                }
                onClick={() => onAddTrackToPlaylist(track.id)}
              >
                {alreadyAdded ? "in" : "+"}
              </button>
            </div>
          );
        })}
        {tracks.length === 0 && <div className="empty-state">Импортируй папку, чтобы собрать медиатеку.</div>}
      </div>
    </>
  );
}

export function PlayerPanel({
  artworkUrl,
  currentTrack,
  currentTimeMs,
  durationMs,
  isPlaying,
  playbackError,
  repeat,
  shuffle,
  volume,
  onNext,
  onPrevious,
  onSeek,
  onTogglePlay,
  onToggleRepeat,
  onToggleShuffle,
  onVolumeChange,
}: PlayerPanelProps) {
  const progress = durationMs ? Math.min(100, (currentTimeMs / durationMs) * 100) : 0;

  return (
    <div className="player-layout">
      <div className="mini-track">
        <div className={`mini-cover ${artworkUrl ? "has-artwork" : ""}`} aria-hidden="true" style={artworkStyle(artworkUrl)} />
        <div>
          <strong>{currentTrack?.title || "Signal Bloom"}</strong>
          <span>{currentTrack?.artist || "Local FLAC"} - {currentTrack?.format || "offline library"}</span>
        </div>
      </div>
      <div className="transport">
        <div className="transport-buttons">
          <button className={`round-btn ${shuffle ? "is-active" : ""}`} type="button" aria-label="Shuffle" onClick={onToggleShuffle}>↟</button>
          <button className="round-btn" type="button" aria-label="Previous" onClick={onPrevious}>◀</button>
          <button className="round-btn primary" type="button" aria-label={isPlaying ? "Pause" : "Start"} onClick={onTogglePlay}>
            {isPlaying ? "Ⅱ" : "▶"}
          </button>
          <button className="round-btn" type="button" aria-label="Next" onClick={onNext}>▶</button>
          <button className={`round-btn ${repeat ? "is-active" : ""}`} type="button" aria-label="Repeat" onClick={onToggleRepeat}>↻</button>
        </div>
        <div className="progress-line">
          <span>{formatDuration(currentTimeMs)}</span>
          <input
            className="range-line"
            type="range"
            min="0"
            max="1000"
            value={Math.round(progress * 10)}
            onChange={(event) => onSeek(Number(event.currentTarget.value) / 1000)}
            aria-label="Seek"
            style={{ "--value": `${progress}%` } as CSSProperties}
          />
          <span>{formatDuration(durationMs || currentTrack?.durationMs)}</span>
        </div>
        {playbackError && <div className="player-error">{playbackError}</div>}
      </div>
      <div className="device-panel">
        <span>WebView audio output</span>
        <input
          className="range-line"
          type="range"
          min="0"
          max="100"
          value={Math.round(volume * 100)}
          onChange={(event) => onVolumeChange(Number(event.currentTarget.value) / 100)}
          aria-label="Volume"
          style={{ "--value": `${Math.round(volume * 100)}%` } as CSSProperties}
        />
      </div>
    </div>
  );
}

export function QueuePanel({ artworkUrls, tracks, currentTrackId, onPlayTrack }: QueuePanelProps) {
  const currentIndex = Math.max(
    0,
    tracks.findIndex((track) => track.id === currentTrackId),
  );
  const queue = tracks.slice(currentIndex, currentIndex + 5);

  return (
    <div className="queue">
      {(queue.length ? queue : tracks.slice(0, 5)).map((track) => (
        <button
          className={`queue-item ${track.id === currentTrackId ? "is-current" : ""} ${track.isMissing ? "is-missing" : ""}`}
          key={track.id}
          type="button"
          disabled={track.isMissing}
          onClick={() => onPlayTrack(track)}
        >
          <div className={`queue-art ${artworkUrls[track.id] ? "has-artwork" : ""}`} style={artworkStyle(artworkUrls[track.id])} />
          <div className="queue-text">
            <strong>{track.title}</strong>
            <span>{track.isMissing ? "Missing file" : track.artist || "Unknown Artist"}</span>
          </div>
          <time>{formatDuration(track.durationMs)}</time>
        </button>
      ))}
      {tracks.length === 0 && <div className="empty-state compact">Добавь треки, чтобы собрать очередь.</div>}
    </div>
  );
}

export function MixerPanel() {
  return (
    <div className="equalizer">
      {[46, 72, 58, 64, 82, 52, 69, 40].map((level, index) => (
        <div className="eq-band" key={index}>
          <div className="eq-slider"><i style={{ "--level": `${level}%` } as CSSProperties} /></div>
          <span>{["60", "125", "250", "500", "1k", "2k", "4k", "8k"][index]}</span>
        </div>
      ))}
    </div>
  );
}

export function PlaylistsPanel({
  artworkUrls,
  playlists,
  activePlaylistId,
  activePlaylistTracks,
  onSelectPlaylist,
  onRemoveTrack,
  onDeletePlaylist,
  onPlayPlaylist,
}: PlaylistsPanelProps) {
  const activePlaylist = playlists.find((playlist) => playlist.id === activePlaylistId) ?? null;

  return (
    <>
      <div className="playlist-grid">
        {playlists.slice(0, 6).map((playlist) => (
          <button
            className={`playlist-card ${playlist.id === activePlaylistId ? "is-active" : ""}`}
            key={playlist.id}
            type="button"
            onClick={() => onSelectPlaylist(playlist.id)}
          >
            <strong>{playlist.name}</strong>
            <span>{playlist.trackCount} tracks</span>
          </button>
        ))}
      </div>

      {playlists.length === 0 && (
        <div className="empty-state">Создай первый плейлист в правой панели, потом добавляй в него треки из коллекции.</div>
      )}

      {activePlaylist && (
        <div className="playlist-detail">
          <div className="playlist-detail-head">
            <div>
              <strong>{activePlaylist.name}</strong>
              <span>{activePlaylist.trackCount} tracks</span>
            </div>
            <div className="playlist-actions">
              <button className="track-action play" type="button" title="Играть плейлист" onClick={onPlayPlaylist}>
                ▶
              </button>
              <button className="track-action danger" type="button" title="Удалить плейлист" onClick={onDeletePlaylist}>
                ×
              </button>
            </div>
          </div>

          <div className="playlist-track-list">
            {activePlaylistTracks.slice(0, 5).map((track) => (
              <div className={`playlist-track ${track.isMissing ? "is-missing" : ""}`} key={track.id}>
                <div className={`playlist-track-art ${artworkUrls[track.id] ? "has-artwork" : ""}`} style={artworkStyle(artworkUrls[track.id])} />
                <div>
                  <strong>{track.title}</strong>
                  <span>{track.isMissing ? "Missing file" : track.artist || "Unknown Artist"}</span>
                </div>
                <button
                  className="track-action"
                  type="button"
                  title="Убрать из плейлиста"
                  onClick={() => onRemoveTrack(track.id)}
                >
                  −
                </button>
              </div>
            ))}
            {activePlaylistTracks.length === 0 && (
              <div className="empty-state compact">Добавь треки кнопкой + в коллекции.</div>
            )}
          </div>
        </div>
      )}
    </>
  );
}

export function StatsPanel({ tracks, scanSummary }: Pick<PanelProps, "tracks" | "scanSummary">) {
  const lossless = tracks.filter((track) => ["FLAC", "WAV", "AIFF"].includes(track.format)).length;
  const missing = tracks.filter((track) => track.missingTags).length;

  return (
    <div className="stat-grid">
      <div className="stat"><strong>{compactNumber(tracks.length)}</strong><span>локальных треков</span></div>
      <div className="stat"><strong>{compactNumber(lossless)}</strong><span>lossless файлов</span></div>
      <div className="stat"><strong>{scanSummary?.errors.length || missing}</strong><span>требуют внимания</span></div>
    </div>
  );
}

function qualityLabel(tracks: Track[]): string {
  if (tracks.length === 0) {
    return "FLAC 0% / MP3 0% / WAV 0%";
  }

  const count = (format: string) => tracks.filter((track) => track.format === format).length;
  const pct = (value: number) => Math.round((value / tracks.length) * 100);

  return `FLAC ${pct(count("FLAC"))}% / MP3 ${pct(count("MP3"))}% / WAV ${pct(count("WAV"))}%`;
}

function formatDuration(value?: number | null): string {
  if (!value) {
    return "0:00";
  }

  const totalSeconds = Math.round(value / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return `${minutes}:${seconds}`;
}

function compactNumber(value: number): string {
  if (value >= 1000) {
    return `${(value / 1000).toFixed(1)}k`;
  }

  return String(value);
}

function artworkStyle(artworkUrl?: string | null): CSSProperties | undefined {
  return artworkUrl ? { backgroundImage: `url("${artworkUrl}")` } : undefined;
}
