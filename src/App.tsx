import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  BarChart3,
  FolderPlus,
  Library,
  ListMusic,
  MessageCircle,
  Music2,
  PanelRightClose,
  PanelRightOpen,
  Play,
  Plus,
  RadioTower,
  Rows3,
  Search,
  SlidersHorizontal,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
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
  SwarmPanel,
} from "./components/Panels";
import {
  addTracksToPlaylist,
  addLibraryFolder,
  audioSourceForTrack,
  cancelP2pTransfer,
  createPlaylist,
  createPlaylistShareTicket,
  createTrackShareTicket,
  deletePlaylist,
  getDiagnostics,
  getLibraryFolders,
  getLibrarySnapshot,
  getP2pSettings,
  getP2pStatus,
  getPlaylistTracks,
  getRustPlaybackState,
  getTrackArtwork,
  markTrackPlayed,
  isTauriRuntime,
  loadLayoutProfile,
  pauseRustPlayback,
  pickArtworkFile,
  pickMusicFiles,
  pickMusicFolders,
  playRustQueueIndex,
  recordClientError,
  removeLibraryFolder,
  removeTrackFromPlaylist,
  resumeP2pShare,
  reorderPlaylistTracks,
  retryP2pTransfer,
  revokeP2pShare,
  resumeRustPlayback,
  saveP2pSettings,
  saveLayoutProfile,
  seekRustPlayback,
  setRustPlaybackQueue,
  setRustPlaybackVolume,
  startDownloadFromTicket,
  startP2p,
  startScan,
  setTrackArtwork,
  stopP2p,
  stopRustPlayback,
  updateTrackDetails,
  updatePlaylist,
  listP2pShares,
  listP2pTransfers,
  pauseP2pShare,
} from "./lib/api";
import {
  applyPreset,
  defaultLayout,
  getBlock,
  normalizeLayout,
  updateBlock,
} from "./lib/layout";
import { readJsonStorage } from "./lib/storage";
import type {
  CollectionView,
  Density,
  LayoutProfile,
  LibraryFolder,
  LibrarySnapshot,
  ModuleId,
  P2pSettings,
  P2pStatus,
  ScanJob,
  ScanSummary,
  SharedItem,
  ThemeName,
  Track,
  TransferTask,
} from "./types";

const layoutStorageKey = "fuse.layout.v2";
const playbackStorageKey = "fuse.playback.v1";
type PlaybackBackend = "rust" | "webview";
type LayoutUpdater = (current: LayoutProfile) => LayoutProfile | Partial<LayoutProfile>;

interface StoredPlayback {
  trackId: number | null;
  queueIds: number[];
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

const moduleMeta: Record<ModuleId, { title: string; icon: LucideIcon }> = {
  library: { title: "Медиатека", icon: Library },
  now: { title: "Сейчас играет", icon: Music2 },
  collection: { title: "Коллекция", icon: ListMusic },
  player: { title: "Плеер", icon: Play },
  queue: { title: "Очередь", icon: Rows3 },
  mixer: { title: "Форматы", icon: SlidersHorizontal },
  swarm: { title: "Swarm", icon: RadioTower },
  playlists: { title: "Плейлисты", icon: FolderPlus },
  stats: { title: "Сводка", icon: BarChart3 },
};

const coreWorkspaceModules: ModuleId[] = ["now", "player", "collection", "queue"];

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
  const previewMode = !isTauriRuntime();
  const [layout, setLayout] = useState<LayoutProfile>(defaultLayout);
  const [library, setLibrary] = useState<LibrarySnapshot>(emptySnapshot);
  const [libraryFolders, setLibraryFolders] = useState<LibraryFolder[]>([]);
  const [p2pStatus, setP2pStatus] = useState<P2pStatus | null>(null);
  const [p2pSettings, setP2pSettings] = useState<P2pSettings>({
    enabled: false,
    autoSeedDownloads: true,
    importDir: null,
    uploadLimitKbps: null,
    downloadLimitKbps: null,
  });
  const [p2pShares, setP2pShares] = useState<SharedItem[]>([]);
  const [p2pTransfers, setP2pTransfers] = useState<TransferTask[]>([]);
  const [shareTicketDraft, setShareTicketDraft] = useState("");
  const [collectionView, setCollectionView] = useState<CollectionView>("tracks");
  const [inspectorOpen, setInspectorOpen] = useState(true);
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
  const [playbackBackend, setPlaybackBackend] = useState<PlaybackBackend>(isTauriRuntime() ? "rust" : "webview");
  const [showLyrics, setShowLyrics] = useState(false);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [scanSummary, setScanSummary] = useState<ScanSummary | null>(null);
  const [scanJob, setScanJob] = useState<ScanJob | null>(null);
  const [search, setSearch] = useState("");
  const [backendStatus, setBackendStatus] = useState(previewMode ? "Веб-превью: демо-данные" : "Загрузка медиатеки...");
  const [diagnosticsPath, setDiagnosticsPath] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [dragging, setDragging] = useState<ModuleId | null>(null);
  const [dropTarget, setDropTarget] = useState<ModuleId | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const workspaceRef = useRef<HTMLElement | null>(null);
  const layoutRef = useRef(layout);
  const readyRef = useRef(false);
  const restoredQueueRef = useRef(false);

