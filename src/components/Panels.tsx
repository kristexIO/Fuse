import { CSSProperties, useEffect, useMemo, useRef } from "react";
import {
  ArrowDown,
  ArrowUp,
  Copy,
  Disc3,
  Download,
  Folder,
  ListMusic,
  Minus,
  Pause,
  Play,
  Plus,
  RadioTower,
  Repeat,
  RotateCcw,
  Share2,
  Shuffle,
  SkipBack,
  SkipForward,
  Square,
  Ticket,
  Trash2,
  Volume2,
  Wifi,
  WifiOff,
  XCircle,
} from "lucide-react";
import type {
  Album,
  Artist,
  CollectionView,
  LibraryFolder,
  P2pStatus,
  Playlist,
  ScanSummary,
  SharedItem,
  Track,
  TransferTask,
} from "../types";

interface PanelProps {
  tracks: Track[];
  albums: Album[];
  artists: Artist[];
  playlists: Playlist[];
  scanSummary: ScanSummary | null;
}

interface CollectionPanelProps {
  tracks: Track[];
  albums: Album[];
  libraryFolders: LibraryFolder[];
  view: CollectionView;
  activePlaylist: Playlist | null;
  activePlaylistTrackIds: Set<number>;
  currentTrackId: number | null;
  onViewChange: (view: CollectionView) => void;
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
  playbackBackend: string;
  repeat: boolean;
  shuffle: boolean;
  volume: number;
  onNext: () => void;
  onPrevious: () => void;
  onSeek: (percent: number) => void;
  onStop: () => void;
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
  onMoveTrack: (trackId: number, direction: -1 | 1) => void;
  onPlayPlaylist: () => void;
  onRenamePlaylist: (name: string, description?: string | null) => void;
}

interface SwarmPanelProps {
  p2pStatus: P2pStatus | null;
  shares: SharedItem[];
  transfers: TransferTask[];
  ticketDraft: string;
  previewMode: boolean;
  currentTrack: Track | null;
  activePlaylist: Playlist | null;
  onTicketDraftChange: (ticket: string) => void;
  onStartP2p: () => void;
  onStopP2p: () => void;
  onShareTrack: () => void;
  onSharePlaylist: () => void;
  onCopyTicket: (ticket: string) => void;
  onDownloadTicket: () => void;
  onPauseShare: (shareId: number) => void;
  onResumeShare: (shareId: number) => void;
  onRevokeShare: (shareId: number) => void;
  onCancelTransfer: (transferId: number) => void;
  onRetryTransfer: (transferId: number) => void;
}

export function LibraryPanel({ tracks, albums, artists }: Pick<PanelProps, "tracks" | "albums" | "artists">) {
  const missing = tracks.filter((track) => track.missingTags || track.isMissing).length;

  return (
    <nav className="nav-list" aria-label="Разделы">
      <a className="nav-item" href="#collection">
        <strong>Все треки</strong>
        <span className="badge">{compactNumber(tracks.length)}</span>
      </a>
      <a className="nav-item" href="#collection">
        <strong>Альбомы</strong>
        <span className="badge">{compactNumber(albums.length)}</span>
      </a>
      <a className="nav-item" href="#collection">
        <strong>Исполнители</strong>
        <span className="badge">{compactNumber(artists.length)}</span>
      </a>
      <a className="nav-item" href="#collection">
        <strong>Требуют внимания</strong>
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
      const fraction = match[3] ? parseInt(match[3], 10) : 0;
      const msFactor = match[3] && match[3].length === 3 ? 1 : 10;
      parsed.push({
        timeMs: mins * 60 * 1000 + secs * 1000 + fraction * msFactor,
        text: match[4].trim(),
      });
    }
  }

  return hasTimestamps ? parsed.sort((a, b) => a.timeMs - b.timeMs) : null;
}

