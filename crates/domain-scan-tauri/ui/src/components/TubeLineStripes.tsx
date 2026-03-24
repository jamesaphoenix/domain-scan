import { memo, useMemo } from "react";
import { useViewport } from "@xyflow/react";
import type { ComputedLine } from "../layout/types";
import { NODE_WIDTH } from "../layout/tubeMap";

interface TubeLineStripesProps {
  lines: ComputedLine[];
  positions: Map<string, { x: number; y: number }>;
}

/** Approximate vertical center offset of a station node */
const NODE_CENTER_Y = 80;
/** SVG stroke width for the tube stripe */
const STRIPE_WIDTH = 8;
/** Opacity for tube stripes (subtle behind nodes) */
const STRIPE_OPACITY = 0.18;
/** Radius for rounded corners at U-bend turns */
const CORNER_RADIUS = 30;

/**
 * Build an SVG path string connecting all station centers along a tube line.
 * Uses quadratic bezier curves at turns for smooth rounded corners.
 */
function buildLinePath(
  line: ComputedLine,
  positions: Map<string, { x: number; y: number }>,
): string {
  const points: Array<{ x: number; y: number }> = [];

  for (const id of line.stationIds) {
    const pos = positions.get(id);
    if (!pos) continue;
    points.push({
      x: pos.x + NODE_WIDTH / 2,
      y: pos.y + NODE_CENTER_Y,
    });
  }

  if (points.length < 2) return "";

  const parts: string[] = [`M ${points[0]!.x} ${points[0]!.y}`];

  for (let i = 1; i < points.length - 1; i++) {
    const prev = points[i - 1]!;
    const curr = points[i]!;
    const next = points[i + 1]!;

    const dx1 = curr.x - prev.x;
    const dy1 = curr.y - prev.y;
    const dx2 = next.x - curr.x;
    const dy2 = next.y - curr.y;

    // A turn occurs when direction changes between horizontal and vertical
    const isHoriz1 = Math.abs(dx1) > 1 && Math.abs(dy1) < 1;
    const isVert1 = Math.abs(dy1) > 1 && Math.abs(dx1) < 1;
    const isHoriz2 = Math.abs(dx2) > 1 && Math.abs(dy2) < 1;
    const isVert2 = Math.abs(dy2) > 1 && Math.abs(dx2) < 1;
    const isTurn = (isHoriz1 && isVert2) || (isVert1 && isHoriz2);

    if (isTurn) {
      const len1 = Math.hypot(dx1, dy1);
      const len2 = Math.hypot(dx2, dy2);

      if (len1 < 1 || len2 < 1) {
        parts.push(`L ${curr.x} ${curr.y}`);
        continue;
      }

      const r = Math.min(CORNER_RADIUS, len1 / 2, len2 / 2);

      // Approach point: r pixels before the corner
      const sx = curr.x - (dx1 / len1) * r;
      const sy = curr.y - (dy1 / len1) * r;

      // Departure point: r pixels after the corner
      const ex = curr.x + (dx2 / len2) * r;
      const ey = curr.y + (dy2 / len2) * r;

      parts.push(`L ${sx} ${sy}`);
      parts.push(`Q ${curr.x} ${curr.y} ${ex} ${ey}`);
    } else {
      parts.push(`L ${curr.x} ${curr.y}`);
    }
  }

  const last = points[points.length - 1]!;
  parts.push(`L ${last.x} ${last.y}`);

  return parts.join(" ");
}

function TubeLineStripesComponent({ lines, positions }: TubeLineStripesProps) {
  const { x, y, zoom } = useViewport();

  const paths = useMemo(
    () =>
      lines
        .map((line) => ({
          domain: line.domain,
          color: line.color,
          d: buildLinePath(line, positions),
        }))
        .filter((p) => p.d.length > 0),
    [lines, positions],
  );

  return (
    <svg
      style={{
        position: "absolute",
        top: 0,
        left: 0,
        width: "100%",
        height: "100%",
        pointerEvents: "none",
        zIndex: -1,
      }}
    >
      <g transform={`translate(${x}, ${y}) scale(${zoom})`}>
        {paths.map((p) => (
          <path
            key={p.domain}
            d={p.d}
            fill="none"
            stroke={p.color}
            strokeWidth={STRIPE_WIDTH}
            strokeLinecap="round"
            strokeLinejoin="round"
            opacity={STRIPE_OPACITY}
          />
        ))}
      </g>
    </svg>
  );
}

export const TubeLineStripes = memo(TubeLineStripesComponent);