  useEffect(() => {
    layoutRef.current = layout;
  }, [layout]);

  useEffect(() => {
    const handleError = (event: ErrorEvent) => {
      void recordClientError(event.message, event.filename || "window.error").catch(() => undefined);
    };
    const handleRejection = (event: PromiseRejectionEvent) => {
      void recordClientError(readError(event.reason), "unhandledrejection").catch(() => undefined);
    };

    window.addEventListener("error", handleError);
    window.addEventListener("unhandledrejection", handleRejection);
    return () => {
      window.removeEventListener("error", handleError);
      window.removeEventListener("unhandledrejection", handleRejection);
    };
  }, []);

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
      setBackendStatus(isTauriRuntime() ? "Настольный движок готов" : "Веб-превью: демо-данные");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }, [search]);

  const refreshLibraryFolders = useCallback(async () => {
    try {
      setLibraryFolders(await getLibraryFolders());
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }, []);

  const refreshDiagnostics = useCallback(async () => {
    try {
      const diagnostics = await getDiagnostics();
      setDiagnosticsPath(diagnostics.logPath ?? diagnostics.appDataDir ?? null);
    } catch {
      setDiagnosticsPath(null);
    }
  }, []);

  const refreshP2p = useCallback(async () => {
    try {
      const [status, settings, shares, transfers] = await Promise.all([
        getP2pStatus(),
        getP2pSettings(),
        listP2pShares(),
        listP2pTransfers(),
      ]);
      setP2pStatus(status);
      setP2pSettings(settings);
      setP2pShares(shares);
      setP2pTransfers(transfers);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }, []);

  useEffect(() => {
    setLayout(normalizeLayout(readJsonStorage<Partial<LayoutProfile> | null>(layoutStorageKey, null)));

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
    void refreshLibraryFolders();
    void refreshDiagnostics();
    void refreshP2p();
  }, [refreshDiagnostics, refreshLibrary, refreshLibraryFolders, refreshP2p]);

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
  const hiddenModules = useMemo(
    () => layout.order.filter((id) => layout.hidden.includes(id)),
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
    if (restoredQueueRef.current || storedPlayback.queueIds.length === 0 || library.tracks.length === 0) {
      return;
    }

    const byId = new Map(library.tracks.map((track) => [track.id, track]));
    const restored = storedPlayback.queueIds
      .map((trackId) => byId.get(trackId))
      .filter((track): track is Track => Boolean(track));

    restoredQueueRef.current = true;

    if (restored.length > 0) {
      setPlaybackQueue(restored);
    }
  }, [library.tracks, storedPlayback.queueIds]);

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
        queueIds: playbackQueue.map((track) => track.id),
        volume,
        shuffle,
        repeat,
      } satisfies StoredPlayback),
    );
  }, [currentTrackId, playbackQueue, volume, shuffle, repeat]);

  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.volume = volume;
    }

    if (playbackBackend === "rust") {
      void setRustPlaybackVolume(volume).catch(() => undefined);
    }
  }, [playbackBackend, volume]);

  useEffect(() => {
    if (playbackBackend !== "rust" || !isPlaying) {
      return;
    }

    const interval = window.setInterval(() => {
      void getRustPlaybackState()
        .then((state) => {
          if (!state) {
            return;
          }

          setCurrentTimeMs(state.positionMs);
          setDurationMs(state.durationMs ?? currentTrack?.durationMs ?? 0);
          if (state.status === "stopped" && isPlaying) {
            setIsPlaying(false);
            handleTrackEnded();
            return;
          }

          setIsPlaying(state.status === "playing");
          if (state.error) {
            setPlaybackError(state.error);
          }
        })
        .catch((error) => {
          setPlaybackError(readError(error));
        });
    }, 500);

    return () => window.clearInterval(interval);
  }, [currentTrack?.durationMs, isPlaying, playbackBackend]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target;
      const isTextInput =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement;

      if (isTextInput) {
        return;
      }

      if (event.code === "Space") {
        event.preventDefault();
        togglePlayback();
      }

      if (event.ctrlKey && event.key.toLowerCase() === "f") {
        event.preventDefault();
        document.querySelector<HTMLInputElement>(".command input")?.focus();
      }

      if (event.ctrlKey && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void importTracks();
      }

      if (event.key === "ArrowRight") {
        playNext();
      }

      if (event.key === "ArrowLeft") {
        playPrevious();
      }

      if (event.key === "Delete" && currentTrack && activePlaylistTrackIds.has(currentTrack.id)) {
        void removeFromActivePlaylist(currentTrack.id);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) {
      return;
    }

    if (playbackBackend === "rust") {
      audio.pause();
      audio.removeAttribute("src");
      audio.load();
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
        setPlaybackError("В веб-превью нет доступа к локальному аудиофайлу. Воспроизведение доступно в настольной версии после импорта.");
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
          recordSuccessfulPlay(currentTrack);
          setBackendStatus(`Играет: ${currentTrack.title}`);
        })
        .catch((error) => {
          setIsPlaying(false);
          setPlaybackError(readError(error));
        });
    }
  }, [currentTrack, pendingPlayback, playbackBackend]);

  function updateLayoutWithTransition(updater: LayoutUpdater) {
    const apply = () => {
      setLayout((current) => normalizeLayout(updater(current)));
    };
    const transitionDocument = document as Document & {
      startViewTransition?: (callback: () => void) => void;
    };

    if (transitionDocument.startViewTransition && !prefersReducedMotion()) {
      transitionDocument.startViewTransition(apply);
      return;
    }

    apply();
  }

  function setTheme(theme: ThemeName) {
    setLayout((current) => normalizeLayout({ ...current, theme }));
  }

  function setDensity(density: Density) {
    setLayout((current) => normalizeLayout({ ...current, density }));
  }

  function toggleModule(id: ModuleId) {
    updateLayoutWithTransition((current) => {
      const hidden = current.hidden.includes(id)
        ? current.hidden.filter((item) => item !== id)
        : [...current.hidden, id];
      return { ...current, hidden };
    });
  }

  function showModule(id: ModuleId) {
    updateLayoutWithTransition((current) => ({
      ...current,
      hidden: current.hidden.filter((item) => item !== id),
    }));
  }

  function hideModule(id: ModuleId) {
    updateLayoutWithTransition((current) => ({
      ...current,
      hidden: current.hidden.includes(id) ? current.hidden : [...current.hidden, id],
    }));
  }

  function hideAllModules() {
    updateLayoutWithTransition((current) => ({
      ...current,
      hidden: [...current.order],
    }));
  }

  function showAllModules() {
    updateLayoutWithTransition((current) => ({
      ...current,
      hidden: [],
    }));
  }

  function showCoreWorkspace() {
    updateLayoutWithTransition((current) => ({
      ...current,
      hidden: current.order.filter((id) => !coreWorkspaceModules.includes(id)),
    }));
  }

  function resetLayout() {
    updateLayoutWithTransition(() => defaultLayout);
    setScanSummary(null);
  }

  function applyLayoutPreset(name: string) {
    updateLayoutWithTransition((current) => applyPreset(current, name));
  }

  function cycleModuleSize(id: ModuleId) {
    updateLayoutWithTransition((current) => {
      const block = getBlock(current, id);
      const currentIndex = sizeCycle.findIndex(
        (size) => size.cols === block.cols && size.rows === block.rows,
      );
      const next = sizeCycle[(currentIndex + 1) % sizeCycle.length] || sizeCycle[0];
      return updateBlock(current, id, next);
    });
  }

  async function importFolders() {
    if (previewMode) {
      setBackendStatus("Импорт папок доступен в настольной версии Fuse.");
      return;
    }

    setScanning(true);
    setBackendStatus("Ожидание выбора папки...");

    try {
      const paths = await pickMusicFolders();
      if (paths.length === 0) {
        setBackendStatus("Импорт отменен");
        return;
      }

      await Promise.all(paths.map((path) => addLibraryFolder(path).catch(() => null)));
      await refreshLibraryFolders();
      setBackendStatus("Сканирование локальной медиатеки...");
      const job = await startScan(paths, { registerFolders: true });
      const summary = scanSummaryFromJob(job);
      setScanJob(job);
      setScanSummary(summary);
      await refreshLibrary(search);
      setBackendStatus(`Сканирование завершено: добавлено ${summary.added}, обновлено ${summary.updated}`);
    } catch (error) {
      setBackendStatus(readError(error));
    } finally {
      setScanning(false);
    }
  }

  async function importTracks() {
    if (previewMode) {
      setBackendStatus("Добавление треков доступно в настольной версии Fuse.");
      return;
    }

    setScanning(true);
    setBackendStatus("Ожидание выбора треков...");

    try {
      const paths = await pickMusicFiles();
      if (paths.length === 0) {
        setBackendStatus("Импорт отменен");
        return;
      }

      setBackendStatus("Добавление выбранных треков...");
      const job = await startScan(paths);
      const summary = scanSummaryFromJob(job);
      setScanJob(job);
      setScanSummary(summary);
      await refreshLibrary(search);
      setBackendStatus(`Треки добавлены: новых ${summary.added}, обновлено ${summary.updated}`);
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
      setBackendStatus(`Плейлист готов: ${playlist.name}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function addTrackToActivePlaylist(trackId: number) {
    try {
      const playlist = activePlaylist ?? (await createPlaylist("Быстрый микс"));
      const updated = await addTracksToPlaylist(playlist.id, [trackId]);
      setActivePlaylistId(updated.id);
      await refreshLibrary(search);
      setActivePlaylistTracks(await getPlaylistTracks(updated.id));
      setBackendStatus(`Добавлено в «${updated.name}»`);
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
      setBackendStatus("Трек удален из плейлиста");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function deleteActivePlaylist() {
    if (!activePlaylistId) {
      return;
    }

    const playlistNameForPrompt = activePlaylist?.name ?? "выбранный плейлист";
    if (!window.confirm(`Удалить плейлист «${playlistNameForPrompt}»?`)) {
      return;
    }

    try {
      await deletePlaylist(activePlaylistId);
      setActivePlaylistId(null);
      setActivePlaylistTracks([]);
      await refreshLibrary(search);
      setBackendStatus("Плейлист удален");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function renameActivePlaylist(name: string, description?: string | null) {
    if (!activePlaylistId) {
      return;
    }

    try {
      const updated = await updatePlaylist(activePlaylistId, { name, description });
      await refreshLibrary(search);
      setActivePlaylistId(updated.id);
      setBackendStatus(`Плейлист обновлен: ${updated.name}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function moveTrackInActivePlaylist(trackId: number, direction: -1 | 1) {
    if (!activePlaylistId) {
      return;
    }

    const index = activePlaylistTracks.findIndex((track) => track.id === trackId);
    const nextIndex = index + direction;
    if (index < 0 || nextIndex < 0 || nextIndex >= activePlaylistTracks.length) {
      return;
    }

    const reordered = [...activePlaylistTracks];
    const [track] = reordered.splice(index, 1);
    reordered.splice(nextIndex, 0, track);

    try {
      await reorderPlaylistTracks(activePlaylistId, reordered.map((item) => item.id));
      setActivePlaylistTracks(reordered);
      await refreshLibrary(search);
      setBackendStatus("Порядок плейлиста обновлен");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function removeFolder(folderId: number) {
    if (!window.confirm("Убрать папку из библиотеки Fuse? Треки останутся на диске.")) {
      return;
    }

    try {
      await removeLibraryFolder(folderId);
      await refreshLibraryFolders();
      setBackendStatus("Папка удалена из библиотеки");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function rescanFolder(path: string) {
    if (previewMode) {
      setBackendStatus("Пересканирование доступно в настольной версии Fuse.");
      return;
    }

    setScanning(true);
    setBackendStatus(`Сканирование: ${path}`);
    try {
      const job = await startScan([path], { registerFolders: true });
      const summary = scanSummaryFromJob(job);
      setScanJob(job);
      setScanSummary(summary);
      await refreshLibrary(search);
      await refreshLibraryFolders();
      setBackendStatus(`Папка обновлена: добавлено ${summary.added}, обновлено ${summary.updated}`);
    } catch (error) {
      setBackendStatus(readError(error));
    } finally {
      setScanning(false);
    }
  }

  async function startSwarm() {
    try {
      const status = await startP2p();
      setP2pStatus(status);
      await refreshP2p();
      setBackendStatus("Swarm включен: приватные ticket-раздачи доступны");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function stopSwarm() {
    try {
      const status = await stopP2p();
      setP2pStatus(status);
      await refreshP2p();
      setBackendStatus("Swarm остановлен");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function updateP2pSettings(settings: P2pSettings) {
    try {
      const saved = await saveP2pSettings(settings);
      setP2pSettings(saved);
      await refreshP2p();
      setBackendStatus("Настройки Swarm сохранены");
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function shareCurrentTrack() {
    if (!currentTrack) {
      setBackendStatus("Выберите трек для приватной раздачи");
      return;
    }

    try {
      const share = await createTrackShareTicket(currentTrack.id);
      setShareTicketDraft(share.ticket);
      await refreshP2p();
      setBackendStatus(`Ticket создан: ${share.title}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function shareActivePlaylist() {
    if (!activePlaylist) {
      setBackendStatus("Выберите плейлист для приватной раздачи");
      return;
    }

    try {
      const share = await createPlaylistShareTicket(activePlaylist.id);
      setShareTicketDraft(share.ticket);
      await refreshP2p();
      setBackendStatus(`Ticket плейлиста создан: ${share.title}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function copyShareTicket(ticket: string) {
    try {
      await navigator.clipboard.writeText(ticket);
      setBackendStatus("Ticket скопирован в буфер обмена");
    } catch {
      setShareTicketDraft(ticket);
      setBackendStatus("Ticket помещен в поле ввода");
    }
  }

  async function downloadShareTicket() {
    const ticket = shareTicketDraft.trim();
    if (!ticket) {
      return;
    }

    try {
      const transfer = await startDownloadFromTicket(ticket);
      await refreshLibrary(search);
      await refreshP2p();
      setBackendStatus(`Swarm загрузка: ${transfer.status}`);
    } catch (error) {
      await refreshP2p();
      setBackendStatus(readError(error));
    }
  }

  async function pauseShare(shareId: number) {
    try {
      await pauseP2pShare(shareId);
      await refreshP2p();
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function resumeShare(shareId: number) {
    try {
      await resumeP2pShare(shareId);
      await refreshP2p();
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function revokeShare(shareId: number) {
    if (!window.confirm("Отозвать приватную раздачу? Уже скопированные ticket нельзя забрать обратно.")) {
      return;
    }

    try {
      await revokeP2pShare(shareId);
      await refreshP2p();
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function cancelTransfer(transferId: number) {
    try {
      await cancelP2pTransfer(transferId);
      await refreshP2p();
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function retryTransfer(transferId: number) {
    try {
      await retryP2pTransfer(transferId);
      await refreshLibrary(search);
      await refreshP2p();
    } catch (error) {
      await refreshP2p();
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
      setBackendStatus(`Трек обновлен: ${updated.title}`);
    } catch (error) {
      setBackendStatus(readError(error));
    }
  }

  async function pickTrackArtwork() {
    if (!currentTrack) {
      return;
    }

    if (previewMode) {
      setBackendStatus("Обложка меняется в настольной версии Fuse.");
      return;
    }

    try {
      const imagePath = await pickArtworkFile();
      if (!imagePath) {
        setBackendStatus(isTauriRuntime() ? "Выбор обложки отменен" : "Обложка меняется в настольной версии Fuse");
        return;
      }

      const updated = await setTrackArtwork(currentTrack.id, imagePath);
      replaceTrack(updated);
      const artwork = await getTrackArtwork(updated.id);
      setArtworkUrls((current) => ({ ...current, [updated.id]: artwork?.dataUrl ?? null }));
      await refreshLibrary(search);
      setBackendStatus(`Обложка обновлена: ${updated.title}`);
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

  function recordSuccessfulPlay(track: Track) {
    void markTrackPlayed(track.id)
      .then((updated) => {
        if (updated) {
          replaceTrack(updated);
        }
      })
      .catch(() => undefined);
  }

  function playTrack(track: Track, queue = playbackSource) {
    if (track.isMissing) {
      setPlaybackError("Файл не найден на диске. Пересканируйте библиотеку или верните файл.");
      setBackendStatus(`Файл не найден: ${track.title}`);
      return;
    }

    const nextQueue = queue.length ? queue : [track];
    setPlaybackQueue(nextQueue);
    setCurrentTrackId(track.id);
    setPlaybackError(null);

    if (!isTauriRuntime()) {
      setPlaybackBackend("webview");
      setPendingPlayback(true);
      return;
    }

    void startRustPlayback(track, nextQueue);
  }

  async function startRustPlayback(track: Track, queue: Track[]) {
    const startIndex = Math.max(
      0,
      queue.findIndex((item) => item.id === track.id),
    );

    try {
      setPlaybackBackend("rust");
      await setRustPlaybackQueue(queue.map((item) => item.id), startIndex);
      const state = await playRustQueueIndex(startIndex);
      setPendingPlayback(false);
      setIsPlaying(state?.status === "playing");
      setCurrentTimeMs(state?.positionMs ?? 0);
      setDurationMs(state?.durationMs ?? track.durationMs ?? 0);
      if (state?.status === "playing") {
        recordSuccessfulPlay(track);
      }
      setBackendStatus(`Системное аудио: ${track.title}`);
    } catch (error) {
      setPlaybackBackend("webview");
      setPendingPlayback(true);
      setPlaybackError(`Переключение на WebView: ${readError(error)}`);
    }
  }

  function playPlaylist() {
    const source = activePlaylistTracks.length ? activePlaylistTracks : playbackSource;
    if (source.length === 0) {
      setPlaybackError("Добавьте треки перед запуском плейлиста.");
      return;
    }

    playTrack(source[0], source);
  }

  function togglePlayback() {
    const audio = audioRef.current;

    if (isPlaying) {
      if (playbackBackend === "rust") {
        void pauseRustPlayback()
          .then((state) => {
            setIsPlaying(false);
            setCurrentTimeMs(state?.positionMs ?? currentTimeMs);
          })
          .catch((error) => setPlaybackError(readError(error)));
        return;
      }

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

    if (playbackBackend === "rust" && isTauriRuntime()) {
      void resumeRustPlayback()
        .then((state) => {
          setIsPlaying(state?.status === "playing");
          setCurrentTimeMs(state?.positionMs ?? currentTimeMs);
        })
        .catch((error) => {
          setPlaybackBackend("webview");
          setPendingPlayback(true);
          setPlaybackError(`Переключение на WebView: ${readError(error)}`);
        });
      return;
    }

    setPendingPlayback(true);
  }

  function stopPlayback() {
    const audio = audioRef.current;

    if (playbackBackend === "rust" && isTauriRuntime()) {
      void stopRustPlayback()
        .then((state) => {
          setIsPlaying(false);
          setCurrentTimeMs(state?.positionMs ?? 0);
        })
        .catch((error) => setPlaybackError(readError(error)));
      return;
    }

    if (audio) {
      audio.pause();
      audio.currentTime = 0;
    }

    setIsPlaying(false);
    setCurrentTimeMs(0);
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
    if (!targetDuration) {
      return;
    }

    const nextTime = clamp(percent, 0, 1) * targetDuration;
    if (playbackBackend === "rust" && isTauriRuntime()) {
      void seekRustPlayback(nextTime)
        .then((state) => {
          setCurrentTimeMs(state?.positionMs ?? nextTime);
        })
        .catch((error) => setPlaybackError(readError(error)));
      return;
    }

    if (!audio) {
      return;
    }

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
    const module = (event.target instanceof Element
      ? event.target.closest<HTMLElement>("[data-module]")
      : null);
    const meta = moduleMeta[id];

    if (module) {
      const rect = module.getBoundingClientRect();
      const ghost = document.createElement("div");
      ghost.className = "drag-preview";
      ghost.textContent = meta.title;
      ghost.style.width = `${Math.min(260, Math.max(180, rect.width * 0.72))}px`;
      document.body.appendChild(ghost);
      event.dataTransfer.setDragImage(ghost, 24, 22);
      window.setTimeout(() => ghost.remove(), 0);
    }
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
            albums={library.albums}
            libraryFolders={libraryFolders}
            view={collectionView}
            activePlaylist={activePlaylist}
            activePlaylistTrackIds={activePlaylistTrackIds}
            currentTrackId={currentTrack?.id ?? null}
            onViewChange={setCollectionView}
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
            playbackBackend={playbackBackend}
            repeat={repeat}
            shuffle={shuffle}
            volume={volume}
            onNext={playNext}
            onPrevious={playPrevious}
            onSeek={seekPlayback}
            onStop={stopPlayback}
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
        return <MixerPanel tracks={library.tracks} />;
      case "swarm":
        return (
          <SwarmPanel
            p2pStatus={p2pStatus}
            shares={p2pShares}
            transfers={p2pTransfers}
            ticketDraft={shareTicketDraft}
            previewMode={previewMode}
            currentTrack={currentTrack}
            activePlaylist={activePlaylist}
            onTicketDraftChange={setShareTicketDraft}
            onStartP2p={startSwarm}
            onStopP2p={stopSwarm}
            onShareTrack={shareCurrentTrack}
            onSharePlaylist={shareActivePlaylist}
            onCopyTicket={copyShareTicket}
            onDownloadTicket={downloadShareTicket}
            onPauseShare={pauseShare}
            onResumeShare={resumeShare}
            onRevokeShare={revokeShare}
            onCancelTransfer={cancelTransfer}
            onRetryTransfer={retryTransfer}
          />
        );
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
            onMoveTrack={moveTrackInActivePlaylist}
            onPlayPlaylist={playPlaylist}
            onRenamePlaylist={renameActivePlaylist}
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
          setPlaybackError("Не удалось воспроизвести локальный аудиофайл.");
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
          <Search size={16} aria-hidden="true" />
          <input
            type="search"
            placeholder="Поиск треков, папок, тегов, плейлистов"
            value={search}
            onChange={(event) => setSearch(event.currentTarget.value)}
          />
        </label>

        <div className="top-actions">
          {previewMode && (
            <span className="mode-badge" title="Веб-превью работает с демо-данными">
              Демо-режим
            </span>
          )}
          <button
            className="icon-btn"
            type="button"
            title={previewMode ? "Импорт доступен в настольной версии" : "Добавить треки"}
            onClick={importTracks}
            disabled={previewMode}
          >
            <Plus size={17} aria-hidden="true" />
          </button>
          <button
            className="icon-btn"
            type="button"
            title={previewMode ? "Импорт доступен в настольной версии" : "Импорт папки"}
            onClick={importFolders}
            disabled={previewMode}
          >
            <FolderPlus size={17} aria-hidden="true" />
          </button>
          <button className="icon-btn" type="button" title={inspectorOpen ? "Скрыть настройки" : "Показать настройки"} onClick={() => setInspectorOpen((current) => !current)}>
            {inspectorOpen ? <PanelRightClose size={17} aria-hidden="true" /> : <PanelRightOpen size={17} aria-hidden="true" />}
          </button>
        </div>
      </header>

      <main className={`app-grid ${inspectorOpen ? "has-inspector" : "is-inspector-closed"}`}>
        <section
          className={`workspace ${dragging ? "is-dragging" : ""}`}
          ref={workspaceRef}
          aria-label="Рабочая область Fuse"
          onDragStart={handleDragStart}
          onDragOver={handleDragOver}
          onDragEnd={handleDragEnd}
          onDrop={handleDragEnd}
        >
          {visibleModules.length === 0 ? (
            <WorkspaceEmpty
              hiddenModules={hiddenModules}
              onImportTracks={importTracks}
              onShowAll={showAllModules}
              onShowCore={showCoreWorkspace}
              onShowModule={showModule}
              onStudio={() => applyLayoutPreset("Studio")}
            />
          ) : (
            <>
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
                    onHide={hideModule}
                    onResizeStart={handleResizeStart}
                    headerActions={
                      id === "now" ? (
                        <button
                          className={`icon-btn ${showLyrics ? "is-active" : ""}`}
                          type="button"
                          title={showLyrics ? "Показать обложку" : "Показать текст песни"}
                          aria-label={showLyrics ? "Показать обложку" : "Показать текст песни"}
                          onClick={() => setShowLyrics(!showLyrics)}
                        >
                          <MessageCircle size={16} aria-hidden="true" />
                        </button>
                      ) : undefined
                    }
                  >
                    {renderModule(id)}
                  </ModuleCard>
                );
              })}

              {hiddenModules.length > 0 && (
                <HiddenModuleDock
                  hiddenModules={hiddenModules}
                  onShowAll={showAllModules}
                  onShowModule={showModule}
                />
              )}
            </>
          )}
        </section>

        <div className={`inspector-shell ${inspectorOpen ? "is-open" : "is-closed"}`}>
          <InspectorPanel
            artworkUrl={currentArtworkUrl}
            currentTrack={currentTrack}
            layout={layout}
            libraryFolders={libraryFolders}
            p2pSettings={p2pSettings}
            p2pStatus={p2pStatus}
            scanSummary={scanSummary}
            scanJob={scanJob}
            backendStatus={backendStatus}
            diagnosticsPath={diagnosticsPath}
            scanning={scanning}
            playlistName={playlistName}
            previewMode={previewMode}
            trackEditorDraft={trackEditorDraft}
            onThemeChange={setTheme}
            onDensityChange={setDensity}
            onPreset={applyLayoutPreset}
            onToggleModule={toggleModule}
            onHideAll={hideAllModules}
            onShowAll={showAllModules}
            onShowCore={showCoreWorkspace}
            onReset={resetLayout}
            onImport={importFolders}
            onImportTracks={importTracks}
            onP2pSettingsChange={updateP2pSettings}
            onStartP2p={startSwarm}
            onStopP2p={stopSwarm}
            onRemoveLibraryFolder={removeFolder}
            onRescanLibraryFolder={rescanFolder}
            onPlaylistNameChange={setPlaylistName}
            onCreatePlaylist={createNewPlaylist}
            onPickArtwork={pickTrackArtwork}
            onSaveTrackDetails={saveTrackDetails}
            onTrackEditorChange={setTrackEditorDraft}
            onClose={() => setInspectorOpen(false)}
          />
        </div>
      </main>
    </div>
  );
}

interface WorkspaceEmptyProps {
  hiddenModules: ModuleId[];
  onImportTracks: () => void;
  onShowAll: () => void;
  onShowCore: () => void;
  onShowModule: (id: ModuleId) => void;
  onStudio: () => void;
}

function WorkspaceEmpty({
  hiddenModules,
  onImportTracks,
  onShowAll,
  onShowCore,
  onShowModule,
  onStudio,
}: WorkspaceEmptyProps) {
  return (
    <div className="workspace-empty">
      <div className="empty-kicker">0 блоков</div>
      <h2>Рабочее поле пустое</h2>
      <div className="empty-actions">
        <button className="import-btn" type="button" onClick={onShowAll}>
          Показать все
        </button>
        <button className="secondary-btn" type="button" onClick={onShowCore}>
          Рабочий минимум
        </button>
        <button className="secondary-btn" type="button" onClick={onStudio}>
          Studio
        </button>
        <button className="secondary-btn" type="button" onClick={onImportTracks}>
          Добавить треки
        </button>
      </div>
      {hiddenModules.length > 0 && (
        <div className="restore-grid" aria-label="Скрытые блоки">
          {hiddenModules.map((id) => {
            const Icon = moduleMeta[id].icon;
            return (
              <button className="restore-chip" type="button" key={id} onClick={() => onShowModule(id)}>
                <span aria-hidden="true"><Icon size={14} /></span>
                {moduleMeta[id].title}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

interface HiddenModuleDockProps {
  hiddenModules: ModuleId[];
  onShowAll: () => void;
  onShowModule: (id: ModuleId) => void;
}

function HiddenModuleDock({ hiddenModules, onShowAll, onShowModule }: HiddenModuleDockProps) {
  return (
    <div className="hidden-dock" aria-label="Скрытые блоки">
      <div className="hidden-dock-label">Скрыто: {hiddenModules.length}</div>
      <div className="hidden-dock-list">
        {hiddenModules.map((id) => {
          const Icon = moduleMeta[id].icon;
          return (
            <button className="restore-chip" type="button" key={id} onClick={() => onShowModule(id)}>
              <span aria-hidden="true"><Icon size={14} /></span>
              {moduleMeta[id].title}
            </button>
          );
        })}
      </div>
      <button className="secondary-btn" type="button" onClick={onShowAll}>
        Показать все
      </button>
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

function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function readStoredPlayback(): StoredPlayback {
  const fallback: StoredPlayback = {
    trackId: null,
    queueIds: [],
    volume: 0.72,
    shuffle: false,
    repeat: false,
  };

  const parsed = readJsonStorage<Partial<StoredPlayback>>(playbackStorageKey, fallback);
  return {
    trackId: typeof parsed.trackId === "number" ? parsed.trackId : null,
    queueIds: Array.isArray(parsed.queueIds)
      ? parsed.queueIds.filter((trackId): trackId is number => typeof trackId === "number")
      : [],
    volume: typeof parsed.volume === "number" ? clamp(parsed.volume, 0, 1) : fallback.volume,
    shuffle: Boolean(parsed.shuffle),
    repeat: Boolean(parsed.repeat),
  };
}

function scanSummaryFromJob(job: ScanJob): ScanSummary {
  return {
    scannedFiles: job.scannedFiles,
    added: job.added,
    updated: job.updated,
    skipped: job.skipped,
    errors: job.errors,
  };
}

function readError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }

  return "Неизвестная ошибка Fuse";
}

export default App;
