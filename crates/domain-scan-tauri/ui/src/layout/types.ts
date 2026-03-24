// ---------------------------------------------------------------------------
// Unassigned domain constants
// ---------------------------------------------------------------------------

/** Synthetic domain ID for subsystems whose domain is not in the manifest's domains map */
export const UNASSIGNED_DOMAIN_ID = "__unassigned__";
/** Gray color for the unassigned domain line */
export const UNASSIGNED_COLOR = "#6b7280";
/** Display label for the unassigned domain */
export const UNASSIGNED_LABEL = "Unassigned";

/** Direction segment for tube line paths */
export interface Segment {
  steps: number;
  dx: number;
  dy: number;
}

/** A computed tube line — one per domain after layout */
export interface ComputedLine {
  domain: string;
  color: string;
  label: string;
  /** Ordered station IDs along this line */
  stationIds: string[];
  /** Starting position for the first station */
  origin: { x: number; y: number };
  /** Direction segments for U-bend wrapping */
  segments: Segment[];
}

/** Grid assignment for a domain */
export interface DomainLayer {
  domain: string;
  /** Topological sort depth (0 = source, higher = more dependent) */
  topoDepth: number;
  /** Grid row */
  row: number;
  /** Grid column */
  col: number;
  /** Number of stations in this domain */
  stationCount: number;
}

/** Cycle break recorded during topo sort */
export interface CycleBreak {
  from: string;
  to: string;
}

/** The full layout grid result */
export interface LayoutGrid {
  layers: DomainLayer[];
  lines: ComputedLine[];
  /** Pre-computed position for every station */
  positions: Map<string, { x: number; y: number }>;
  /** Cycle breaks detected during topo sort */
  cycleBreaks: CycleBreak[];
}
