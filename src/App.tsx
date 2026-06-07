import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import { InspectorPanel } from "./components/InspectorPanel";
import { ModuleCard } from "./components/ModuleCard";
import {
  CollectionPanel,
  LibraryPanel,
  MixerPanel,
  NowPlayingPanel,
  PlayerPanel,
  PlaylistsPanel,
  QueuePanel,
  StatsPanel,
} from "./components/Panels";
import {
  addTracksToPlaylist,
  audioSourceForTrack,
  createPlaylist,
  deletePlaylist,
  getLibrarySnapshot,
  getPlaylistTracks,
  getTrackArtwork,
  isTauriRuntime,
  loadLayoutProfile,
  pickArtworkFile,
  pickMusicFiles,
  pickMusicFolders,
  removeTrackFromPlaylist,
  saveLayoutProfile,
  scanLibrary,
  setTrackArtwork,
  updateTrackDetails,
} from "./lib/api";
import {
  applyPreset,
  defaultLayout,
  getBlock,
  normalizeLayout,
  updateBlock,
} from "./lib/layout";
import type {
  Density,
  LayoutProfile,
  LibrarySnapshot,
  ModuleId,
  ScanSummary,
  ThemeName,
  Track,
} from "./types";

const layoutStorageKey = "fuse.layout.v2";
const playbackStorageKey = "fuse.playback.v1";

interface StoredPlayback {
  trackId: number | null;
  volume: number;
  shuffle: boolean;
  repeat: boolean;
}

interface TrackEditorDraft {
  title: string;
  artist: string;
  album: string;
  lyrics: string;
}

const moduleMeta: Record<ModuleId, { title: string; icon: string }> = {
  library: { title: "Медиатека", icon: "□" },
  now: { title: "Сейчас играет", icon: "♪" },
  collection: { title: "Коллекция", icon: "≡" },
  player: { title: "Плеер", icon: "▶" },
  queue: { title: "Очередь", icon: "↳" },
  mixer: { title: "Микшер", icon: "≋" },
  playlists: { title: "Плейлисты", icon: "▦" },
  stats: { title: "Сводка", icon: "◇" },
};

const sizeCycle = [
  { cols: 3, rows: 1 },
  { cols: 6, rows: 1 },
  { cols: 3, rows: 2 },
  { cols: 6, rows: 2 },
];

const emptySnapshot: LibrarySnapshot = {
  tracks: [],
  albums: [],
  artists: [],
  playlists: [],
};

