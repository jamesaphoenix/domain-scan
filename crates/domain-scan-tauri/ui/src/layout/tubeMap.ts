import type {
  TubeMapData,
  TubeMapSubsystem,
  TubeMapConnection,
} from "../types";
import type {
  ComputedLine,
  CycleBreak,
  DomainLayer,
  LayoutGrid,
  Segment,
} from "./types";
import { assignDomainColors } from "./colors";

// ---------------------------------------------------------------------------
// Layout constants (from spec section 5.2)
// ---------------------------------------------------------------------------

export const STATION_GAP = 420;
export const LINE_GAP = 320;
export const NODE_WIDTH = 360;
export const COL_MARGIN = 300;
export const LINE_ROW_HEIGHT = 640;
export const MAX_STATIONS_PER_SEGMENT = 10;

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/**
 * Build a complete tube map layout from TubeMapData.
 * Returns positions for all stations, computed lines, and grid layers.
 */
export function buildDynamicLayout(data: TubeMapData): LayoutGrid {
  // Get unique domain IDs (preserving order of first appearance)
  const seen = new Set<string>();
  const domainIds: string[] = [];
  for (const s of data.subsystems) {
    if (!seen.has(s.domain)) {
      seen.add(s.domain);
      domainIds.push(s.domain);
    }
  }

  // Step 1: Topo sort domains
  const { layers: rawLayers, cycleBreaks } = assignDomainLayers(
    domainIds,
    data.connections,
    data.subsystems,
  );

  // Step 2: Grid packing
  const gridLayers = assignDomainGrid(rawLayers);

  // Step 3: Assign colors
  const domainColors = assignDomainColors(data.domains, domainIds);

  // Step 4: Build computed lines
  const origins = computeOrigins(gridLayers);
  const lines: ComputedLine[] = [];

  for (const layer of gridLayers) {
    const domainSubs = data.subsystems.filter(
      (s) => s.domain === layer.domain,
    );
    const stationIds = orderStationsWithinLine(
      layer.domain,
      domainSubs,
      data.connections,
      data.subsystems,
    );

    const segments = generateSegments(stationIds.length);
    const origin = origins.get(layer.domain) ?? { x: 0, y: 0 };

    lines.push({
      domain: layer.domain,
      color: domainColors.get(layer.domain) ?? "#6b7280",
      label: data.domains[layer.domain]?.label ?? layer.domain,
      stationIds,
      origin,
      segments,
    });
  }

  // Step 5: Build canonical positions
  const positions = buildCanonicalPositions(lines);

  return { layers: gridLayers, lines, positions, cycleBreaks };
}

// ---------------------------------------------------------------------------
// Phase 1: Domain row assignment — Kahn's topo sort on domain DAG
// ---------------------------------------------------------------------------

/**
 * Topological sort on the domain dependency DAG.
 * Uses Kahn's algorithm. Breaks cycles by removing the lowest-weight edge.
 *
 * Edge direction: if subsystem A (domain X) depends on subsystem B (domain Y),
 * then Y should come before X in topo order. DAG edge: Y → X.
 */
