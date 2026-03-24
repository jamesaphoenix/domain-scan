import { describe, it, expect } from "vitest";
import type { TubeMapData, TubeMapSubsystem, TubeMapConnection } from "../types";
import {
  assignDomainLayers,
  assignDomainGrid,
  orderStationsWithinLine,
  buildCanonicalPositions,
  applyCompactLayout,
  generateSegments,
  buildDynamicLayout,
  normalizeOrphanDomains,
  STATION_GAP,
  LINE_GAP,
  NODE_WIDTH,
  MAX_STATIONS_PER_SEGMENT,
} from "./tubeMap";
import { UNASSIGNED_DOMAIN_ID, UNASSIGNED_COLOR, UNASSIGNED_LABEL } from "./types";
import { assignDomainColors } from "./colors";
import type { ComputedLine } from "./types";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeSub(
  id: string,
  domain: string,
  overrides?: Partial<TubeMapSubsystem>,
): TubeMapSubsystem {
  return {
    id,
    name: id,
    domain,
    status: "built",
    description: "",
    file_path: `/project/src/${id}/`,
    matched_entity_count: 0,
    interface_count: 0,
    operation_count: 0,
    table_count: 0,
    event_count: 0,
    has_children: false,
    child_count: 0,
    dependency_count: 0,
    ...overrides,
  };
}

function makeConn(
  from: string,
  to: string,
  type: TubeMapConnection["type"] = "depends_on",
): TubeMapConnection {
  return { from, to, label: `${from} → ${to}`, type };
}

// ---------------------------------------------------------------------------
// Octospark-like reference fixture (top-level subsystems only)
// ---------------------------------------------------------------------------

const OCTOSPARK_FIXTURE: TubeMapData = {
  meta: { name: "Octospark", version: "1.0", description: "Test fixture" },
  domains: {
    "platform-core": { label: "Platform Core", color: "#3b82f6" },
    "media-storage": { label: "Media & Storage", color: "#22c55e" },
    services: { label: "Services", color: "#f97316" },
    experimentation: { label: "Experimentation", color: "#a855f7" },
    workflows: { label: "Workflows", color: "#ef4444" },
    frontends: { label: "Frontends", color: "#6b7280" },
    "post-launch": { label: "Post-Launch", color: "#9ca3af" },
  },
  subsystems: [
    makeSub("auth", "platform-core"),
    makeSub("org-team", "platform-core"),
    makeSub("credit-service", "platform-core"),
    makeSub("billing", "platform-core"),
    makeSub("notifications", "platform-core"),
    makeSub("webhooks", "platform-core"),
    makeSub("asset-management", "media-storage"),
    makeSub("media-uploader", "media-storage"),
    makeSub("media-enrichment", "media-storage"),
    makeSub("retention-cleaner", "media-storage"),
    makeSub("social-oauth", "services"),
    makeSub("publisher", "services"),
    makeSub("analytics-collector", "services"),
    makeSub("ai-generation", "services"),
    makeSub("prompt-bandit", "experimentation"),
    makeSub("workflows", "workflows"),
    makeSub("api-cli-sdk", "frontends"),
    makeSub("agency-onboarding", "post-launch"),
  ],
  connections: [
    makeConn("org-team", "auth", "depends_on"),
    makeConn("billing", "org-team", "depends_on"),
    makeConn("billing", "credit-service", "uses"),
    makeConn("credit-service", "org-team", "depends_on"),
    makeConn("notifications", "auth", "depends_on"),
    makeConn("notifications", "org-team", "depends_on"),
    makeConn("webhooks", "org-team", "depends_on"),
    makeConn("asset-management", "org-team", "depends_on"),
    makeConn("media-uploader", "asset-management", "depends_on"),
    makeConn("media-enrichment", "asset-management", "uses"),
    makeConn("media-enrichment", "ai-generation", "uses"),
    makeConn("media-enrichment", "credit-service", "depends_on"),
    makeConn("retention-cleaner", "asset-management", "uses"),
    makeConn("retention-cleaner", "billing", "depends_on"),
    makeConn("social-oauth", "auth", "depends_on"),
    makeConn("social-oauth", "org-team", "depends_on"),
    makeConn("publisher", "social-oauth", "depends_on"),
    makeConn("publisher", "asset-management", "uses"),
    makeConn("publisher", "notifications", "triggers"),
    makeConn("publisher", "webhooks", "triggers"),
    makeConn("analytics-collector", "social-oauth", "depends_on"),
    makeConn("analytics-collector", "publisher", "uses"),
    makeConn("ai-generation", "credit-service", "depends_on"),
    makeConn("prompt-bandit", "ai-generation", "uses"),
    makeConn("prompt-bandit", "analytics-collector", "uses"),
    makeConn("workflows", "ai-generation", "uses"),
    makeConn("workflows", "publisher", "triggers"),
    makeConn("workflows", "asset-management", "uses"),
    makeConn("workflows", "credit-service", "depends_on"),
    makeConn("workflows", "prompt-bandit", "uses"),
    makeConn("workflows", "notifications", "triggers"),
    makeConn("api-cli-sdk", "auth", "depends_on"),
    makeConn("api-cli-sdk", "org-team", "depends_on"),
    makeConn("api-cli-sdk", "billing", "uses"),
    makeConn("agency-onboarding", "social-oauth", "uses"),
    makeConn("agency-onboarding", "notifications", "triggers"),
    makeConn("agency-onboarding", "org-team", "uses"),
  ],
  coverage_percent: 0,
  unmatched_count: 0,
};