export function NowPlayingPanel({
  artworkUrl,
  currentTrack,
  isPlaying,
  currentTimeMs,
  showLyrics,
}: NowPlayingPanelProps) {
  const lyricsText = currentTrack?.lyrics;
  const parsedLyrics = useMemo(() => (lyricsText ? parseLrc(lyricsText) : null), [lyricsText]);
  const activeLineRef = useRef<HTMLDivElement | null>(null);
  const activeIndex = useMemo(() => {
    if (!parsedLyrics) {
      return -1;
    }

    let index = -1;
    for (let i = 0; i < parsedLyrics.length; i += 1) {
      if (parsedLyrics[i].timeMs <= currentTimeMs) {
        index = i;
      } else {
        break;
      }
    }

    return index;
  }, [parsedLyrics, currentTimeMs]);

  useEffect(() => {
    if (showLyrics && activeLineRef.current) {
      activeLineRef.current.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [activeIndex, showLyrics]);

  return (
    <div className={`cover-wrap ${isPlaying ? "is-playing" : ""}`}>
      {showLyrics ? (
        <div className="lyrics-view">
          {parsedLyrics ? (
            <div className="lyrics-scroll">
              {parsedLyrics.map((line, index) => (
                <div
                  className={`lyric-line ${index === activeIndex ? "is-active" : ""}`}
                  key={`${line.timeMs}-${index}`}
                  ref={index === activeIndex ? activeLineRef : null}
                >
                  {line.text}
                </div>
              ))}
            </div>
          ) : lyricsText ? (
            <div className="lyrics-plain">
              {lyricsText.split("\n").map((line, index) => (
                <div className="lyric-plain-line" key={`${line}-${index}`}>
                  {line}
                </div>
              ))}
            </div>
          ) : (
            <div className="lyrics-empty">
              <span>Текст песни отсутствует</span>
              <p>Добавьте его в редакторе трека справа.</p>
            </div>
          )}
        </div>
      ) : (
        <div
          aria-label="Обложка трека"
          className={`cover ${artworkUrl ? "has-artwork" : ""}`}
          style={artworkStyle(artworkUrl)}
        />
      )}
      <div className="now-meta">
        <h2>{currentTrack?.title || "Нет выбранного трека"}</h2>
        <p>{currentTrack?.artist || "Локальная медиатека"}</p>
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
  albums,
  libraryFolders,
  view,
  activePlaylist,
  activePlaylistTrackIds,
  currentTrackId,
  onViewChange,
  onPlayTrack,
  onAddTrackToPlaylist,
}: CollectionPanelProps) {
  return (
    <>
      <div className="collection-toolbar">
        <div className="segmented" role="tablist" aria-label="Вид коллекции">
          {collectionTabs.map((tab) => (
            <button
              aria-selected={view === tab.id}
              className={view === tab.id ? "is-active" : ""}
              key={tab.id}
              onClick={() => onViewChange(tab.id)}
              role="tab"
              type="button"
            >
              {tab.label}
            </button>
          ))}
        </div>
        <span className="pill">
          {activePlaylist
            ? `В плейлисте «${activePlaylist.name}»: ${formatTrackCount(activePlaylist.trackCount)}`
            : qualityLabel(tracks)}
        </span>
      </div>

      <div id="collection">
        {view === "tracks" && (
          <div className="track-table">
            {tracks.slice(0, 12).map((track) => {
              const alreadyAdded = activePlaylistTrackIds.has(track.id);

              return (
                <div
                  className={`track-row ${track.id === currentTrackId ? "is-current" : ""} ${track.isMissing ? "is-missing" : ""}`}
                  key={track.id}
                >
                  <button
                    className="track-action play"
                    disabled={track.isMissing}
                    onClick={() => onPlayTrack(track)}
                    title={track.isMissing ? "Файл не найден на диске" : "Играть сейчас"}
                    type="button"
                  >
                    <Play size={14} aria-hidden="true" />
                  </button>
                  <strong>{track.title}</strong>
                  <span>{track.isMissing ? "Файл не найден" : track.artist || "Неизвестный исполнитель"}</span>
                  <time>{formatDuration(track.durationMs)}</time>
                  <button
                    className="track-action"
                    disabled={!activePlaylist || alreadyAdded || track.isMissing}
                    onClick={() => onAddTrackToPlaylist(track.id)}
                    title={
                      alreadyAdded
                        ? "Трек уже в активном плейлисте"
                        : activePlaylist
                          ? `Добавить в «${activePlaylist.name}»`
                          : "Создайте или выберите плейлист"
                    }
                    type="button"
                  >
                    {alreadyAdded ? <ListMusic size={14} aria-hidden="true" /> : <Plus size={14} aria-hidden="true" />}
                  </button>
                </div>
              );
            })}
            {tracks.length === 0 && (
              <div className="empty-state">Добавьте треки в настольной версии, чтобы собрать медиатеку.</div>
            )}
          </div>
        )}

        {view === "albums" && (
          <div className="info-grid">
            {albums.slice(0, 12).map((album) => (
              <div className="info-card" key={`${album.name}-${album.artist || ""}`}>
                <Disc3 size={18} aria-hidden="true" />
                <strong>{album.name || "Без альбома"}</strong>
                <span>{album.artist || "Разные исполнители"}</span>
                <small>{formatTrackCount(album.trackCount)}</small>
              </div>
            ))}
            {albums.length === 0 && <div className="empty-state">Альбомы появятся после импорта треков.</div>}
          </div>
        )}

        {view === "folders" && (
          <div className="folder-summary-list">
            {libraryFolders.map((folder) => (
              <div className="folder-summary" key={folder.id} title={folder.path}>
                <Folder size={18} aria-hidden="true" />
                <strong>{compactPath(folder.path)}</strong>
                <span>{folder.lastScannedAt ? `Последнее сканирование: ${formatDate(folder.lastScannedAt)}` : "Еще не сканировалась"}</span>
              </div>
            ))}
            {libraryFolders.length === 0 && (
              <div className="empty-state">Папки библиотеки добавляются в настольной версии Fuse.</div>
            )}
          </div>
        )}
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
  playbackBackend,
  repeat,
  shuffle,
  volume,
  onNext,
  onPrevious,
  onSeek,
  onStop,
  onTogglePlay,
  onToggleRepeat,
  onToggleShuffle,
  onVolumeChange,
}: PlayerPanelProps) {
  const progress = durationMs ? Math.min(100, (currentTimeMs / durationMs) * 100) : 0;

  return (
    <div className="player-layout">
      <div className="mini-track">
        <div
          className={`mini-cover ${artworkUrl ? "has-artwork" : ""}`}
          aria-hidden="true"
          style={artworkStyle(artworkUrl)}
        />
        <div>
          <strong>{currentTrack?.title || "Выберите трек"}</strong>
          <span>
            {currentTrack
              ? `${currentTrack.artist || "Неизвестный исполнитель"} · ${currentTrack.format}`
              : "Локальная медиатека"}
          </span>
        </div>
      </div>

      <div className="transport">
        <div className="transport-buttons">
          <button className={`round-btn ${shuffle ? "is-active" : ""}`} type="button" aria-label="Перемешать" onClick={onToggleShuffle}>
            <Shuffle size={16} aria-hidden="true" />
          </button>
          <button className="round-btn" type="button" aria-label="Предыдущий трек" onClick={onPrevious}>
            <SkipBack size={17} aria-hidden="true" />
          </button>
          <button className="round-btn primary" type="button" aria-label={isPlaying ? "Пауза" : "Играть"} onClick={onTogglePlay}>
            {isPlaying ? <Pause size={19} aria-hidden="true" /> : <Play size={19} aria-hidden="true" />}
          </button>
          <button className="round-btn" type="button" aria-label="Следующий трек" onClick={onNext}>
            <SkipForward size={17} aria-hidden="true" />
          </button>
          <button className="round-btn" type="button" aria-label="Остановить" onClick={onStop}>
            <Square size={14} aria-hidden="true" />
          </button>
          <button className={`round-btn ${repeat ? "is-active" : ""}`} type="button" aria-label="Повтор" onClick={onToggleRepeat}>
            <Repeat size={16} aria-hidden="true" />
          </button>
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
            aria-label="Позиция трека"
            style={{ "--value": `${progress}%` } as CSSProperties}
          />
          <span>{formatDuration(durationMs || currentTrack?.durationMs)}</span>
        </div>
        {playbackError && <div className="player-error">{playbackError}</div>}
      </div>

      <div className="device-panel">
        <span>{playbackBackend === "rust" ? "Системный аудиодвижок" : "Аудио WebView"}</span>
        <label className="volume-line">
          <Volume2 size={15} aria-hidden="true" />
          <input
            className="range-line"
            type="range"
            min="0"
            max="100"
            value={Math.round(volume * 100)}
            onChange={(event) => onVolumeChange(Number(event.currentTarget.value) / 100)}
            aria-label="Громкость"
            style={{ "--value": `${Math.round(volume * 100)}%` } as CSSProperties}
          />
        </label>
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
            <span>{track.isMissing ? "Файл не найден" : track.artist || "Неизвестный исполнитель"}</span>
          </div>
          <time>{formatDuration(track.durationMs)}</time>
        </button>
      ))}
      {tracks.length === 0 && <div className="empty-state compact">Добавьте треки, чтобы собрать очередь.</div>}
    </div>
  );
}

