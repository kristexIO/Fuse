import { useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CollectionPanel, PlayerPanel, PlaylistsPanel, SwarmPanel } from "./Panels";
import type { Album, CollectionView, LibraryFolder, P2pStatus, Playlist, SharedItem, Track, TransferTask } from "../types";

const tracks: Track[] = [
  {
    id: 1,
    path: "C:/Music/signal.flac",
    title: "Signal Bloom",
    artist: "Northline Archive",
    album: "Late Focus",
    durationMs: 228000,
    format: "FLAC",
    sizeBytes: 1,
    modifiedAt: 1,
    missingTags: false,
    hasArtwork: false,
    dateAdded: 1,
    playCount: 0,
    isMissing: false,
  },
  {
    id: 2,
    path: "C:/Music/glass.wav",
    title: "Glass Relay",
    artist: "Paper Harbor",
    album: null,
    durationMs: 176000,
    format: "WAV",
    sizeBytes: 1,
    modifiedAt: 1,
    missingTags: true,
    hasArtwork: false,
    dateAdded: 1,
    playCount: 0,
    isMissing: false,
  },
];

const albums: Album[] = [
  { name: "Late Focus", artist: "Northline Archive", trackCount: 1 },
];

const folders: LibraryFolder[] = [
  { id: 1, path: "C:/Music/Northline Archive", addedAt: 1, lastScannedAt: 1 },
];

const playlists: Playlist[] = [
  {
    id: 1,
    name: "Late Focus",
    trackCount: 2,
    createdAt: 1,
    description: "Night tracks",
    artworkUri: null,
    updatedAt: 1,
    sortOrder: 1,
  },
];

function CollectionHarness() {
  const [view, setView] = useState<CollectionView>("tracks");

  return (
    <CollectionPanel
      tracks={tracks}
      albums={albums}
      libraryFolders={folders}
      view={view}
      activePlaylist={playlists[0]}
      activePlaylistTrackIds={new Set([1])}
      currentTrackId={1}
      onViewChange={setView}
      onPlayTrack={vi.fn()}
      onAddTrackToPlaylist={vi.fn()}
    />
  );
}

describe("CollectionPanel", () => {
  it("switches between tracks, albums, and folders", async () => {
    const user = userEvent.setup();
    render(<CollectionHarness />);

    expect(screen.getByText("Signal Bloom")).toBeTruthy();

    await user.click(screen.getByRole("tab", { name: "Альбомы" }));
    expect(screen.getByText("Late Focus")).toBeTruthy();
    expect(screen.getByText("Northline Archive")).toBeTruthy();

    await user.click(screen.getByRole("tab", { name: "Папки" }));
    expect(screen.getByText("C:/Music/Northline Archive")).toBeTruthy();
  });
});

describe("PlaylistsPanel", () => {
  it("emits rename, reorder, delete, and remove actions", async () => {
    const user = userEvent.setup();
    const onRenamePlaylist = vi.fn();
    const onMoveTrack = vi.fn();
    const onDeletePlaylist = vi.fn();
    const onRemoveTrack = vi.fn();

    render(
      <PlaylistsPanel
        artworkUrls={{}}
        playlists={playlists}
        activePlaylistId={1}
        activePlaylistTracks={tracks}
        onSelectPlaylist={vi.fn()}
        onRemoveTrack={onRemoveTrack}
        onDeletePlaylist={onDeletePlaylist}
        onMoveTrack={onMoveTrack}
        onPlayPlaylist={vi.fn()}
        onRenamePlaylist={onRenamePlaylist}
      />,
    );

    await user.clear(screen.getByLabelText("Название плейлиста"));
    await user.type(screen.getByLabelText("Название плейлиста"), "Focus Updated");
    await user.click(screen.getByRole("button", { name: "Сохранить" }));
    expect(onRenamePlaylist).toHaveBeenCalledWith("Focus Updated", "Night tracks");

    await user.click(screen.getAllByTitle("Ниже")[0]);
    expect(onMoveTrack).toHaveBeenCalledWith(1, 1);

    await user.click(screen.getAllByTitle("Убрать из плейлиста")[0]);
    expect(onRemoveTrack).toHaveBeenCalledWith(1);

    await user.click(screen.getByTitle("Удалить плейлист"));
    expect(onDeletePlaylist).toHaveBeenCalled();
  });
});

describe("PlayerPanel", () => {
  it("routes playback controls through callbacks", async () => {
    const user = userEvent.setup();
    const onNext = vi.fn();
    const onStop = vi.fn();

    render(
      <PlayerPanel
        artworkUrl={null}
        currentTrack={tracks[0]}
        currentTimeMs={0}
        durationMs={228000}
        isPlaying={false}
        playbackError={null}
        playbackBackend="webview"
        repeat={false}
        shuffle={false}
        volume={0.72}
        onNext={onNext}
        onPrevious={vi.fn()}
        onSeek={vi.fn()}
        onStop={onStop}
        onTogglePlay={vi.fn()}
        onToggleRepeat={vi.fn()}
        onToggleShuffle={vi.fn()}
        onVolumeChange={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Следующий трек" }));
    await user.click(screen.getByRole("button", { name: "Остановить" }));

    expect(onNext).toHaveBeenCalled();
    expect(onStop).toHaveBeenCalled();
  });
});

describe("SwarmPanel", () => {
  it("routes share, copy, and download actions through callbacks", async () => {
    const user = userEvent.setup();
    const onShareTrack = vi.fn();
    const onCopyTicket = vi.fn();
    const onDownloadTicket = vi.fn();
    const p2pStatus: P2pStatus = {
      enabled: true,
      running: true,
      nodeId: "preview-node",
      nodeAddr: null,
      activeShares: 1,
      activeDownloads: 0,
      autoSeedDownloads: true,
      importDir: null,
      uploadLimitKbps: null,
      downloadLimitKbps: null,
      lastError: null,
    };
    const shares: SharedItem[] = [
      {
        id: 1,
        scope: "track",
        trackId: 1,
        playlistId: null,
        title: "Signal Bloom",
        artist: "Northline Archive",
        album: "Late Focus",
        manifestHash: "hash",
        swarmTopic: "topic",
        sizeBytes: 1024,
        itemCount: 1,
        ticket: "fuse-share:v1:test",
        state: "active",
        createdAt: 1,
        updatedAt: 1,
        revokedAt: null,
      },
    ];
    const transfers: TransferTask[] = [];

    render(
      <SwarmPanel
        p2pStatus={p2pStatus}
        shares={shares}
        transfers={transfers}
        ticketDraft="fuse-share:v1:test"
        previewMode={false}
        currentTrack={tracks[0]}
        activePlaylist={playlists[0]}
        onTicketDraftChange={vi.fn()}
        onStartP2p={vi.fn()}
        onStopP2p={vi.fn()}
        onShareTrack={onShareTrack}
        onSharePlaylist={vi.fn()}
        onCopyTicket={onCopyTicket}
        onDownloadTicket={onDownloadTicket}
        onPauseShare={vi.fn()}
        onResumeShare={vi.fn()}
        onRevokeShare={vi.fn()}
        onCancelTransfer={vi.fn()}
        onRetryTransfer={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Трек" }));
    await user.click(screen.getByTitle("Скопировать ticket"));
    await user.click(screen.getByTitle("Скачать по ticket"));

    expect(onShareTrack).toHaveBeenCalled();
    expect(onCopyTicket).toHaveBeenCalledWith("fuse-share:v1:test");
    expect(onDownloadTicket).toHaveBeenCalled();
  });
});
