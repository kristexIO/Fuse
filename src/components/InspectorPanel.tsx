import type { CSSProperties } from "react";
import type { Density, LayoutProfile, LibraryFolder, ModuleId, ScanJob, ScanSummary, ThemeName, Track } from "../types";
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
  scanSummary: ScanSummary | null;
  scanJob: ScanJob | null;
  backendStatus: string;
  diagnosticsPath: string | null;
  scanning: boolean;
  playlistName: string;
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
  onPlaylistNameChange: (name: string) => void;
  onCreatePlaylist: () => void;
  onPickArtwork: () => void;
  onSaveTrackDetails: () => void;
  onTrackEditorChange: (draft: TrackEditorDraft) => void;
}

const themes: Array<{ id: ThemeName; label: string; a: string; b: string }> = [
  { id: "obsidian", label: "Obsidian", a: "#ff715b", b: "#4fd8c6" },
  { id: "porcelain", label: "Porcelain", a: "#f06449", b: "#1aa89b" },
  { id: "oled", label: "OLED", a: "#f0f0f0", b: "#50e3c2" },
  { id: "boreal", label: "Boreal", a: "#67e0a3", b: "#57c7ff" },
  { id: "ember", label: "Ember", a: "#ff7a45", b: "#f3557c" },
  { id: "violet", label: "Violet", a: "#a98bff", b: "#39d6d6" },
  { id: "rose", label: "Rose", a: "#ff86a5", b: "#d1a55d" },
];

const moduleLabels: Record<ModuleId, string> = {
  library: "Медиатека",
  now: "Сейчас играет",
  collection: "Коллекция",
  player: "Плеер",
  queue: "Очередь",
  mixer: "Микшер",
  playlists: "Плейлисты",
  stats: "Сводка",
};

export function InspectorPanel({
  artworkUrl,
  currentTrack,
  layout,
  libraryFolders,
  scanSummary,
  scanJob,
  backendStatus,
  diagnosticsPath,
  scanning,
  playlistName,
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
  onPlaylistNameChange,
  onCreatePlaylist,
  onPickArtwork,
  onSaveTrackDetails,
  onTrackEditorChange,
}: InspectorPanelProps) {
  const visibleCount = layout.order.length - layout.hidden.length;
  const hiddenCount = layout.hidden.length;

  return (
    <aside className="inspector" aria-label="Настройка дизайна">
      <div className="inspector-header">
        <h2>Кастомизация</h2>
        <span>{backendStatus}</span>
      </div>

      <section className="inspector-section">
        <div className="section-label">Медиатека</div>
        <div className="action-row">
          <button className="import-btn" type="button" onClick={onImportTracks} disabled={scanning}>
            {scanning ? "Добавление..." : "Добавить треки"}
          </button>
          <button className="secondary-btn" type="button" onClick={onImport} disabled={scanning}>
            Папка
          </button>
        </div>
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
            {scanSummary.scannedFiles} scanned / {scanSummary.added} added / {scanSummary.updated} updated
          </p>
        )}
        {scanJob && (
          <p className="status-line">
            scan #{scanJob.id} / {scanJob.state} / {scanJob.skipped} skipped
          </p>
        )}
        {libraryFolders.length > 0 && (
          <div className="folder-list">
            {libraryFolders.slice(0, 4).map((folder) => (
              <span key={folder.id} title={folder.path}>
                {folder.path}
              </span>
            ))}
          </div>
        )}
        {diagnosticsPath && (
          <p className="status-line" title={diagnosticsPath}>
            diagnostics: {compactPath(diagnosticsPath)}
          </p>
        )}
      </section>

      {currentTrack && (
        <section className="inspector-section track-editor">
          <div className="section-label">Трек</div>
          <div className="track-editor-cover">
            <div
              className={`editor-cover ${artworkUrl ? "has-artwork" : ""}`}
              style={artworkUrl ? ({ backgroundImage: `url("${artworkUrl}")` } as CSSProperties) : undefined}
            />
            <button className="secondary-btn" type="button" onClick={onPickArtwork}>
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
            placeholder="Текст / lyrics"
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
            Показать всё
          </button>
          <button className="secondary-btn" type="button" onClick={onHideAll} disabled={visibleCount === 0}>
            Скрыть всё
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
                  <small>{block.cols}×{block.rows}</small>
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