export function SwarmPanel({
  p2pStatus,
  shares,
  transfers,
  ticketDraft,
  previewMode,
  currentTrack,
  activePlaylist,
  onTicketDraftChange,
  onStartP2p,
  onStopP2p,
  onShareTrack,
  onSharePlaylist,
  onCopyTicket,
  onDownloadTicket,
  onPauseShare,
  onResumeShare,
  onRevokeShare,
  onCancelTransfer,
  onRetryTransfer,
}: SwarmPanelProps) {
  const running = Boolean(p2pStatus?.running);
  const activeShares = shares.filter((share) => share.state === "active");
  const recentTransfers = transfers.slice(0, 4);

  return (
    <div className="swarm-panel">
      <div className="swarm-status-row">
        <div className={`swarm-state ${running ? "is-running" : "is-stopped"}`}>
          {running ? <Wifi size={16} aria-hidden="true" /> : <WifiOff size={16} aria-hidden="true" />}
          <div>
            <strong>{running ? "Swarm online" : "Swarm offline"}</strong>
            <span>{previewMode ? "Preview mode" : p2pStatus?.nodeId ? compactNode(p2pStatus.nodeId) : "Private tickets only"}</span>
          </div>
        </div>
        <button className="secondary-btn" type="button" onClick={running ? onStopP2p : onStartP2p}>
          {running ? <XCircle size={15} aria-hidden="true" /> : <RadioTower size={15} aria-hidden="true" />}
          {running ? "Стоп" : "Старт"}
        </button>
      </div>

      <div className="swarm-actions">
        <button className="secondary-btn" type="button" onClick={onShareTrack} disabled={!currentTrack || currentTrack.isMissing}>
          <Share2 size={15} aria-hidden="true" />
          Трек
        </button>
        <button className="secondary-btn" type="button" onClick={onSharePlaylist} disabled={!activePlaylist}>
          <Share2 size={15} aria-hidden="true" />
          Плейлист
        </button>
      </div>

      <div className="ticket-line">
        <Ticket size={15} aria-hidden="true" />
        <input
          aria-label="Fuse share ticket"
          type="text"
          value={ticketDraft}
          onChange={(event) => onTicketDraftChange(event.currentTarget.value)}
          placeholder="fuse-share:v1:..."
        />
        <button className="track-action play" type="button" title="Скачать по ticket" onClick={onDownloadTicket} disabled={!ticketDraft.trim()}>
          <Download size={14} aria-hidden="true" />
        </button>
      </div>

      <div className="swarm-lanes">
        <div className="swarm-lane">
          <div className="lane-head">
            <span>Раздачи</span>
            <strong>{activeShares.length}</strong>
          </div>
          <div className="swarm-list">
            {shares.slice(0, 4).map((share) => (
              <div className={`swarm-item is-${share.state}`} key={share.id}>
                <div>
                  <strong>{share.title}</strong>
                  <span>{share.scope === "playlist" ? `${share.itemCount} треков` : formatBytes(share.sizeBytes)}</span>
                </div>
                <div className="swarm-item-actions">
                  <button className="track-action" type="button" title="Скопировать ticket" onClick={() => onCopyTicket(share.ticket)}>
                    <Copy size={13} aria-hidden="true" />
                  </button>
                  {share.state === "active" ? (
                    <button className="track-action" type="button" title="Пауза" onClick={() => onPauseShare(share.id)}>
                      <Pause size={13} aria-hidden="true" />
                    </button>
                  ) : share.state === "paused" ? (
                    <button className="track-action play" type="button" title="Возобновить" onClick={() => onResumeShare(share.id)}>
                      <Play size={13} aria-hidden="true" />
                    </button>
                  ) : null}
                  <button className="track-action danger" type="button" title="Отозвать" onClick={() => onRevokeShare(share.id)}>
                    <Trash2 size={13} aria-hidden="true" />
                  </button>
                </div>
              </div>
            ))}
            {shares.length === 0 && <div className="empty-state compact">Выберите трек или плейлист и создайте приватный ticket.</div>}
          </div>
        </div>

        <div className="swarm-lane">
          <div className="lane-head">
            <span>Загрузки</span>
            <strong>{recentTransfers.length}</strong>
          </div>
          <div className="swarm-list">
            {recentTransfers.map((transfer) => (
              <div className={`swarm-item is-${transfer.status}`} key={transfer.id}>
                <div>
                  <strong>{transfer.title}</strong>
                  <span>{transfer.status} · {formatBytes(transfer.downloadedBytes || transfer.sizeBytes)}</span>
                </div>
                <div className="swarm-item-actions">
                  {transfer.status === "failed" && (
                    <button className="track-action play" type="button" title="Повторить" onClick={() => onRetryTransfer(transfer.id)}>
                      <RotateCcw size={13} aria-hidden="true" />
                    </button>
                  )}
                  {["pending", "downloading", "failed"].includes(transfer.status) && (
                    <button className="track-action danger" type="button" title="Отменить" onClick={() => onCancelTransfer(transfer.id)}>
                      <XCircle size={13} aria-hidden="true" />
                    </button>
                  )}
                </div>
              </div>
            ))}
            {recentTransfers.length === 0 && <div className="empty-state compact">Вставьте приватный ticket, чтобы скачать и проверить трек.</div>}
          </div>
        </div>
      </div>
    </div>
  );
}