export function assignDomainLayers(
  domainIds: string[],
  connections: TubeMapConnection[],
  subsystems: TubeMapSubsystem[],
): { layers: DomainLayer[]; cycleBreaks: CycleBreak[] } {
  if (domainIds.length === 0) {
    return { layers: [], cycleBreaks: [] };
  }

  // Build subsystem → domain lookup
  const subsystemDomain = new Map<string, string>();
  for (const s of subsystems) {
    subsystemDomain.set(s.id, s.domain);
  }

  // Build cross-domain edge weights and adjacency
  const crossWeights = new Map<string, number>(); // "Y->X" → count
  const adj = new Map<string, Set<string>>();
  const inDegree = new Map<string, number>();

  for (const d of domainIds) {
    adj.set(d, new Set());
    inDegree.set(d, 0);
  }

  for (const conn of connections) {
    const fromDomain = subsystemDomain.get(conn.from);
    const toDomain = subsystemDomain.get(conn.to);
    if (!fromDomain || !toDomain || fromDomain === toDomain) continue;

    // conn.from depends on conn.to → toDomain before fromDomain
    const key = `${toDomain}->${fromDomain}`;
    crossWeights.set(key, (crossWeights.get(key) ?? 0) + 1);

    const neighbors = adj.get(toDomain)!;
    if (!neighbors.has(fromDomain)) {
      neighbors.add(fromDomain);
      inDegree.set(fromDomain, (inDegree.get(fromDomain) ?? 0) + 1);
    }
  }

  // Kahn's algorithm with cycle breaking
  const topoOrder: string[][] = [];
  const cycleBreaks: CycleBreak[] = [];
  const remaining = new Set(domainIds);
  const currentInDegree = new Map(inDegree);

  while (remaining.size > 0) {
    // Find nodes with in-degree 0
    const layer: string[] = [];
    for (const d of remaining) {
      if ((currentInDegree.get(d) ?? 0) === 0) {
        layer.push(d);
      }
    }

    if (layer.length === 0) {
      // Cycle detected — break the lowest-weight edge among remaining nodes
      let minWeight = Infinity;
      let minEdge: CycleBreak | null = null;

      for (const from of remaining) {
        const neighbors = adj.get(from);
        if (!neighbors) continue;
        for (const to of neighbors) {
          if (!remaining.has(to)) continue;
          const key = `${from}->${to}`;
          const weight = crossWeights.get(key) ?? 0;
          if (weight < minWeight) {
            minWeight = weight;
            minEdge = { from, to };
          }
        }
      }

      if (minEdge) {
        adj.get(minEdge.from)?.delete(minEdge.to);
        currentInDegree.set(
          minEdge.to,
          Math.max(0, (currentInDegree.get(minEdge.to) ?? 0) - 1),
        );
        cycleBreaks.push(minEdge);
      } else {
        // Fallback: treat all remaining as one layer
        topoOrder.push([...remaining]);
        remaining.clear();
      }
      continue;
    }

    topoOrder.push(layer);

    for (const d of layer) {
      remaining.delete(d);
      const neighbors = adj.get(d);
      if (!neighbors) continue;
      for (const neighbor of neighbors) {
        if (remaining.has(neighbor)) {
          currentInDegree.set(
            neighbor,
            Math.max(0, (currentInDegree.get(neighbor) ?? 0) - 1),
          );
        }
      }
    }
  }

  // Build DomainLayer array with station counts
  const stationCountByDomain = new Map<string, number>();
  for (const s of subsystems) {
    stationCountByDomain.set(
      s.domain,
      (stationCountByDomain.get(s.domain) ?? 0) + 1,
    );
  }

  const layers: DomainLayer[] = [];
  for (let depth = 0; depth < topoOrder.length; depth++) {
    for (const domain of topoOrder[depth]!) {
      layers.push({
        domain,
        topoDepth: depth,
        row: -1,
        col: -1,
        stationCount: stationCountByDomain.get(domain) ?? 0,
      });
    }
  }

  return { layers, cycleBreaks };
}

// ---------------------------------------------------------------------------
// Phase 2: Grid packing — bin-pack domains into (row, col) grid
// ---------------------------------------------------------------------------

/**
 * Assign (row, col) to each domain.
 * MAX_COLS = ceil(sqrt(N)). Domains at same topo depth share a row.
 * Within a row, sorted by descending station count.
 */
export function assignDomainGrid(layers: DomainLayer[]): DomainLayer[] {
  if (layers.length === 0) return [];

  const maxCols = Math.ceil(Math.sqrt(layers.length));

  // Group by topo depth
  const byDepth = new Map<number, DomainLayer[]>();
  for (const layer of layers) {
    const group = byDepth.get(layer.topoDepth) ?? [];
    group.push(layer);
    byDepth.set(layer.topoDepth, group);
  }

  const result: DomainLayer[] = [];
  let currentRow = 0;

  const sortedDepths = [...byDepth.keys()].sort((a, b) => a - b);

  for (const depth of sortedDepths) {
    const group = byDepth.get(depth)!;
    // Sort by descending station count
    group.sort((a, b) => b.stationCount - a.stationCount);

    for (let i = 0; i < group.length; i++) {
      const col = i % maxCols;
      const rowOffset = Math.floor(i / maxCols);

      result.push({
        ...group[i]!,
        row: currentRow + rowOffset,
        col,
      });
    }

    currentRow += Math.ceil(group.length / maxCols);
  }

  return result;
}

// ---------------------------------------------------------------------------
// Phase 3: Station ordering within a domain line
// ---------------------------------------------------------------------------

/**
 * Order stations within a domain line.
 * Sort by: intra-domain topo depth (asc), cross-domain fan-out (asc), alphabetical ID.
 */