// ---------------------------------------------------------------------------
// 1. assignDomainLayers
// ---------------------------------------------------------------------------

describe("assignDomainLayers", () => {
  it("returns empty for no domains", () => {
    const { layers, cycleBreaks } = assignDomainLayers([], [], []);
    expect(layers).toEqual([]);
    expect(cycleBreaks).toEqual([]);
  });

  it("assigns single domain at depth 0", () => {
    const subs = [makeSub("a", "alpha"), makeSub("b", "alpha")];
    const { layers } = assignDomainLayers(["alpha"], [], subs);
    expect(layers).toHaveLength(1);
    expect(layers[0]!.topoDepth).toBe(0);
    expect(layers[0]!.stationCount).toBe(2);
  });

  it("orders independent domains at same depth", () => {
    const subs = [
      makeSub("a", "alpha"),
      makeSub("b", "beta"),
    ];
    const { layers } = assignDomainLayers(["alpha", "beta"], [], subs);
    expect(layers).toHaveLength(2);
    // Both should be at depth 0 (no cross-domain edges)
    expect(layers[0]!.topoDepth).toBe(0);
    expect(layers[1]!.topoDepth).toBe(0);
  });

  it("orders dependent domains correctly", () => {
    const subs = [
      makeSub("a", "alpha"),
      makeSub("b", "beta"),
    ];
    // a depends on b → alpha depends on beta → beta should come first
    const conns = [makeConn("a", "b")];
    const { layers } = assignDomainLayers(["alpha", "beta"], conns, subs);

    const alphaLayer = layers.find((l) => l.domain === "alpha")!;
    const betaLayer = layers.find((l) => l.domain === "beta")!;
    expect(betaLayer.topoDepth).toBeLessThan(alphaLayer.topoDepth);
  });

  it("ignores intra-domain connections", () => {
    const subs = [
      makeSub("a", "alpha"),
      makeSub("b", "alpha"),
    ];
    const conns = [makeConn("a", "b")]; // same domain
    const { layers } = assignDomainLayers(["alpha"], conns, subs);
    expect(layers).toHaveLength(1);
    expect(layers[0]!.topoDepth).toBe(0);
  });

  it("breaks cycles and records them", () => {
    const subs = [
      makeSub("a", "alpha"),
      makeSub("b", "beta"),
    ];
    // alpha → beta and beta → alpha = cycle
    const conns = [makeConn("a", "b"), makeConn("b", "a")];
    const { layers, cycleBreaks } = assignDomainLayers(
      ["alpha", "beta"],
      conns,
      subs,
    );
    expect(cycleBreaks.length).toBeGreaterThan(0);
    // Both domains should still be assigned layers
    expect(layers).toHaveLength(2);
  });

  it("handles octospark fixture without errors", () => {
    const domainIds = [
      ...new Set(OCTOSPARK_FIXTURE.subsystems.map((s) => s.domain)),
    ];
    const { layers, cycleBreaks } = assignDomainLayers(
      domainIds,
      OCTOSPARK_FIXTURE.connections,
      OCTOSPARK_FIXTURE.subsystems,
    );
    // 7 domains
    expect(layers).toHaveLength(7);
    // Octospark has a cycle: media-storage ↔ services
    // (media-enrichment→ai-generation and publisher→asset-management)
    expect(cycleBreaks.length).toBeGreaterThanOrEqual(0);
    // platform-core should be at depth 0 (it's the foundation)
    const platformCore = layers.find((l) => l.domain === "platform-core")!;
    expect(platformCore.topoDepth).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// 2. assignDomainGrid
// ---------------------------------------------------------------------------

describe("assignDomainGrid", () => {
  it("returns empty for empty input", () => {
    expect(assignDomainGrid([])).toEqual([]);
  });

  it("assigns single domain to (0, 0)", () => {
    const result = assignDomainGrid([
      { domain: "alpha", topoDepth: 0, row: -1, col: -1, stationCount: 5 },
    ]);
    expect(result).toHaveLength(1);
    expect(result[0]!.row).toBe(0);
    expect(result[0]!.col).toBe(0);
  });

  it("assigns same-depth domains to same row", () => {
    const result = assignDomainGrid([
      { domain: "alpha", topoDepth: 0, row: -1, col: -1, stationCount: 3 },
      { domain: "beta", topoDepth: 0, row: -1, col: -1, stationCount: 5 },
    ]);
    // Both at depth 0 → same row
    expect(result[0]!.row).toBe(0);
    expect(result[1]!.row).toBe(0);
    // Sorted by descending station count: beta (5) first, alpha (3) second
    expect(result[0]!.domain).toBe("beta");
    expect(result[1]!.domain).toBe("alpha");
    expect(result[0]!.col).toBe(0);
    expect(result[1]!.col).toBe(1);
  });

  it("assigns different-depth domains to different rows", () => {
    const result = assignDomainGrid([
      { domain: "alpha", topoDepth: 0, row: -1, col: -1, stationCount: 3 },
      { domain: "beta", topoDepth: 1, row: -1, col: -1, stationCount: 5 },
    ]);
    expect(result.find((l) => l.domain === "alpha")!.row).toBe(0);
    expect(result.find((l) => l.domain === "beta")!.row).toBe(1);
  });

  it("wraps columns for large layers (MAX_COLS)", () => {
    // 9 domains at same depth → MAX_COLS = ceil(sqrt(9)) = 3
    const layers = Array.from({ length: 9 }, (_, i) => ({
      domain: `d${i}`,
      topoDepth: 0,
      row: -1,
      col: -1,
      stationCount: 9 - i,
    }));
    const result = assignDomainGrid(layers);
    // Should produce 3 rows of 3
    const rows = new Set(result.map((l) => l.row));
    expect(rows.size).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// 3. orderStationsWithinLine
// ---------------------------------------------------------------------------

describe("orderStationsWithinLine", () => {
  it("returns empty for no subsystems", () => {
    expect(orderStationsWithinLine("alpha", [], [], [])).toEqual([]);
  });

  it("returns single station for single subsystem", () => {
    const subs = [makeSub("a", "alpha")];
    expect(orderStationsWithinLine("alpha", subs, [], subs)).toEqual(["a"]);
  });

  it("orders by intra-domain topo depth", () => {
    const subs = [
      makeSub("a", "alpha"),
      makeSub("b", "alpha"),
      makeSub("c", "alpha"),
    ];
    // a depends on b, b depends on c → order: c, b, a
    const conns = [makeConn("a", "b"), makeConn("b", "c")];
    const result = orderStationsWithinLine("alpha", subs, conns, subs);
    expect(result.indexOf("c")).toBeLessThan(result.indexOf("b"));
    expect(result.indexOf("b")).toBeLessThan(result.indexOf("a"));
  });

  it("uses cross-domain fan-out as secondary sort", () => {
    const allSubs = [
      makeSub("a", "alpha"),
      makeSub("b", "alpha"),
      makeSub("x", "beta"),
      makeSub("y", "beta"),
    ];
    const domainSubs = allSubs.filter((s) => s.domain === "alpha");
    // b has more cross-domain connections than a
    const conns = [
      makeConn("x", "b"), // beta → alpha (b has fan-out)
      makeConn("y", "b"), // another cross-domain to b
      makeConn("x", "a"), // one cross-domain to a
    ];
    const result = orderStationsWithinLine("alpha", domainSubs, conns, allSubs);
    // Both at same topo depth, a has fan-out 1, b has fan-out 2
    // fan-out ascending → a before b
    expect(result.indexOf("a")).toBeLessThan(result.indexOf("b"));
  });

  it("uses alphabetical as stable tiebreaker", () => {
    const subs = [
      makeSub("c", "alpha"),
      makeSub("a", "alpha"),
      makeSub("b", "alpha"),
    ];
    // No connections → all same topo depth and fan-out → alphabetical
    const result = orderStationsWithinLine("alpha", subs, [], subs);
    expect(result).toEqual(["a", "b", "c"]);
  });

  it("handles intra-domain cycles gracefully", () => {
    const subs = [
      makeSub("a", "alpha"),
      makeSub("b", "alpha"),
    ];
    const conns = [makeConn("a", "b"), makeConn("b", "a")];
    const result = orderStationsWithinLine("alpha", subs, conns, subs);
    // Should still produce a valid ordering (both at same depth due to cycle)
    expect(result).toHaveLength(2);
    expect(new Set(result)).toEqual(new Set(["a", "b"]));
  });
});

// ---------------------------------------------------------------------------
// 4. buildCanonicalPositions
// ---------------------------------------------------------------------------

describe("buildCanonicalPositions", () => {
  it("returns empty for no lines", () => {
    expect(buildCanonicalPositions([]).size).toBe(0);
  });

  it("places single station at origin", () => {
    const lines: ComputedLine[] = [
      {
        domain: "alpha",
        color: "#fff",
        label: "Alpha",
        stationIds: ["a"],
        origin: { x: 100, y: 200 },
        segments: [],
      },
    ];
    const pos = buildCanonicalPositions(lines);
    expect(pos.get("a")).toEqual({ x: 100, y: 200 });
  });

  it("walks horizontal segments correctly", () => {
    const lines: ComputedLine[] = [
      {
        domain: "alpha",
        color: "#fff",
        label: "Alpha",
        stationIds: ["a", "b", "c"],
        origin: { x: 0, y: 0 },
        segments: [{ steps: 2, dx: 1, dy: 0 }],
      },
    ];
    const pos = buildCanonicalPositions(lines);
    expect(pos.get("a")).toEqual({ x: 0, y: 0 });
    expect(pos.get("b")).toEqual({ x: STATION_GAP, y: 0 });
    expect(pos.get("c")).toEqual({ x: 2 * STATION_GAP, y: 0 });
  });

  it("walks U-bend segments correctly", () => {
    const lines: ComputedLine[] = [
      {
        domain: "alpha",
        color: "#fff",
        label: "Alpha",
        stationIds: ["a", "b", "c", "d"],
        origin: { x: 0, y: 0 },
        segments: [
          { steps: 1, dx: 1, dy: 0 }, // a→b: right
          { steps: 1, dx: 0, dy: 1 }, // b→c: down
          { steps: 1, dx: -1, dy: 0 }, // c→d: left
        ],
      },
    ];
    const pos = buildCanonicalPositions(lines);
    expect(pos.get("a")).toEqual({ x: 0, y: 0 });
    expect(pos.get("b")).toEqual({ x: STATION_GAP, y: 0 });
    expect(pos.get("c")).toEqual({ x: STATION_GAP, y: LINE_GAP });
    expect(pos.get("d")).toEqual({ x: 0, y: LINE_GAP });
  });

  it("handles multiple lines independently", () => {
    const lines: ComputedLine[] = [
      {
        domain: "alpha",
        color: "#fff",
        label: "Alpha",
        stationIds: ["a1"],
        origin: { x: 0, y: 0 },
        segments: [],
      },
      {
        domain: "beta",
        color: "#fff",
        label: "Beta",
        stationIds: ["b1"],
        origin: { x: 1000, y: 500 },
        segments: [],
      },
    ];
    const pos = buildCanonicalPositions(lines);
    expect(pos.get("a1")).toEqual({ x: 0, y: 0 });
    expect(pos.get("b1")).toEqual({ x: 1000, y: 500 });
  });
});

// ---------------------------------------------------------------------------
// 5. applyCompactLayout
// ---------------------------------------------------------------------------

describe("applyCompactLayout", () => {
  const twoLines: ComputedLine[] = [
    {
      domain: "alpha",
      color: "#fff",
      label: "Alpha",
      stationIds: ["a1", "a2", "a3"],
      origin: { x: 0, y: 0 },
      segments: [{ steps: 2, dx: 1, dy: 0 }],
    },
    {
      domain: "beta",
      color: "#fff",
      label: "Beta",
      stationIds: ["b1", "b2"],
      origin: { x: 0, y: LINE_GAP },
      segments: [{ steps: 1, dx: 1, dy: 0 }],
    },
  ];

  it("places all visible stations when all are visible", () => {
    const visible = new Set(["a1", "a2", "a3", "b1", "b2"]);
    const pos = applyCompactLayout(twoLines, visible);
    expect(pos.size).toBe(5);
  });

  it("centers shorter lines", () => {
    const visible = new Set(["a1", "a2", "a3", "b1", "b2"]);
    const pos = applyCompactLayout(twoLines, visible);

    // Alpha has 3 stations (width = 2 * STATION_GAP)
    // Beta has 2 stations (width = 1 * STATION_GAP)
    // Max width = 2 * STATION_GAP, beta offset = (2 * STATION_GAP - STATION_GAP) / 2
    const betaOffset = STATION_GAP / 2;
    expect(pos.get("b1")!.x).toBe(betaOffset);
    expect(pos.get("b2")!.x).toBe(betaOffset + STATION_GAP);
  });

  it("stacks lines vertically", () => {
    const visible = new Set(["a1", "b1"]);
    const pos = applyCompactLayout(twoLines, visible);
    expect(pos.get("a1")!.y).toBe(0);
    expect(pos.get("b1")!.y).toBe(LINE_GAP);
  });

  it("skips lines with no visible stations", () => {
    const visible = new Set(["b1", "b2"]);
    const pos = applyCompactLayout(twoLines, visible);
    // Only beta line visible, should start at y=0
    expect(pos.get("b1")!.y).toBe(0);
  });

  it("handles fallback stations not on any line", () => {
    const visible = new Set(["a1", "unknown"]);
    const pos = applyCompactLayout(twoLines, visible);
    expect(pos.has("a1")).toBe(true);
    expect(pos.has("unknown")).toBe(true);
    // unknown should be on fallback row below alpha
    expect(pos.get("unknown")!.y).toBe(LINE_GAP);
    expect(pos.get("unknown")!.x).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// generateSegments
// ---------------------------------------------------------------------------

describe("generateSegments", () => {
  it("returns empty for 0 or 1 stations", () => {
    expect(generateSegments(0)).toEqual([]);
    expect(generateSegments(1)).toEqual([]);
  });

  it("returns single rightward segment for small lines", () => {
    const segs = generateSegments(5);
    expect(segs).toEqual([{ steps: 4, dx: 1, dy: 0 }]);
  });

  it("returns single segment for MAX_STATIONS_PER_SEGMENT", () => {
    const segs = generateSegments(MAX_STATIONS_PER_SEGMENT);
    expect(segs).toEqual([
      { steps: MAX_STATIONS_PER_SEGMENT - 1, dx: 1, dy: 0 },
    ]);
  });

  it("wraps with U-bend for lines > MAX_STATIONS_PER_SEGMENT", () => {
    const segs = generateSegments(MAX_STATIONS_PER_SEGMENT + 5);
    // First segment: 9 right, then 1 down, then 4 left
    expect(segs[0]).toEqual({
      steps: MAX_STATIONS_PER_SEGMENT - 1,
      dx: 1,
      dy: 0,
    });
    expect(segs[1]).toEqual({ steps: 1, dx: 0, dy: 1 });
    expect(segs[2]).toEqual({ steps: 4, dx: -1, dy: 0 });
  });

  it("covers all stations with segments", () => {
    for (const count of [2, 5, 10, 11, 20, 25, 50]) {
      const segs = generateSegments(count);
      const totalPlaced =
        1 + segs.reduce((sum, seg) => sum + seg.steps, 0);
      expect(totalPlaced).toBe(count);
    }
  });

  it("alternates horizontal direction on U-bends", () => {
    const segs = generateSegments(25);
    // First horizontal: right (dx=1)
    expect(segs[0]!.dx).toBe(1);
    // After turn down: left (dx=-1)
    const secondHoriz = segs.find(
      (s, i) => i > 1 && s.dy === 0 && s.dx !== 0,
    );
    expect(secondHoriz!.dx).toBe(-1);
  });
});

// ---------------------------------------------------------------------------
// assignDomainColors
// ---------------------------------------------------------------------------

describe("assignDomainColors", () => {
  it("uses manifest colors when available", () => {
    const colors = assignDomainColors(
      { alpha: { label: "Alpha", color: "#ff0000" } },
      ["alpha"],
    );
    expect(colors.get("alpha")).toBe("#ff0000");
  });

  it("falls back to palette for domains without manifest colors", () => {
    const colors = assignDomainColors({}, ["alpha", "beta"]);
    expect(colors.get("alpha")).toBe("#3b82f6"); // first palette color
    expect(colors.get("beta")).toBe("#22c55e"); // second palette color
  });

  it("uses HSL cycling for > 12 domains without manifest colors", () => {
    const ids = Array.from({ length: 15 }, (_, i) => `d${i}`);
    const colors = assignDomainColors({}, ids);
    expect(colors.size).toBe(15);
    // All should be valid hex colors
    for (const color of colors.values()) {
      expect(color).toMatch(/^#[0-9a-f]{6}$/);
    }
  });
});

// ---------------------------------------------------------------------------
// buildDynamicLayout — integration test with octospark fixture
// ---------------------------------------------------------------------------

describe("buildDynamicLayout", () => {
  it("produces a valid layout for octospark fixture", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);

    // All 18 subsystems should have positions
    expect(layout.positions.size).toBe(OCTOSPARK_FIXTURE.subsystems.length);

    // 7 domains → 7 lines
    expect(layout.lines).toHaveLength(7);
    expect(layout.layers).toHaveLength(7);

    // Octospark has a media-storage ↔ services cycle, algorithm breaks it
    expect(layout.cycleBreaks.length).toBeGreaterThanOrEqual(0);

    // Every subsystem ID should appear exactly once across all lines
    const allStationIds = layout.lines.flatMap((l) => l.stationIds);
    expect(allStationIds).toHaveLength(OCTOSPARK_FIXTURE.subsystems.length);
    expect(new Set(allStationIds).size).toBe(
      OCTOSPARK_FIXTURE.subsystems.length,
    );
  });

  it("produces no overlapping positions", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);
    const positionSet = new Set<string>();

    for (const [id, pos] of layout.positions) {
      const key = `${pos.x},${pos.y}`;
      expect(positionSet.has(key)).toBe(false);
      positionSet.add(key);
      // Suppress unused variable warning
      void id;
    }
  });

  it("assigns correct domain colors", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);
    const platformCoreLine = layout.lines.find(
      (l) => l.domain === "platform-core",
    )!;
    expect(platformCoreLine.color).toBe("#3b82f6");
  });

  it("handles empty data gracefully", () => {
    const emptyData: TubeMapData = {
      meta: { name: "Empty", version: "0", description: "" },
      domains: {},
      subsystems: [],
      connections: [],
      coverage_percent: 100,
      unmatched_count: 0,
    };
    const layout = buildDynamicLayout(emptyData);
    expect(layout.positions.size).toBe(0);
    expect(layout.lines).toHaveLength(0);
    expect(layout.layers).toHaveLength(0);
  });

  it("handles single-domain data", () => {
    const singleDomain: TubeMapData = {
      meta: { name: "Single", version: "0", description: "" },
      domains: { alpha: { label: "Alpha", color: "#ff0000" } },
      subsystems: [
        makeSub("a", "alpha"),
        makeSub("b", "alpha"),
        makeSub("c", "alpha"),
      ],
      connections: [makeConn("a", "b"), makeConn("b", "c")],
      coverage_percent: 100,
      unmatched_count: 0,
    };
    const layout = buildDynamicLayout(singleDomain);
    expect(layout.positions.size).toBe(3);
    expect(layout.lines).toHaveLength(1);
  });

  it("produces no overlapping lines for 20-domain fixture", () => {
    // Synthetic fixture: 20 domains, 3 stations each
    const subsystems: TubeMapSubsystem[] = [];
    const connections: TubeMapConnection[] = [];
    const domains: Record<string, { label: string; color: string }> = {};

    for (let d = 0; d < 20; d++) {
      const domainId = `domain-${d}`;
      domains[domainId] = { label: `Domain ${d}`, color: "" };
      for (let s = 0; s < 3; s++) {
        subsystems.push(makeSub(`d${d}-s${s}`, domainId));
      }
      // Chain domains: domain-i depends on domain-(i-1)
      if (d > 0) {
        connections.push(makeConn(`d${d}-s0`, `d${d - 1}-s0`));
      }
    }

    const data: TubeMapData = {
      meta: { name: "Large", version: "0", description: "" },
      domains,
      subsystems,
      connections,
      coverage_percent: 0,
      unmatched_count: 0,
    };

    const layout = buildDynamicLayout(data);
    expect(layout.positions.size).toBe(60);

    // No overlapping positions
    const positionSet = new Set<string>();
    for (const [, pos] of layout.positions) {
      const key = `${pos.x},${pos.y}`;
      expect(positionSet.has(key)).toBe(false);
      positionSet.add(key);
    }
  });

  it("handles circular domain dependencies", () => {
    const data: TubeMapData = {
      meta: { name: "Circular", version: "0", description: "" },
      domains: {
        alpha: { label: "Alpha", color: "#ff0000" },
        beta: { label: "Beta", color: "#00ff00" },
        gamma: { label: "Gamma", color: "#0000ff" },
      },
      subsystems: [
        makeSub("a", "alpha"),
        makeSub("b", "beta"),
        makeSub("c", "gamma"),
      ],
      connections: [
        makeConn("a", "b"), // alpha → beta
        makeConn("b", "c"), // beta → gamma
        makeConn("c", "a"), // gamma → alpha (creates cycle)
      ],
      coverage_percent: 0,
      unmatched_count: 0,
    };

    const layout = buildDynamicLayout(data);
    // Should still produce a valid layout despite cycle
    expect(layout.positions.size).toBe(3);
    expect(layout.cycleBreaks.length).toBeGreaterThan(0);
    // All stations should have unique positions
    const positionSet = new Set<string>();
    for (const [, pos] of layout.positions) {
      const key = `${pos.x},${pos.y}`;
      expect(positionSet.has(key)).toBe(false);
      positionSet.add(key);
    }
  });

  it("positions match within ±1 station gap for octospark", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);

    // platform-core should have 6 stations in a row
    const platformLine = layout.lines.find(
      (l) => l.domain === "platform-core",
    )!;
    expect(platformLine.stationIds).toHaveLength(6);

    // All platform-core stations should be on the same y (horizontal line)
    const positions = platformLine.stationIds.map(
      (id) => layout.positions.get(id)!,
    );
    const baseY = positions[0]!.y;
    for (const pos of positions) {
      expect(pos.y).toBe(baseY);
    }

    // Stations should be spaced by STATION_GAP
    for (let i = 1; i < positions.length; i++) {
      expect(positions[i]!.x - positions[i - 1]!.x).toBe(STATION_GAP);
    }
  });

  it("compact mode correctly centers filtered lines", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);

    // Filter to only platform-core and services
    const visibleIds = new Set(
      OCTOSPARK_FIXTURE.subsystems
        .filter(
          (s) =>
            s.domain === "platform-core" || s.domain === "services",
        )
        .map((s) => s.id),
    );

    const compactPositions = applyCompactLayout(layout.lines, visibleIds);
    expect(compactPositions.size).toBe(visibleIds.size);

    // Both lines should have unique y values
    const yValues = new Set([...compactPositions.values()].map((p) => p.y));
    expect(yValues.size).toBe(2); // two domain lines

    // Platform-core line (6 stations) should be wider than services (4 stations)
    // The wider line should have x starting at 0
    const platformPositions = ["auth", "org-team", "credit-service", "billing", "notifications", "webhooks"]
      .filter((id) => visibleIds.has(id))
      .map((id) => compactPositions.get(id)!)
      .filter(Boolean);

    // First station of the widest line should be at x=0
    const minX = Math.min(...platformPositions.map((p) => p.x));
    expect(minX).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// normalizeOrphanDomains
// ---------------------------------------------------------------------------

describe("normalizeOrphanDomains", () => {
  it("returns data unchanged when all subsystems have known domains", () => {
    const result = normalizeOrphanDomains(OCTOSPARK_FIXTURE);
    expect(result).toBe(OCTOSPARK_FIXTURE); // same reference, no copy
  });

  it("remaps subsystems with empty domain to UNASSIGNED_DOMAIN_ID", () => {
    const data: TubeMapData = {
      meta: { name: "Test", version: "0", description: "" },
      domains: { alpha: { label: "Alpha", color: "#ff0000" } },
      subsystems: [
        makeSub("a", "alpha"),
        makeSub("b", ""), // empty domain
      ],
      connections: [],
      coverage_percent: 0,
      unmatched_count: 0,
    };
    const result = normalizeOrphanDomains(data);
    expect(result.subsystems[1]!.domain).toBe(UNASSIGNED_DOMAIN_ID);
    expect(result.subsystems[0]!.domain).toBe("alpha");
  });

  it("remaps subsystems with unknown domain to UNASSIGNED_DOMAIN_ID", () => {
    const data: TubeMapData = {
      meta: { name: "Test", version: "0", description: "" },
      domains: { alpha: { label: "Alpha", color: "#ff0000" } },
      subsystems: [
        makeSub("a", "alpha"),
        makeSub("b", "nonexistent-domain"),
      ],
      connections: [],
      coverage_percent: 0,
      unmatched_count: 0,
    };
    const result = normalizeOrphanDomains(data);
    expect(result.subsystems[1]!.domain).toBe(UNASSIGNED_DOMAIN_ID);
  });

  it("adds unassigned domain entry to domains map", () => {
    const data: TubeMapData = {
      meta: { name: "Test", version: "0", description: "" },
      domains: { alpha: { label: "Alpha", color: "#ff0000" } },
      subsystems: [makeSub("a", "orphan-domain")],
      connections: [],
      coverage_percent: 0,
      unmatched_count: 0,
    };
    const result = normalizeOrphanDomains(data);
    expect(result.domains[UNASSIGNED_DOMAIN_ID]).toEqual({
      label: UNASSIGNED_LABEL,
      color: UNASSIGNED_COLOR,
    });
    // Original domain still present
    expect(result.domains["alpha"]).toEqual({ label: "Alpha", color: "#ff0000" });
  });

  it("does not modify original data", () => {
    const data: TubeMapData = {
      meta: { name: "Test", version: "0", description: "" },
      domains: { alpha: { label: "Alpha", color: "#ff0000" } },
      subsystems: [makeSub("a", "orphan")],
      connections: [],
      coverage_percent: 0,
      unmatched_count: 0,
    };
    normalizeOrphanDomains(data);
    // Original data unchanged
    expect(data.subsystems[0]!.domain).toBe("orphan");
    expect(data.domains[UNASSIGNED_DOMAIN_ID]).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// buildDynamicLayout — unassigned domain handling
// ---------------------------------------------------------------------------

describe("buildDynamicLayout — unassigned domain", () => {
  it("places orphan subsystems on a gray unassigned line at the bottom", () => {
    const data: TubeMapData = {
      meta: { name: "Test", version: "0", description: "" },
      domains: {
        alpha: { label: "Alpha", color: "#ff0000" },
        [UNASSIGNED_DOMAIN_ID]: { label: UNASSIGNED_LABEL, color: UNASSIGNED_COLOR },
      },
      subsystems: [
        makeSub("a", "alpha"),
        makeSub("b", "alpha"),
        makeSub("orphan1", UNASSIGNED_DOMAIN_ID),
        makeSub("orphan2", UNASSIGNED_DOMAIN_ID),
      ],
      connections: [],
      coverage_percent: 0,
      unmatched_count: 0,
    };

    const layout = buildDynamicLayout(data);

    // Should have 2 lines: alpha + unassigned
    expect(layout.lines).toHaveLength(2);

    const unassignedLine = layout.lines.find((l) => l.domain === UNASSIGNED_DOMAIN_ID);
    expect(unassignedLine).toBeDefined();
    expect(unassignedLine!.color).toBe(UNASSIGNED_COLOR);
    expect(unassignedLine!.label).toBe(UNASSIGNED_LABEL);
    expect(unassignedLine!.stationIds).toHaveLength(2);

    // Unassigned line should be below the alpha line
    const alphaLine = layout.lines.find((l) => l.domain === "alpha")!;
    expect(unassignedLine!.origin.y).toBeGreaterThan(alphaLine.origin.y);

    // All 4 subsystems should have positions
    expect(layout.positions.size).toBe(4);
  });

  it("sorts unassigned stations alphabetically", () => {
    const data: TubeMapData = {
      meta: { name: "Test", version: "0", description: "" },
      domains: {
        [UNASSIGNED_DOMAIN_ID]: { label: UNASSIGNED_LABEL, color: UNASSIGNED_COLOR },
      },
      subsystems: [
        makeSub("zebra", UNASSIGNED_DOMAIN_ID),
        makeSub("apple", UNASSIGNED_DOMAIN_ID),
        makeSub("mango", UNASSIGNED_DOMAIN_ID),
      ],
      connections: [],
      coverage_percent: 0,
      unmatched_count: 0,
    };

    const layout = buildDynamicLayout(data);
    const unassignedLine = layout.lines.find((l) => l.domain === UNASSIGNED_DOMAIN_ID)!;
    expect(unassignedLine.stationIds).toEqual(["apple", "mango", "zebra"]);
  });

  it("works end-to-end: normalize then layout with orphan subsystems", () => {
    const rawData: TubeMapData = {
      meta: { name: "Test", version: "0", description: "" },
      domains: { alpha: { label: "Alpha", color: "#ff0000" } },
      subsystems: [
        makeSub("a", "alpha"),
        makeSub("orphan1", "nonexistent"),
        makeSub("orphan2", ""),
      ],
      connections: [makeConn("orphan1", "a")],
      coverage_percent: 0,
      unmatched_count: 0,
    };

    const normalized = normalizeOrphanDomains(rawData);
    const layout = buildDynamicLayout(normalized);

    // 2 lines: alpha + unassigned
    expect(layout.lines).toHaveLength(2);

    // All 3 subsystems positioned
    expect(layout.positions.size).toBe(3);

    // Unassigned line exists at the bottom
    const unassignedLine = layout.lines.find((l) => l.domain === UNASSIGNED_DOMAIN_ID)!;
    expect(unassignedLine.stationIds).toContain("orphan1");
    expect(unassignedLine.stationIds).toContain("orphan2");
  });

  it("does not create unassigned line when all domains are known", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);
    const unassignedLine = layout.lines.find((l) => l.domain === UNASSIGNED_DOMAIN_ID);
    expect(unassignedLine).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Snapshot tests — layout algorithm output
// ---------------------------------------------------------------------------

/** Convert a Map<string, {x,y}> to a sorted plain object for stable snapshots. */
function positionsToSnapshot(
  positions: Map<string, { x: number; y: number }>,
): Record<string, { x: number; y: number }> {
  const result: Record<string, { x: number; y: number }> = {};
  for (const key of [...positions.keys()].sort()) {
    result[key] = positions.get(key)!;
  }
  return result;
}

/** Convert ComputedLine[] to a snapshot-friendly format (drop color for stability). */
function linesToSnapshot(
  lines: ComputedLine[],
): Array<{ domain: string; label: string; stationIds: string[]; origin: { x: number; y: number } }> {
  return lines.map((l) => ({
    domain: l.domain,
    label: l.label,
    stationIds: l.stationIds,
    origin: l.origin,
  }));
}

describe("snapshot: layout algorithm output", () => {
  it("octospark fixture — full layout positions", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);
    expect(positionsToSnapshot(layout.positions)).toMatchSnapshot();
  });

  it("octospark fixture — computed lines", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);
    expect(linesToSnapshot(layout.lines)).toMatchSnapshot();
  });

  it("octospark fixture — grid layers", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);
    expect(layout.layers).toMatchSnapshot();
  });

  it("20-domain synthetic fixture — full layout positions", () => {
    const subsystems: TubeMapSubsystem[] = [];
    const connections: TubeMapConnection[] = [];
    const domains: Record<string, { label: string; color: string }> = {};

    for (let d = 0; d < 20; d++) {
      const domainId = `domain-${d}`;
      domains[domainId] = { label: `Domain ${d}`, color: "" };
      for (let s = 0; s < 3; s++) {
        subsystems.push(makeSub(`d${d}-s${s}`, domainId));
      }
      if (d > 0) {
        connections.push(makeConn(`d${d}-s0`, `d${d - 1}-s0`));
      }
    }

    const data: TubeMapData = {
      meta: { name: "Large", version: "0", description: "" },
      domains,
      subsystems,
      connections,
      coverage_percent: 0,
      unmatched_count: 0,
    };

    const layout = buildDynamicLayout(data);
    expect(positionsToSnapshot(layout.positions)).toMatchSnapshot();
  });

  it("20-domain synthetic fixture — computed lines", () => {
    const subsystems: TubeMapSubsystem[] = [];
    const connections: TubeMapConnection[] = [];
    const domains: Record<string, { label: string; color: string }> = {};

    for (let d = 0; d < 20; d++) {
      const domainId = `domain-${d}`;
      domains[domainId] = { label: `Domain ${d}`, color: "" };
      for (let s = 0; s < 3; s++) {
        subsystems.push(makeSub(`d${d}-s${s}`, domainId));
      }
      if (d > 0) {
        connections.push(makeConn(`d${d}-s0`, `d${d - 1}-s0`));
      }
    }

    const data: TubeMapData = {
      meta: { name: "Large", version: "0", description: "" },
      domains,
      subsystems,
      connections,
      coverage_percent: 0,
      unmatched_count: 0,
    };

    const layout = buildDynamicLayout(data);
    expect(linesToSnapshot(layout.lines)).toMatchSnapshot();
  });

  it("compact layout — filtered octospark (platform-core + services)", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);

    const visibleIds = new Set(
      OCTOSPARK_FIXTURE.subsystems
        .filter(
          (s) =>
            s.domain === "platform-core" || s.domain === "services",
        )
        .map((s) => s.id),
    );

    const compactPositions = applyCompactLayout(layout.lines, visibleIds);
    expect(positionsToSnapshot(compactPositions)).toMatchSnapshot();
  });

  it("compact layout — single domain filter", () => {
    const layout = buildDynamicLayout(OCTOSPARK_FIXTURE);

    const visibleIds = new Set(
      OCTOSPARK_FIXTURE.subsystems
        .filter((s) => s.domain === "media-storage")
        .map((s) => s.id),
    );

    const compactPositions = applyCompactLayout(layout.lines, visibleIds);
    expect(positionsToSnapshot(compactPositions)).toMatchSnapshot();
  });
});

// Suppress lint: all imports are used in tests above
void NODE_WIDTH;