export function MixerPanel({ tracks }: Pick<PanelProps, "tracks">) {
  const groups = useMemo(() => {
    const byFormat = new Map<string, number>();
    tracks.forEach((track) => byFormat.set(track.format, (byFormat.get(track.format) ?? 0) + 1));
    return [...byFormat.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6);
  }, [tracks]);
  const total = Math.max(1, tracks.length);

  return (
    <div className="format-panel">
      {groups.length > 0 ? (
        groups.map(([format, count]) => (
          <div className="format-row" key={format}>
            <span>{format}</span>
            <div className="format-meter" aria-hidden="true">
              <i style={{ "--value": `${Math.round((count / total) * 100)}%` } as CSSProperties} />
            </div>
            <strong>{formatTrackCount(count)}</strong>
          </div>
        ))
      ) : (
        <div className="empty-state compact">Форматы появятся после импорта локальной музыки.</div>
      )}
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
  onMoveTrack,
  onPlayPlaylist,
  onRenamePlaylist,
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
            <span className="playlist-card-art" aria-hidden="true">
              <i />
              <i />
              <i />
            </span>
            <strong>{playlist.name}</strong>
            <span>{formatTrackCount(playlist.trackCount)}</span>
          </button>
        ))}
      </div>

      {playlists.length === 0 && (
        <div className="empty-state">Создайте первый плейлист в правой панели, затем добавляйте в него треки из коллекции.</div>
      )}

      {activePlaylist && (
        <div className="playlist-detail">
          <div className="playlist-detail-head">
            <div>
              <strong>{activePlaylist.name}</strong>
              <span>{formatTrackCount(activePlaylist.trackCount)}</span>
            </div>
            <div className="playlist-actions">
              <button className="track-action play" type="button" title="Играть плейлист" onClick={onPlayPlaylist}>
                <Play size={14} aria-hidden="true" />
              </button>
              <button className="track-action danger" type="button" title="Удалить плейлист" onClick={onDeletePlaylist}>
                <Trash2 size={14} aria-hidden="true" />
              </button>
            </div>
          </div>

          <form
            className="playlist-rename"
            onSubmit={(event) => {
              event.preventDefault();
              const form = new FormData(event.currentTarget);
              onRenamePlaylist(String(form.get("name") ?? ""), String(form.get("description") ?? ""));
            }}
          >
            <input aria-label="Название плейлиста" defaultValue={activePlaylist.name} name="name" type="text" />
            <input aria-label="Описание плейлиста" defaultValue={activePlaylist.description ?? ""} name="description" type="text" placeholder="Описание" />
            <button className="secondary-btn" type="submit">Сохранить</button>
          </form>

          <div className="playlist-track-list">
            {activePlaylistTracks.slice(0, 8).map((track, index) => (
              <div className={`playlist-track ${track.isMissing ? "is-missing" : ""}`} key={track.id}>
                <div className={`playlist-track-art ${artworkUrls[track.id] ? "has-artwork" : ""}`} style={artworkStyle(artworkUrls[track.id])} />
                <div>
                  <strong>{track.title}</strong>
                  <span>{track.isMissing ? "Файл не найден" : track.artist || "Неизвестный исполнитель"}</span>
                </div>
                <div className="playlist-track-actions">
                  <button className="track-action" disabled={index === 0} type="button" title="Выше" onClick={() => onMoveTrack(track.id, -1)}>
                    <ArrowUp size={13} aria-hidden="true" />
                  </button>
                  <button
                    className="track-action"
                    disabled={index === activePlaylistTracks.length - 1}
                    type="button"
                    title="Ниже"
                    onClick={() => onMoveTrack(track.id, 1)}
                  >
                    <ArrowDown size={13} aria-hidden="true" />
                  </button>
                  <button className="track-action" type="button" title="Убрать из плейлиста" onClick={() => onRemoveTrack(track.id)}>
                    <Minus size={13} aria-hidden="true" />
                  </button>
                </div>
              </div>
            ))}
            {activePlaylistTracks.length === 0 && (
              <div className="empty-state compact">Добавьте треки кнопкой «+» в коллекции.</div>
            )}
          </div>
        </div>
      )}
    </>
  );
}