export function orderStationsWithinLine(
  domainId: string,
  domainSubsystems: TubeMapSubsystem[],
  connections: TubeMapConnection[],
  allSubsystems: TubeMapSubsystem[],
): string[] {
  if (domainSubsystems.length === 0) return [];
  if (domainSubsystems.length === 1) return [domainSubsystems[0]!.id];

  const ids = new Set(domainSubsystems.map((s) => s.id));

  // Build subsystem → domain lookup
  const subsystemDomain = new Map<string, string>();
  for (const s of allSubsystems) {
    subsystemDomain.set(s.id, s.domain);
  }

  // 1. Intra-domain topo sort
  // conn.from depends on conn.to → to should come before from
  // DAG edge: to → from
  const intraAdj = new Map<string, string[]>();
  const intraInDegree = new Map<string, number>();
  for (const id of ids) {
    intraAdj.set(id, []);
    intraInDegree.set(id, 0);
  }

  for (const conn of connections) {
    if (!ids.has(conn.from) || !ids.has(conn.to)) continue;
    intraAdj.get(conn.to)!.push(conn.from);
    intraInDegree.set(
      conn.from,
      (intraInDegree.get(conn.from) ?? 0) + 1,
    );
  }

  // Kahn's for intra-domain depth
  const topoDepth = new Map<string, number>();
  const remaining = new Set(ids);
  let currentDepth = 0;

  while (remaining.size > 0) {
    const layer: string[] = [];
    for (const id of remaining) {
      if ((intraInDegree.get(id) ?? 0) === 0) {
        layer.push(id);
      }
    }

    if (layer.length === 0) {
      // Cycle — assign remaining to current depth
      for (const id of remaining) {
        topoDepth.set(id, currentDepth);
      }
      break;
    }

    for (const id of layer) {
      topoDepth.set(id, currentDepth);
      remaining.delete(id);
      for (const neighbor of intraAdj.get(id) ?? []) {
        if (remaining.has(neighbor)) {
          intraInDegree.set(
            neighbor,
            Math.max(0, (intraInDegree.get(neighbor) ?? 0) - 1),
          );
        }
      }
    }
    currentDepth++;
  }

  // 2. Cross-domain fan-out count
  const fanOut = new Map<string, number>();
  for (const id of ids) {
    fanOut.set(id, 0);
  }

  for (const conn of connections) {
    if (ids.has(conn.to)) {
      const fromDomain = subsystemDomain.get(conn.from);
      if (fromDomain && fromDomain !== domainId) {
        fanOut.set(conn.to, (fanOut.get(conn.to) ?? 0) + 1);
      }
    }
    if (ids.has(conn.from)) {
      const toDomain = subsystemDomain.get(conn.to);
      if (toDomain && toDomain !== domainId) {
        fanOut.set(conn.from, (fanOut.get(conn.from) ?? 0) + 1);
      }
    }
  }

  // 3. Sort: topoDepth asc → fanOut asc → alphabetical
  return [...ids].sort((a, b) => {
    const depthDiff = (topoDepth.get(a) ?? 0) - (topoDepth.get(b) ?? 0);
    if (depthDiff !== 0) return depthDiff;

    const fanDiff = (fanOut.get(a) ?? 0) - (fanOut.get(b) ?? 0);
    if (fanDiff !== 0) return fanDiff;

    return a.localeCompare(b);
  });
}

// ---------------------------------------------------------------------------
// Phase 4: Build canonical positions from computed lines
// ---------------------------------------------------------------------------

/**
 * Build canonical station positions by walking each line's segments.
 * Reuses the segment walker pattern from octospark-visualizer.
 */
export function buildCanonicalPositions(
  lines: ComputedLine[],
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();

  for (const line of lines) {
    const { stationIds, origin, segments } = line;
    if (stationIds.length === 0) continue;

    positions.set(stationIds[0]!, { x: origin.x, y: origin.y });

    let stationIdx = 1;
    let currentX = origin.x;
    let currentY = origin.y;

    for (const seg of segments) {
      for (
        let step = 0;
        step < seg.steps && stationIdx < stationIds.length;
        step++
      ) {
        currentX += seg.dx * STATION_GAP;
        currentY += seg.dy * LINE_GAP;
        positions.set(stationIds[stationIdx]!, {
          x: currentX,
          y: currentY,
        });
        stationIdx++;
      }
    }

    // Fallback: remaining stations not covered by segments
    while (stationIdx < stationIds.length) {
      currentX += STATION_GAP;
      positions.set(stationIds[stationIdx]!, {
        x: currentX,
        y: currentY,
      });
      stationIdx++;
    }
  }

  return positions;
}