function App() {
  const storedPlayback = useMemo(readStoredPlayback, []);
  const [layout, setLayout] = useState<LayoutProfile>(defaultLayout);
  const [library, setLibrary] = useState<LibrarySnapshot>(emptySnapshot);
  const [activePlaylistId, setActivePlaylistId] = useState<number | null>(null);
  const [activePlaylistTracks, setActivePlaylistTracks] = useState<Track[]>([]);
  const [playlistName, setPlaylistName] = useState("");
  const [trackEditorDraft, setTrackEditorDraft] = useState<TrackEditorDraft>({
    title: "",
    artist: "",
    album: "",
    lyrics: "",
  });
  const [artworkUrls, setArtworkUrls] = useState<Record<number, string | null>>({});
  const [playbackQueue, setPlaybackQueue] = useState<Track[]>([]);
  const [currentTrackId, setCurrentTrackId] = useState<number | null>(storedPlayback.trackId);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTimeMs, setCurrentTimeMs] = useState(0);
  const [durationMs, setDurationMs] = useState(0);
  const [volume, setVolume] = useState(storedPlayback.volume);
  const [shuffle, setShuffle] = useState(storedPlayback.shuffle);
  const [repeat, setRepeat] = useState(storedPlayback.repeat);
  const [pendingPlayback, setPendingPlayback] = useState(false);
  const [showLyrics, setShowLyrics] = useState(false);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [scanSummary, setScanSummary] = useState<ScanSummary | null>(null);
  const [search, setSearch] = useState("");
  const [backendStatus, setBackendStatus] = useState("Loading library...");
  const [scanning, setScanning] = useState(false);
  const [dragging, setDragging] = useState<ModuleId | null>(null);
  const [dropTarget, setDropTarget] = useState<ModuleId | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const workspaceRef = useRef<HTMLElement | null>(null);
  const layoutRef = useRef(layout);
  const readyRef = useRef(false);

  useEffect(() => {
    layoutRef.current = layout;
  }, [layout]);

  const refreshLibrary = useCallback(async (value = search) => {
    try {
      const snapshot = await getLibrarySnapshot({ search: value, limit: 500 });
      setLibrary(snapshot);
      setActivePlaylistId((current) => {
        if (current && snapshot.playlists.some((playlist) => playlist.id === current)) {
          return current;
        }

        return snapshot.playlists[0]?.id ?? null;
      });
      setBackendStatus(isTauriRuntime() ? "Rust backend ready" : "Browser preview with mock data");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }, [search]);

  useEffect(() => {
    const local = localStorage.getItem(layoutStorageKey);
    if (local) {
      setLayout(normalizeLayout(JSON.parse(local)));
    }

    loadLayoutProfile("Studio")
      .then((profile) => {
        if (profile) {
          setLayout(normalizeLayout(profile));
        }
      })
      .catch(() => undefined)
      .finally(() => {
        readyRef.current = true;
      });

    void refreshLibrary("");
  }, [refreshLibrary]);

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      void refreshLibrary(search);
    }, 180);

    return () => window.clearTimeout(timeout);
  }, [search, refreshLibrary]);

  useEffect(() => {
    localStorage.setItem(layoutStorageKey, JSON.stringify(layout));

    if (readyRef.current) {
      void saveLayoutProfile(layout).catch(() => undefined);
    }
  }, [layout]);

  const visibleModules = useMemo(
    () => layout.order.filter((id) => !layout.hidden.includes(id)),
    [layout],
  );

  const activePlaylist = useMemo(
    () => library.playlists.find((playlist) => playlist.id === activePlaylistId) ?? null,
    [activePlaylistId, library.playlists],
  );

  const activePlaylistTrackIds = useMemo(
    () => new Set(activePlaylistTracks.map((track) => track.id)),
    [activePlaylistTracks],
  );

  const playbackSource = useMemo(
    () => (activePlaylistTracks.length ? activePlaylistTracks : library.tracks),
    [activePlaylistTracks, library.tracks],
  );

  const effectiveQueue = playbackQueue.length ? playbackQueue : playbackSource;

  const currentTrack = useMemo(
    () => effectiveQueue.find((track) => track.id === currentTrackId) ?? effectiveQueue[0] ?? null,
    [currentTrackId, effectiveQueue],
  );

  const currentArtworkUrl = currentTrack ? artworkUrls[currentTrack.id] ?? null : null;

  useEffect(() => {
    if (!currentTrack) {
      setTrackEditorDraft({ title: "", artist: "", album: "", lyrics: "" });
      return;
    }

    setTrackEditorDraft({
      title: currentTrack.title,
      artist: currentTrack.artist ?? "",
      album: currentTrack.album ?? "",
      lyrics: currentTrack.lyrics ?? "",
    });
  }, [currentTrack?.id]);

  useEffect(() => {
    const targets = [
      currentTrack,
      ...effectiveQueue.slice(0, 6),
      ...activePlaylistTracks.slice(0, 6),
    ].filter((track): track is Track => Boolean(track?.hasArtwork));
    const missing = targets.filter((track) => !(track.id in artworkUrls));

    if (missing.length === 0) {
      return;
    }

    let cancelled = false;

    Promise.all(
      missing.map(async (track) => {
        const artwork = await getTrackArtwork(track.id).catch(() => null);
        return [track.id, artwork?.dataUrl ?? null] as const;
      }),
    ).then((entries) => {
      if (!cancelled) {
        setArtworkUrls((current) => {
          const next = { ...current };
          entries.forEach(([trackId, dataUrl]) => {
            next[trackId] = dataUrl;
          });
          return next;
        });
      }
    });

    return () => {
      cancelled = true;
    };
  }, [activePlaylistTracks, artworkUrls, currentTrack, effectiveQueue]);

  useEffect(() => {
    let cancelled = false;

    if (!activePlaylistId) {
      setActivePlaylistTracks([]);
      return;
    }

    getPlaylistTracks(activePlaylistId)
      .then((tracks) => {
        if (!cancelled) {
          setActivePlaylistTracks(tracks);
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setBackendStatus(readError(error));
          setActivePlaylistTracks([]);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [activePlaylistId, library.playlists]);

  useEffect(() => {
    const source = playbackQueue.length ? playbackQueue : playbackSource;

    if (source.length === 0) {
      setCurrentTrackId(null);
      return;
    }

    setCurrentTrackId((current) => {
      if (current && source.some((track) => track.id === current)) {
        return current;
      }

      return source[0].id;
    });
  }, [playbackQueue, playbackSource]);

  useEffect(() => {
    localStorage.setItem(
      playbackStorageKey,
      JSON.stringify({
        trackId: currentTrackId,
        volume,
        shuffle,
        repeat,
      } satisfies StoredPlayback),
    );
  }, [currentTrackId, volume, shuffle, repeat]);

  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.volume = volume;
    }
  }, [volume]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) {
      return;
    }

    setPlaybackError(null);

    if (!currentTrack) {
      audio.pause();
      audio.removeAttribute("src");
      audio.load();
      setCurrentTimeMs(0);
      setDurationMs(0);
      setIsPlaying(false);
      return;
    }

    const source = audioSourceForTrack(currentTrack);
    setCurrentTimeMs(0);
    setDurationMs(currentTrack.durationMs ?? 0);

    if (!source) {
      audio.pause();
      audio.removeAttribute("src");
      audio.load();
      setIsPlaying(false);

      if (pendingPlayback) {
        setPendingPlayback(false);
        setPlaybackError("Playback works in the Tauri desktop window after importing real local files.");
      }

      return;
    }

    if (audio.src !== source) {
      audio.src = source;
      audio.load();
    }

    if (pendingPlayback) {
      setPendingPlayback(false);
      void audio
        .play()
        .then(() => {
          setIsPlaying(true);
          setBackendStatus(`Playing: ${currentTrack.title}`);
        })
        .catch((error) => {
          setIsPlaying(false);
          setPlaybackError(readError(error));
        });
    }
  }, [currentTrack, pendingPlayback]);

  function setTheme(theme: ThemeName) {
    setLayout((current) => normalizeLayout({ ...current, theme }));
  }

  function setDensity(density: Density) {
    setLayout((current) => normalizeLayout({ ...current, density }));
  }

  function toggleModule(id: ModuleId) {
    setLayout((current) => {
      const hidden = current.hidden.includes(id)
        ? current.hidden.filter((item) => item !== id)
        : [...current.hidden, id];
      return normalizeLayout({ ...current, hidden });
    });
  }

  function resetLayout() {
    setLayout(defaultLayout);
    setScanSummary(null);
  }

  function applyLayoutPreset(name: string) {
    setLayout((current) => applyPreset(current, name));
  }

  function cycleModuleSize(id: ModuleId) {
    setLayout((current) => {
      const block = getBlock(current, id);
      const currentIndex = sizeCycle.findIndex(
        (size) => size.cols === block.cols && size.rows === block.rows,
      );
      const next = sizeCycle[(currentIndex + 1) % sizeCycle.length] || sizeCycle[0];
      return updateBlock(current, id, next);
    });
  }

  async function importFolders() {
    setScanning(true);
    setBackendStatus("Waiting for folder selection...");

    try {
      const paths = await pickMusicFolders();
      if (paths.length === 0) {
        setBackendStatus(isTauriRuntime() ? "Import cancelled" : "Open the Tauri window to scan folders");
        return;
      }

      setBackendStatus("Scanning local library...");
      const summary = await scanLibrary(paths);
      setScanSummary(summary);
      await refreshLibrary(search);
      setBackendStatus(`Scan complete: ${summary.added} added, ${summary.updated} updated`);
    } catch (error) {
      setBackendStatus(readError(error));
    } finally {
      setScanning(false);
    }
  }

  async function importTracks() {
    setScanning(true);
    setBackendStatus("Waiting for track selection...");

    try {
      const paths = await pickMusicFiles();
      if (paths.length === 0) {
        setBackendStatus(isTauriRuntime() ? "Import cancelled" : "Open the Tauri window to add tracks");
        return;
      }

      setBackendStatus("Adding selected tracks...");
      const summary = await scanLibrary(paths);
      setScanSummary(summary);
      await refreshLibrary(search);
      setBackendStatus(`Tracks added: ${summary.added} new, ${summary.updated} updated`);
    } catch (error) {
      setBackendStatus(readError(error));
    } finally {
      setScanning(false);
    }
  }

  async function createNewPlaylist() {
    try {
      const playlist = await createPlaylist(playlistName);
      setPlaylistName("");
      setActivePlaylistId(playlist.id);
      await refreshLibrary(search);
      setActivePlaylistId(playlist.id);
      setBackendStatus(`Playlist ready: ${playlist.name}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function addTrackToActivePlaylist(trackId: number) {
    try {
      const playlist = activePlaylist ?? (await createPlaylist("Quick Mix"));
      const updated = await addTracksToPlaylist(playlist.id, [trackId]);
      setActivePlaylistId(updated.id);
      await refreshLibrary(search);
      setActivePlaylistTracks(await getPlaylistTracks(updated.id));
      setBackendStatus(`Added to ${updated.name}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function removeFromActivePlaylist(trackId: number) {
    if (!activePlaylistId) {
      return;
    }

    try {
      await removeTrackFromPlaylist(activePlaylistId, trackId);
      await refreshLibrary(search);
      setActivePlaylistTracks(await getPlaylistTracks(activePlaylistId));
      setBackendStatus("Track removed from playlist");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function deleteActivePlaylist() {
    if (!activePlaylistId) {
      return;
    }

    try {
      await deletePlaylist(activePlaylistId);
      setActivePlaylistId(null);
      setActivePlaylistTracks([]);
      await refreshLibrary(search);
      setBackendStatus("Playlist deleted");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function saveTrackDetails() {
    if (!currentTrack) {
      return;
    }

    try {
      const updated = await updateTrackDetails(currentTrack.id, {
        title: trackEditorDraft.title,
        artist: trackEditorDraft.artist,
        album: trackEditorDraft.album,
        lyrics: trackEditorDraft.lyrics,
      });
      replaceTrack(updated);
      await refreshLibrary(search);
      setBackendStatus(`Track updated: ${updated.title}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function pickTrackArtwork() {
    if (!currentTrack) {
      return;
    }

    try {
      const imagePath = await pickArtworkFile();
      if (!imagePath) {
        setBackendStatus(isTauriRuntime() ? "Artwork selection cancelled" : "Open the Tauri window to choose artwork");
        return;
      }

      const updated = await setTrackArtwork(currentTrack.id, imagePath);
      replaceTrack(updated);
      const artwork = await getTrackArtwork(updated.id);
      setArtworkUrls((current) => ({ ...current, [updated.id]: artwork?.dataUrl ?? null }));
      await refreshLibrary(search);
      setBackendStatus(`Artwork updated: ${updated.title}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  function replaceTrack(updated: Track) {
    const replace = (track: Track) => (track.id === updated.id ? updated : track);

    setLibrary((current) => ({
      ...current,
      tracks: current.tracks.map(replace),
    }));
    setActivePlaylistTracks((current) => current.map(replace));
    setPlaybackQueue((current) => current.map(replace));
  }

  function playTrack(track: Track, queue = playbackSource) {
    const nextQueue = queue.length ? queue : [track];
    setPlaybackQueue(nextQueue);
    setCurrentTrackId(track.id);
    setPendingPlayback(true);
  }

  function playPlaylist() {
    const source = activePlaylistTracks.length ? activePlaylistTracks : playbackSource;
    if (source.length === 0) {
      setPlaybackError("Add tracks before starting playback.");
      return;
    }

    playTrack(source[0], source);
  }

  function togglePlayback() {
    const audio = audioRef.current;

    if (isPlaying) {
      audio?.pause();
      setIsPlaying(false);
      return;
    }

    if (!currentTrack) {
      const first = playbackSource[0];
      if (first) {
        playTrack(first, playbackSource);
      }
      return;
    }

    setPendingPlayback(true);
  }

  function playNext() {
    const next = getRelativeTrack(1);
    if (next) {
      playTrack(next, effectiveQueue);
    }
  }

  function playPrevious() {
    const audio = audioRef.current;
    if (audio && audio.currentTime > 3) {
      audio.currentTime = 0;
      setCurrentTimeMs(0);
      return;
    }

    const previous = getRelativeTrack(-1);
    if (previous) {
      playTrack(previous, effectiveQueue);
    }
  }

  function seekPlayback(percent: number) {
    const audio = audioRef.current;
    const targetDuration = Number.isFinite(audio?.duration) && audio?.duration ? audio.duration * 1000 : durationMs;
    if (!audio || !targetDuration) {
      return;
    }

    const nextTime = clamp(percent, 0, 1) * targetDuration;
    audio.currentTime = nextTime / 1000;
    setCurrentTimeMs(nextTime);
  }

  function updateVolume(nextVolume: number) {
    setVolume(clamp(nextVolume, 0, 1));
  }

  function handleTrackEnded() {
    if (repeat && currentTrack) {
      playTrack(currentTrack, effectiveQueue);
      return;
    }

    const next = getRelativeTrack(1, false);
    if (next) {
      playTrack(next, effectiveQueue);
      return;
    }

    setIsPlaying(false);
  }

  function getRelativeTrack(offset: 1 | -1, wrap = true): Track | null {
    if (effectiveQueue.length === 0) {
      return null;
    }

    if (shuffle && offset > 0 && effectiveQueue.length > 1) {
      const candidates = effectiveQueue.filter((track) => track.id !== currentTrack?.id);
      return candidates[Math.floor(Math.random() * candidates.length)] ?? null;
    }

    const currentIndex = Math.max(
      0,
      effectiveQueue.findIndex((track) => track.id === currentTrack?.id),
    );
    const nextIndex = currentIndex + offset;

    if (nextIndex >= 0 && nextIndex < effectiveQueue.length) {
      return effectiveQueue[nextIndex];
    }

    if (!wrap) {
      return null;
    }

    return offset > 0 ? effectiveQueue[0] : effectiveQueue[effectiveQueue.length - 1];
  }

  function handleDragStart(event: React.DragEvent<HTMLElement>) {
    const id = readModuleId(event.target);
    if (!id) {
      return;
    }

    setDragging(id);
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", id);
  }

  function handleDragEnd() {
    setDragging(null);
    setDropTarget(null);
  }

  function handleDragOver(event: React.DragEvent<HTMLElement>) {
    event.preventDefault();
    if (!dragging) {
      return;
    }

    const target = findInsertTarget(event.clientX, event.clientY);
    setDropTarget(target);

    setLayout((current) => {
      const order = current.order.filter((id) => id !== dragging);
      const index = target ? order.indexOf(target) : order.length;
      order.splice(index < 0 ? order.length : index, 0, dragging);
      return normalizeLayout({ ...current, order });
    });
  }

  function handleResizeStart(event: React.PointerEvent<HTMLButtonElement>, id: ModuleId) {
    const workspace = workspaceRef.current;
    const module = event.currentTarget.closest<HTMLElement>(".module");
    if (!workspace || !module) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    module.classList.add("resizing");
    workspace.classList.add("is-resizing");

    const rect = module.getBoundingClientRect();
    const startX = event.clientX;
    const startY = event.clientY;
    const startWidth = rect.width;
    const startHeight = rect.height;
    const pointerId = event.pointerId;
    event.currentTarget.setPointerCapture(pointerId);

    let animationFrame = 0;
    let latestX = startX;
    let latestY = startY;

    const applyResize = () => {
      animationFrame = 0;
      const metrics = getGridMetrics(workspace);
      const targetWidth = startWidth + latestX - startX;
      const targetHeight = startHeight + latestY - startY;
      const cols = clamp(
        Math.round((targetWidth + metrics.gap) / (metrics.columnWidth + metrics.gap)),
        2,
        metrics.columns,
      );
      const rows = clamp(Math.round((targetHeight - 20) / metrics.rowStep), 1, 4);
      setLayout((current) => updateBlock(current, id, { cols, rows }));
    };

    const handleMove = (moveEvent: PointerEvent) => {
      if (moveEvent.pointerId !== pointerId) {
        return;
      }

      latestX = moveEvent.clientX;
      latestY = moveEvent.clientY;

      if (!animationFrame) {
        animationFrame = window.requestAnimationFrame(applyResize);
      }
    };

    const handleUp = (upEvent: PointerEvent) => {
      if (upEvent.pointerId !== pointerId) {
        return;
      }

      if (animationFrame) {
        window.cancelAnimationFrame(animationFrame);
        applyResize();
      }

      module.classList.remove("resizing");
      module.classList.add("drag-release");
      workspace.classList.remove("is-resizing");
      window.setTimeout(() => module.classList.remove("drag-release"), 520);
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
      window.removeEventListener("pointercancel", handleUp);
    };

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
    window.addEventListener("pointercancel", handleUp);
  }

  function renderModule(id: ModuleId) {
    const panelProps = {
      tracks: library.tracks,
      albums: library.albums,
      artists: library.artists,
      playlists: library.playlists,
      scanSummary,
    };

    switch (id) {
      case "library":
        return <LibraryPanel {...panelProps} />;
      case "now":
        return (
          <NowPlayingPanel
            artworkUrl={currentArtworkUrl}
            currentTrack={currentTrack}
            isPlaying={isPlaying}
            currentTimeMs={currentTimeMs}
            showLyrics={showLyrics}
          />
        );
      case "collection":
        return (
          <CollectionPanel
            tracks={library.tracks}
            activePlaylist={activePlaylist}
            activePlaylistTrackIds={activePlaylistTrackIds}
            currentTrackId={currentTrack?.id ?? null}
            onPlayTrack={(track) => playTrack(track, library.tracks)}
            onAddTrackToPlaylist={addTrackToActivePlaylist}
          />
        );
      case "player":
        return (
          <PlayerPanel
            artworkUrl={currentArtworkUrl}
            currentTrack={currentTrack}
            currentTimeMs={currentTimeMs}
            durationMs={durationMs}
            isPlaying={isPlaying}
            playbackError={playbackError}
            repeat={repeat}
            shuffle={shuffle}
            volume={volume}
            onNext={playNext}
            onPrevious={playPrevious}
            onSeek={seekPlayback}
            onTogglePlay={togglePlayback}
            onToggleRepeat={() => setRepeat((current) => !current)}
            onToggleShuffle={() => setShuffle((current) => !current)}
            onVolumeChange={updateVolume}
          />
        );
      case "queue":
        return (
          <QueuePanel
            artworkUrls={artworkUrls}
            tracks={effectiveQueue}
            currentTrackId={currentTrack?.id ?? null}
            onPlayTrack={(track) => playTrack(track, effectiveQueue)}
          />
        );
      case "mixer":
        return <MixerPanel />;
      case "playlists":
        return (
          <PlaylistsPanel
            artworkUrls={artworkUrls}
            playlists={library.playlists}
            activePlaylistId={activePlaylistId}
            activePlaylistTracks={activePlaylistTracks}
            onSelectPlaylist={setActivePlaylistId}
            onRemoveTrack={removeFromActivePlaylist}
            onDeletePlaylist={deleteActivePlaylist}
            onPlayPlaylist={playPlaylist}
          />
        );
      case "stats":
        return <StatsPanel tracks={library.tracks} scanSummary={scanSummary} />;
    }
  }

  return (
    <div className="fuse-app" data-theme={layout.theme} data-density={layout.density}>
      <audio
        ref={audioRef}
        preload="metadata"
        onDurationChange={(event) => {
          const seconds = event.currentTarget.duration;
          setDurationMs(Number.isFinite(seconds) ? seconds * 1000 : (currentTrack?.durationMs ?? 0));
        }}
        onEnded={handleTrackEnded}
        onError={() => {
          setIsPlaying(false);
          setPlaybackError("Could not play this local audio file.");
        }}
        onLoadedMetadata={(event) => {
          const seconds = event.currentTarget.duration;
          setDurationMs(Number.isFinite(seconds) ? seconds * 1000 : (currentTrack?.durationMs ?? 0));
        }}
        onPause={() => setIsPlaying(false)}
        onPlay={() => setIsPlaying(true)}
        onTimeUpdate={(event) => setCurrentTimeMs(event.currentTarget.currentTime * 1000)}
      />
      <header className="titlebar">
        <div className="brand">
          <div className="logo" aria-hidden="true" />
          <div className="brand-copy">
            <strong>Fuse</strong>
            <span>локальная музыка, собранная под себя</span>
          </div>
        </div>

        <label className="command">
          <span aria-hidden="true">⌕</span>
          <input
            type="search"
            placeholder="Поиск треков, папок, тегов, плейлистов"
            value={search}
            onChange={(event) => setSearch(event.currentTarget.value)}
          />
        </label>

        <div className="top-actions">
          <button className="icon-btn" type="button" title="Добавить треки" onClick={importTracks}>
            +
          </button>
          <button className="icon-btn" type="button" title="Импорт папки" onClick={importFolders}>
            ⌁
          </button>
          <button className="icon-btn" type="button" title="Настройки">⚙</button>
          <button className="icon-btn" type="button" title="Свернуть">−</button>
          <button className="icon-btn" type="button" title="Развернуть">□</button>
        </div>
      </header>

      <main className="app-grid">
        <section
          className={`workspace ${dragging ? "is-dragging" : ""}`}
          ref={workspaceRef}
          aria-label="Рабочая область Fuse"
          onDragStart={handleDragStart}
          onDragOver={handleDragOver}
          onDragEnd={handleDragEnd}
          onDrop={handleDragEnd}
        >
          {visibleModules.map((id) => {
            const meta = moduleMeta[id];
            return (
              <ModuleCard
                key={id}
                id={id}
                title={meta.title}
                icon={meta.icon}
                block={getBlock(layout, id)}
                dragging={dragging === id}
                dropTarget={dropTarget === id}
                onCycleSize={cycleModuleSize}
                onResizeStart={handleResizeStart}
                headerActions={
                  id === "now" ? (
                    <button
                      className={`icon-btn ${showLyrics ? "is-active" : ""}`}
                      type="button"
                      title={showLyrics ? "Показать обложку" : "Показать текст песни"}
                      onClick={() => setShowLyrics(!showLyrics)}
                    >
                      💬
                    </button>
                  ) : undefined
                }
              >
                {renderModule(id)}
              </ModuleCard>
            );
          })}
        </section>

        <InspectorPanel
          artworkUrl={currentArtworkUrl}
          currentTrack={currentTrack}
          layout={layout}
          scanSummary={scanSummary}
          backendStatus={backendStatus}
          scanning={scanning}
          playlistName={playlistName}
          trackEditorDraft={trackEditorDraft}
          onThemeChange={setTheme}
          onDensityChange={setDensity}
          onPreset={applyLayoutPreset}
          onToggleModule={toggleModule}
          onReset={resetLayout}
          onImport={importFolders}
          onImportTracks={importTracks}
          onPlaylistNameChange={setPlaylistName}
          onCreatePlaylist={createNewPlaylist}
          onPickArtwork={pickTrackArtwork}
          onSaveTrackDetails={saveTrackDetails}
          onTrackEditorChange={setTrackEditorDraft}
        />
      </main>
    </div>
  );
}

function readModuleId(target: EventTarget): ModuleId | null {
  const element = target instanceof Element ? target.closest<HTMLElement>("[data-module]") : null;
  return element?.dataset.module as ModuleId | null;
}

function findInsertTarget(x: number, y: number): ModuleId | null {
  const candidates = [...document.querySelectorAll<HTMLElement>(".module:not(.dragging)")];
  const rowTarget = candidates.find((element) => {
    const box = element.getBoundingClientRect();
    const buffer = Math.max(18, box.height * 0.18);
    return y >= box.top - buffer && y <= box.bottom + buffer && x < box.left + box.width / 2;
  });

  if (rowTarget?.dataset.module) {
    return rowTarget.dataset.module as ModuleId;
  }

  const verticalTarget = candidates.reduce<{ offset: number; id: ModuleId | null }>(
    (closest, element) => {
      const box = element.getBoundingClientRect();
      const offset = y - box.top - box.height / 2;

      if (offset < 0 && offset > closest.offset) {
        return { offset, id: element.dataset.module as ModuleId };
      }

      return closest;
    },
    { offset: Number.NEGATIVE_INFINITY, id: null },
  );

  return verticalTarget.id;
}

function getGridMetrics(workspace: HTMLElement) {
  const styles = window.getComputedStyle(workspace);
  const columns = styles.gridTemplateColumns.split(" ").filter(Boolean).length || 12;
  const gap = Number.parseFloat(styles.columnGap) || 0;
  const width = workspace.getBoundingClientRect().width;
  const columnWidth = (width - gap * (columns - 1)) / columns;

  return { columns, gap, columnWidth, rowStep: 206 };
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function readStoredPlayback(): StoredPlayback {
  const fallback: StoredPlayback = {
    trackId: null,
    volume: 0.72,
    shuffle: false,
    repeat: false,
  };

  try {
    const raw = localStorage.getItem(playbackStorageKey);
    if (!raw) {
      return fallback;
    }

    const parsed = JSON.parse(raw) as Partial<StoredPlayback>;
    return {
      trackId: typeof parsed.trackId === "number" ? parsed.trackId : null,
      volume: typeof parsed.volume === "number" ? clamp(parsed.volume, 0, 1) : fallback.volume,
      shuffle: Boolean(parsed.shuffle),
      repeat: Boolean(parsed.repeat),
    };
  } catch {
    return fallback;
  }
}

function readError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }

  return "Unknown Fuse error";
}

export default App;