export function StatsPanel({ tracks, scanSummary }: Pick<PanelProps, "tracks" | "scanSummary">) {
  const lossless = tracks.filter((track) => ["FLAC", "WAV", "AIFF"].includes(track.format)).length;
  const needsAttention = tracks.filter((track) => track.missingTags || track.isMissing).length;

  return (
    <div className="stat-grid">
      <div className="stat"><strong>{compactNumber(tracks.length)}</strong><span>{formatTrackCount(tracks.length)}</span></div>
      <div className="stat"><strong>{compactNumber(lossless)}</strong><span>lossless-файлов</span></div>
      <div className="stat"><strong>{scanSummary?.errors.length || needsAttention}</strong><span>требуют внимания</span></div>
    </div>
  );
}

const collectionTabs: Array<{ id: CollectionView; label: string }> = [
  { id: "tracks", label: "Треки" },
  { id: "albums", label: "Альбомы" },
  { id: "folders", label: "Папки" },
];

export function formatTrackCount(value: number): string {
  const mod10 = value % 10;
  const mod100 = value % 100;
  const noun = mod10 === 1 && mod100 !== 11
    ? "трек"
    : mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)
      ? "трека"
      : "треков";

  return `${value} ${noun}`;
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

function formatDate(value?: number | null): string {
  if (!value) {
    return "неизвестно";
  }

  return new Intl.DateTimeFormat("ru", { dateStyle: "medium" }).format(value * 1000);
}

function compactNumber(value: number): string {
  if (value >= 1000) {
    return `${(value / 1000).toFixed(1)}k`;
  }

  return String(value);
}

function compactPath(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);

  if (parts.length <= 3) {
    return path;
  }

  return `.../${parts.slice(-3).join("/")}`;
}

function compactNode(nodeId: string): string {
  if (nodeId.length <= 16) {
    return nodeId;
  }

  return `${nodeId.slice(0, 8)}…${nodeId.slice(-6)}`;
}

function formatBytes(value: number): string {
  if (!value) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB"];
  let unitIndex = 0;
  let amount = value;

  while (amount >= 1024 && unitIndex < units.length - 1) {
    amount /= 1024;
    unitIndex += 1;
  }

  return `${amount >= 10 || unitIndex === 0 ? amount.toFixed(0) : amount.toFixed(1)} ${units[unitIndex]}`;
}

function artworkStyle(artworkUrl?: string | null): CSSProperties | undefined {
  return artworkUrl ? { backgroundImage: `url("${artworkUrl}")` } : undefined;
}
