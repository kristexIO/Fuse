import type { Density, LayoutBlock, LayoutProfile, ModuleId, ThemeName } from "../types";

export const moduleOrder: ModuleId[] = [
  "library",
  "now",
  "collection",
  "player",
  "queue",
  "mixer",
  "swarm",
  "playlists",
  "stats",
];

export const defaultBlocks: Record<ModuleId, LayoutBlock> = {
  library: { id: "library", cols: 3, rows: 1 },
  now: { id: "now", cols: 3, rows: 2 },
  collection: { id: "collection", cols: 6, rows: 1 },
  player: { id: "player", cols: 6, rows: 1 },
  queue: { id: "queue", cols: 3, rows: 1 },
  mixer: { id: "mixer", cols: 3, rows: 1 },
  swarm: { id: "swarm", cols: 6, rows: 1 },
  playlists: { id: "playlists", cols: 6, rows: 1 },
  stats: { id: "stats", cols: 6, rows: 1 },
};

export const defaultLayout: LayoutProfile = {
  name: "Studio",
  theme: "obsidian",
  density: "comfortable",
  order: moduleOrder,
  hidden: [],
  blocks: Object.values(defaultBlocks),
};

export const layoutPresets: Record<string, Pick<LayoutProfile, "order" | "hidden" | "blocks">> = {
  Studio: {
    order: ["now", "player", "queue", "mixer", "swarm", "collection", "library", "playlists", "stats"],
    hidden: [],
    blocks: [
      { id: "now", cols: 3, rows: 2 },
      { id: "player", cols: 6, rows: 1 },
      { id: "queue", cols: 3, rows: 1 },
      { id: "mixer", cols: 3, rows: 1 },
      { id: "swarm", cols: 6, rows: 1 },
      { id: "collection", cols: 6, rows: 1 },
      { id: "library", cols: 3, rows: 1 },
      { id: "playlists", cols: 6, rows: 1 },
      { id: "stats", cols: 6, rows: 1 },
    ],
  },
  Library: {
    order: ["library", "collection", "playlists", "stats", "swarm", "player", "now", "queue", "mixer"],
    hidden: [],
    blocks: [
      { id: "library", cols: 3, rows: 1 },
      { id: "collection", cols: 6, rows: 2 },
      { id: "playlists", cols: 6, rows: 1 },
      { id: "stats", cols: 6, rows: 1 },
      { id: "swarm", cols: 6, rows: 1 },
      { id: "player", cols: 6, rows: 1 },
      { id: "now", cols: 3, rows: 2 },
      { id: "queue", cols: 3, rows: 1 },
      { id: "mixer", cols: 3, rows: 1 },
    ],
  },
  Minimal: {
    order: ["now", "player", "collection", "queue", "swarm", "library", "mixer", "playlists", "stats"],
    hidden: ["library", "mixer", "swarm", "playlists", "stats"],
    blocks: [
      { id: "now", cols: 6, rows: 2 },
      { id: "player", cols: 6, rows: 1 },
      { id: "collection", cols: 6, rows: 1 },
      { id: "queue", cols: 3, rows: 1 },
      { id: "swarm", cols: 6, rows: 1 },
      { id: "library", cols: 3, rows: 1 },
      { id: "mixer", cols: 3, rows: 1 },
      { id: "playlists", cols: 6, rows: 1 },
      { id: "stats", cols: 6, rows: 1 },
    ],
  },
  Showcase: {
    order: ["now", "player", "playlists", "collection", "swarm", "queue", "stats", "library", "mixer"],
    hidden: [],
    blocks: [
      { id: "now", cols: 4, rows: 2 },
      { id: "player", cols: 8, rows: 1 },
      { id: "playlists", cols: 8, rows: 2 },
      { id: "collection", cols: 4, rows: 2 },
      { id: "swarm", cols: 8, rows: 1 },
      { id: "queue", cols: 4, rows: 1 },
      { id: "stats", cols: 4, rows: 1 },
      { id: "library", cols: 4, rows: 1 },
      { id: "mixer", cols: 4, rows: 1 },
    ],
  },
  Playlist: {
    order: ["playlists", "player", "queue", "collection", "swarm", "now", "library", "stats", "mixer"],
    hidden: ["mixer"],
    blocks: [
      { id: "playlists", cols: 6, rows: 2 },
      { id: "player", cols: 6, rows: 1 },
      { id: "queue", cols: 3, rows: 2 },
      { id: "collection", cols: 6, rows: 2 },
      { id: "swarm", cols: 6, rows: 1 },
      { id: "now", cols: 3, rows: 2 },
      { id: "library", cols: 3, rows: 1 },
      { id: "stats", cols: 6, rows: 1 },
      { id: "mixer", cols: 3, rows: 1 },
    ],
  },
};

