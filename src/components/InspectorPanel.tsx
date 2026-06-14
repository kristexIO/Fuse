import type { CSSProperties } from "react";
import { FolderPlus, Music, RefreshCcw, Trash2, Wifi, WifiOff, X } from "lucide-react";
import type { Density, LayoutProfile, LibraryFolder, ModuleId, P2pSettings, P2pStatus, ScanJob, ScanSummary, ThemeName, Track } from "../types";
import { getBlock, layoutPresets } from "../lib/layout";

interface TrackEditorDraft {
  title: string;
  artist: string;
  album: string;
  lyrics: string;
}

interface InspectorPanelProps {
  artworkUrl: string | null;
  currentTrack: Track | null;
  layout: LayoutProfile;
  libraryFolders: LibraryFolder[];
  p2pSettings: P2pSettings;
  p2pStatus: P2pStatus | null;
  scanSummary: ScanSummary | null;
  scanJob: ScanJob | null;
  backendStatus: string;
  diagnosticsPath: string | null;
  scanning: boolean;
  playlistName: string;
  previewMode: boolean;
  trackEditorDraft: TrackEditorDraft;
  onThemeChange: (theme: ThemeName) => void;
  onDensityChange: (density: Density) => void;
  onPreset: (name: string) => void;
  onToggleModule: (id: ModuleId) => void;
  onHideAll: () => void;
  onShowAll: () => void;
  onShowCore: () => void;
  onReset: () => void;
  onImport: () => void;
  onImportTracks: () => void;
  onP2pSettingsChange: (settings: P2pSettings) => void;
  onStartP2p: () => void;
  onStopP2p: () => void;
  onRemoveLibraryFolder: (folderId: number) => void;
  onRescanLibraryFolder: (path: string) => void;
  onPlaylistNameChange: (name: string) => void;
  onCreatePlaylist: () => void;
  onPickArtwork: () => void;
  onSaveTrackDetails: () => void;
  onTrackEditorChange: (draft: TrackEditorDraft) => void;
  onClose: () => void;
}

const themes: Array<{ id: ThemeName; label: string; a: string; b: string }> = [
  { id: "obsidian", label: "Obsidian", a: "#ff715b", b: "#4fd8c6" },
  { id: "porcelain", label: "Porcelain", a: "#f06449", b: "#1aa89b" },
  { id: "oled", label: "OLED", a: "#f0f0f0", b: "#50e3c2" },
  { id: "boreal", label: "Boreal", a: "#67e0a3", b: "#57c7ff" },
  { id: "ember", label: "Ember", a: "#ff7a45", b: "#f3557c" },
  { id: "violet", label: "Violet", a: "#a98bff", b: "#39d6d6" },
  { id: "rose", label: "Rose", a: "#ff86a5", b: "#d1a55d" },
  { id: "graphite", label: "Graphite", a: "#d8ff5f", b: "#ff7c6e" },
  { id: "lagoon", label: "Lagoon", a: "#4ff0c5", b: "#f2d46f" },
  { id: "daybreak", label: "Daybreak", a: "#ef6351", b: "#2776d8" },
];

const moduleLabels: Record<ModuleId, string> = {
  library: "Медиатека",
  now: "Сейчас играет",
  collection: "Коллекция",
  player: "Плеер",
  queue: "Очередь",
  mixer: "Форматы",
  swarm: "Swarm",
  playlists: "Плейлисты",
  stats: "Сводка",
};