// ---------------------------------------------------------------------------
// Phase 5: Compact layout for filtered views
// ---------------------------------------------------------------------------

/**
 * Re-space visible stations within each line, centered and stacked.
 * Used when a filter is active (fewer visible than total).
 */
export function applyCompactLayout(
  lines: ComputedLine[],
  visibleIds: Set<string>,
): Map<string, { x: number; y: number }> {
  const positions = new Map<string, { x: number; y: number }>();

  // Group visible stations by their line, preserving order
  const visibleLineStations: Array<{
    domain: string;
    stations: string[];
  }> = [];
  for (const line of lines) {
    const visible = line.stationIds.filter((id) => visibleIds.has(id));
    if (visible.length > 0) {
      visibleLineStations.push({ domain: line.domain, stations: visible });
    }
  }

  // Collect fallback stations (not on any line)
  const allLineStationIds = new Set<string>();
  for (const line of lines) {
    for (const id of line.stationIds) {
      allLineStationIds.add(id);
    }
  }
  const fallbackStations = [...visibleIds].filter(
    (id) => !allLineStationIds.has(id),
  );

  // Find max line width for centering
  let maxLineWidth = 0;
  for (const group of visibleLineStations) {
    const w = (group.stations.length - 1) * STATION_GAP;
    if (w > maxLineWidth) maxLineWidth = w;
  }

  // Place each line centered, stacked vertically
  let currentY = 0;
  for (const group of visibleLineStations) {
    const lineWidth = (group.stations.length - 1) * STATION_GAP;
    const offsetX = (maxLineWidth - lineWidth) / 2;

    for (let i = 0; i < group.stations.length; i++) {
      positions.set(group.stations[i]!, {
        x: offsetX + i * STATION_GAP,
        y: currentY,
      });
    }
    currentY += LINE_GAP;
  }

  // Fallback row
  for (let i = 0; i < fallbackStations.length; i++) {
    positions.set(fallbackStations[i]!, {
      x: i * (NODE_WIDTH + 40),
      y: currentY,
    });
  }

  return positions;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Compute grid origins for each domain from DomainLayer grid assignments. */
function computeOrigins(
  gridLayers: DomainLayer[],
): Map<string, { x: number; y: number }> {
  const origins = new Map<string, { x: number; y: number }>();

  // Group by row to compute per-row column width
  const byRow = new Map<number, DomainLayer[]>();
  for (const layer of gridLayers) {
    const group = byRow.get(layer.row) ?? [];
    group.push(layer);
    byRow.set(layer.row, group);
  }

  for (const [row, rowLayers] of byRow) {
    // Effective horizontal stations (capped at MAX_STATIONS_PER_SEGMENT)
    const maxStations = Math.max(
      ...rowLayers.map((l) =>
        Math.min(l.stationCount, MAX_STATIONS_PER_SEGMENT),
      ),
    );
    const colWidth = maxStations * STATION_GAP + COL_MARGIN;

    for (const layer of rowLayers) {
      origins.set(layer.domain, {
        x: layer.col * colWidth,
        y: row * LINE_ROW_HEIGHT,
      });
    }
  }

  return origins;
}

/**
 * Generate direction segments for a line, with U-bend wrapping for long lines.
 * Lines with <= MAX_STATIONS_PER_SEGMENT stations get a single rightward segment.
 * Longer lines wrap: right → down → left → down → right → ...
 */
export function generateSegments(stationCount: number): Segment[] {
  if (stationCount <= 1) return [];

  const segments: Segment[] = [];
  let placed = 1; // origin already placed
  let direction = 1; // 1 = right, -1 = left

  while (placed < stationCount) {
    const horizontalSteps = Math.min(
      MAX_STATIONS_PER_SEGMENT - 1,
      stationCount - placed,
    );
    segments.push({ steps: horizontalSteps, dx: direction, dy: 0 });
    placed += horizontalSteps;

    if (placed >= stationCount) break;

    // U-bend turn down
    segments.push({ steps: 1, dx: 0, dy: 1 });
    placed += 1;
    direction *= -1;
  }

  return segments;
}