export function normalizeLayout(layout: Partial<LayoutProfile> | null | undefined): LayoutProfile {
  const theme = isTheme(layout?.theme) ? layout.theme : defaultLayout.theme;
  const density = isDensity(layout?.density) ? layout.density : defaultLayout.density;
  const order = uniqueModuleOrder(layout?.order);
  const hidden = uniqueModules(layout?.hidden);
  const blocksById = new Map(defaultLayout.blocks.map((block) => [block.id, block]));

  layout?.blocks?.forEach((block) => {
    if (isModuleId(block.id)) {
      blocksById.set(block.id, {
        id: block.id,
        cols: clamp(Math.round(block.cols), 2, 12),
        rows: clamp(Math.round(block.rows), 1, 4),
      });
    }
  });

  return {
    name: layout?.name || defaultLayout.name,
    theme,
    density,
    order,
    hidden,
    blocks: moduleOrder.map((id) => blocksById.get(id) || defaultBlocks[id]),
  };
}

export function getBlock(layout: LayoutProfile, id: ModuleId): LayoutBlock {
  return layout.blocks.find((block) => block.id === id) || defaultBlocks[id];
}

export function updateBlock(layout: LayoutProfile, id: ModuleId, patch: Partial<LayoutBlock>): LayoutProfile {
  return normalizeLayout({
    ...layout,
    blocks: layout.blocks.map((block) =>
      block.id === id ? { ...block, ...patch, id } : block,
    ),
  });
}

export function applyPreset(layout: LayoutProfile, presetName: string): LayoutProfile {
  const preset = layoutPresets[presetName] || layoutPresets.Studio;
  return normalizeLayout({
    ...layout,
    name: presetName,
    order: preset.order,
    hidden: preset.hidden,
    blocks: preset.blocks,
  });
}

function uniqueModuleOrder(value: unknown): ModuleId[] {
  const seen = new Set<ModuleId>();
  const order = Array.isArray(value)
    ? value.filter((item): item is ModuleId => isModuleId(item))
    : [];

  const result = [...order, ...moduleOrder].filter((item) => {
    if (seen.has(item)) {
      return false;
    }
    seen.add(item);
    return true;
  });

  return result;
}

function uniqueModules(value: unknown): ModuleId[] {
  const seen = new Set<ModuleId>();

  if (!Array.isArray(value)) {
    return [];
  }

  return value.filter((item): item is ModuleId => {
    if (!isModuleId(item) || seen.has(item)) {
      return false;
    }
    seen.add(item);
    return true;
  });
}

function isModuleId(value: unknown): value is ModuleId {
  return typeof value === "string" && moduleOrder.includes(value as ModuleId);
}

function isTheme(value: unknown): value is ThemeName {
  return (
    value === "obsidian" ||
    value === "porcelain" ||
    value === "oled" ||
    value === "boreal" ||
    value === "ember" ||
    value === "violet" ||
    value === "rose" ||
    value === "graphite" ||
    value === "lagoon" ||
    value === "daybreak"
  );
}

function isDensity(value: unknown): value is Density {
  return value === "compact" || value === "comfortable" || value === "spacious";
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