export function InspectorPanel({
  artworkUrl,
  currentTrack,
  layout,
  libraryFolders,
  p2pSettings,
  p2pStatus,
  scanSummary,
  scanJob,
  backendStatus,
  diagnosticsPath,
  scanning,
  playlistName,
  previewMode,
  trackEditorDraft,
  onThemeChange,
  onDensityChange,
  onPreset,
  onToggleModule,
  onHideAll,
  onShowAll,
  onShowCore,
  onReset,
  onImport,
  onImportTracks,
  onP2pSettingsChange,
  onStartP2p,
  onStopP2p,
  onRemoveLibraryFolder,
  onRescanLibraryFolder,
  onPlaylistNameChange,
  onCreatePlaylist,
  onPickArtwork,
  onSaveTrackDetails,
  onTrackEditorChange,
  onClose,
}: InspectorPanelProps) {
  const visibleCount = layout.order.length - layout.hidden.length;
  const hiddenCount = layout.hidden.length;
  const desktopOnlyTitle = previewMode
    ? "Доступно в настольной версии Fuse"
    : undefined;

  return (
    <aside className="inspector" aria-label="Настройка Fuse">
      <div className="inspector-header">
        <div>
          <h2>Настройки</h2>
          <span>{backendStatus}</span>
        </div>
        <button className="icon-btn inspector-close" type="button" title="Скрыть панель" onClick={onClose}>
          <X size={16} aria-hidden="true" />
        </button>
      </div>

      <section className="inspector-section">
        <div className="section-label">Медиатека</div>
        <div className="action-row">
          <button
            className="import-btn"
            type="button"
            onClick={onImportTracks}
            disabled={scanning || previewMode}
            title={desktopOnlyTitle}
          >
            <Music size={16} aria-hidden="true" />
            {scanning ? "Добавление..." : "Добавить треки"}
          </button>
          <button
            className="secondary-btn"
            type="button"
            onClick={onImport}
            disabled={scanning || previewMode}
            title={desktopOnlyTitle}
          >
            <FolderPlus size={16} aria-hidden="true" />
            Папка
          </button>
        </div>
        {previewMode && (
          <p className="status-line">Веб-превью работает с демо-данными. Импорт файлов доступен в Tauri-приложении.</p>
        )}
        <form
          className="playlist-create"
          onSubmit={(event) => {
            event.preventDefault();
            onCreatePlaylist();
          }}
        >
          <input
            type="text"
            placeholder="Новый плейлист"
            value={playlistName}
            onChange={(event) => onPlaylistNameChange(event.currentTarget.value)}
          />
          <button className="secondary-btn" type="submit">
            Создать
          </button>
        </form>
        {scanSummary && (
          <p className="status-line">
            Просканировано: {scanSummary.scannedFiles}, добавлено: {scanSummary.added}, обновлено: {scanSummary.updated}
          </p>
        )}
        {scanJob && (
          <p className="status-line">
            Сканирование #{scanJob.id}: {scanStateLabel(scanJob.state)}, пропущено: {scanJob.skipped}
          </p>
        )}
        {libraryFolders.length > 0 && (
          <div className="folder-list">
            {libraryFolders.slice(0, 6).map((folder) => (
              <div className="folder-row" key={folder.id} title={folder.path}>
                <span>{folder.path}</span>
                <button
                  className="track-action"
                  type="button"
                  title="Пересканировать папку"
                  onClick={() => onRescanLibraryFolder(folder.path)}
                >
                  <RefreshCcw size={13} aria-hidden="true" />
                </button>
                <button
                  className="track-action danger"
                  type="button"
                  title="Удалить папку из библиотеки"
                  onClick={() => onRemoveLibraryFolder(folder.id)}
                >
                  <Trash2 size={13} aria-hidden="true" />
                </button>
              </div>
            ))}
          </div>
        )}
        {diagnosticsPath && (
          <p className="status-line" title={diagnosticsPath}>
            Диагностика: {compactPath(diagnosticsPath)}
          </p>
        )}
      </section>

      <section className="inspector-section">
        <div className="section-head">
          <div className="section-label">Swarm</div>
          <span>{p2pStatus?.running ? "online" : "offline"}</span>
        </div>
        <div className="action-row">
          <button
            className={p2pStatus?.running ? "secondary-btn" : "import-btn"}
            type="button"
            onClick={p2pStatus?.running ? onStopP2p : onStartP2p}
            disabled={previewMode}
            title={desktopOnlyTitle}
          >
            {p2pStatus?.running ? <WifiOff size={16} aria-hidden="true" /> : <Wifi size={16} aria-hidden="true" />}
            {p2pStatus?.running ? "Отключить" : "Включить"}
          </button>
          <label className="toggle inline-toggle">
            <span>
              <strong>Auto-seed</strong>
              <small>после загрузки</small>
            </span>
            <input
              type="checkbox"
              checked={p2pSettings.autoSeedDownloads}
              onChange={(event) => onP2pSettingsChange({ ...p2pSettings, autoSeedDownloads: event.currentTarget.checked })}
            />
          </label>
        </div>
        <input
          type="text"
          value={p2pSettings.importDir ?? ""}
          onChange={(event) => onP2pSettingsChange({ ...p2pSettings, importDir: event.currentTarget.value || null })}
          placeholder="Папка импорта Swarm"
          disabled={previewMode}
          title={desktopOnlyTitle}
        />
        <div className="limit-grid">
          <input
            type="number"
            min="0"
            value={p2pSettings.downloadLimitKbps ?? ""}
            onChange={(event) => onP2pSettingsChange({ ...p2pSettings, downloadLimitKbps: readOptionalNumber(event.currentTarget.value) })}
            placeholder="Download KB/s"
          />
          <input
            type="number"
            min="0"
            value={p2pSettings.uploadLimitKbps ?? ""}
            onChange={(event) => onP2pSettingsChange({ ...p2pSettings, uploadLimitKbps: readOptionalNumber(event.currentTarget.value) })}
            placeholder="Upload KB/s"
          />
        </div>
        <p className="status-line">
          {p2pStatus?.activeShares ?? 0} раздач · {p2pStatus?.activeDownloads ?? 0} загрузок
        </p>
        {p2pStatus?.lastError && <p className="status-line">{p2pStatus.lastError}</p>}
      </section>

      {currentTrack && (
        <section className="inspector-section track-editor">
          <div className="section-label">Трек</div>
          <div className="track-editor-cover">
            <div
              className={`editor-cover ${artworkUrl ? "has-artwork" : ""}`}
              style={artworkUrl ? ({ backgroundImage: `url("${artworkUrl}")` } as CSSProperties) : undefined}
            />
            <button className="secondary-btn" type="button" onClick={onPickArtwork} disabled={previewMode} title={desktopOnlyTitle}>
              Обложка
            </button>
          </div>
          <input
            type="text"
            value={trackEditorDraft.title}
            onChange={(event) => onTrackEditorChange({ ...trackEditorDraft, title: event.currentTarget.value })}
            placeholder="Название"
          />
          <input
            type="text"
            value={trackEditorDraft.artist}
            onChange={(event) => onTrackEditorChange({ ...trackEditorDraft, artist: event.currentTarget.value })}
            placeholder="Артист"
          />
          <input
            type="text"
            value={trackEditorDraft.album}
            onChange={(event) => onTrackEditorChange({ ...trackEditorDraft, album: event.currentTarget.value })}
            placeholder="Альбом"
          />
          <textarea
            value={trackEditorDraft.lyrics}
            onChange={(event) => onTrackEditorChange({ ...trackEditorDraft, lyrics: event.currentTarget.value })}
            placeholder="Текст песни или LRC"
            rows={5}
          />
          <button className="import-btn" type="button" onClick={onSaveTrackDetails}>
            Сохранить трек
          </button>
        </section>
      )}

      <section className="inspector-section">
        <div className="section-label">Темы</div>
        <div className="theme-grid">
          {themes.map((theme) => (
            <button
              className={`theme-btn ${layout.theme === theme.id ? "is-active" : ""}`}
              key={theme.id}
              type="button"
              onClick={() => onThemeChange(theme.id)}
            >
              <span
                className="swatch"
                style={{ "--sw-a": theme.a, "--sw-b": theme.b } as CSSProperties}
              />
              <span>{theme.label}</span>
            </button>
          ))}
        </div>
      </section>

      <section className="inspector-section">
        <div className="section-label">Пресеты</div>
        <div className="preset-grid">
          {Object.keys(layoutPresets).map((preset) => (
            <button className="preset-btn" type="button" key={preset} onClick={() => onPreset(preset)}>
              {preset}
            </button>
          ))}
        </div>
      </section>

      <section className="inspector-section">
        <div className="section-label">Плотность</div>
        <div className="segmented" role="group" aria-label="Плотность интерфейса">
          {(["compact", "comfortable", "spacious"] as Density[]).map((density) => (
            <button
              className={layout.density === density ? "is-active" : ""}
              type="button"
              key={density}
              onClick={() => onDensityChange(density)}
            >
              {density === "compact" ? "Плотно" : density === "comfortable" ? "Нормально" : "Свободно"}
            </button>
          ))}
        </div>
      </section>

      <section className="inspector-section">
        <div className="section-head">
          <div className="section-label">Блоки</div>
          <span>{visibleCount} / {layout.order.length}</span>
        </div>
        <div className="layout-action-grid">
          <button className="secondary-btn" type="button" onClick={onShowAll} disabled={hiddenCount === 0}>
            Показать все
          </button>
          <button className="secondary-btn" type="button" onClick={onHideAll} disabled={visibleCount === 0}>
            Скрыть все
          </button>
          <button className="secondary-btn" type="button" onClick={onShowCore}>
            Рабочий минимум
          </button>
          <button className="reset-btn" type="button" onClick={onReset}>
            Сбросить
          </button>
        </div>
        <div className="toggle-list">
          {layout.order.map((id) => {
            const block = getBlock(layout, id);
            const hidden = layout.hidden.includes(id);

            return (
              <label className={`toggle module-toggle-row ${hidden ? "is-hidden" : "is-visible"}`} key={id}>
                <span>
                  <strong>{moduleLabels[id]}</strong>
                  <small>{block.cols}x{block.rows}</small>
                </span>
                <em>{hidden ? "скрыт" : "виден"}</em>
                <input
                  type="checkbox"
                  checked={!hidden}
                  onChange={() => onToggleModule(id)}
                />
              </label>
            );
          })}
        </div>
      </section>
    </aside>
  );
}

function compactPath(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);

  if (parts.length <= 3) {
    return path;
  }

  return `.../${parts.slice(-3).join("/")}`;
}

function scanStateLabel(state: string): string {
  if (state === "completed") {
    return "завершено";
  }

  if (state === "completed_with_errors") {
    return "завершено с ошибками";
  }

  if (state === "running") {
    return "выполняется";
  }

  if (state === "cancelled") {
    return "отменено";
  }

  return state;
}

function readOptionalNumber(value: string): number | null {
  if (!value.trim()) {
    return null;
  }

  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed > 0 ? Math.round(parsed) : null;
}
