import React, { useState, useCallback } from "react";
import {
  getSmoothStepPath,
  EdgeLabelRenderer,
  type EdgeProps,
  type Edge,
} from "@xyflow/react";
import type { ConnectionType } from "../types";
import { EdgeTooltip, type EdgeTooltipData } from "./EdgeTooltip";

export interface BundledConnection {
  fromName: string;
  toName: string;
  label: string;
  type: ConnectionType;
}

export interface DependencyEdgeData extends Record<string, unknown> {
  connectionType: ConnectionType;
  label: string;
  sourceName: string;
  targetName: string;
  sourceInterfaces: string[];
  targetInterfaces: string[];
  sourceDomainColor: string;
  targetDomainColor: string;
  /** Number of edges in this bundle (1 = normal edge, >1 = bundled) */
  bundleCount?: number;
  /** Individual connections in a bundle */
  bundledConnections?: BundledConnection[];
}

type DependencyEdgeType = Edge<DependencyEdgeData, "dependency">;

function getTubeStyle(
  connType: ConnectionType,
  sourceDomainColor: string,
  isHovered: boolean,
) {
  const base = {
    depends_on: {
      color: sourceDomainColor,
      width: 5,
      dasharray: undefined as string | undefined,
      opacity: 1,
      hoverWidth: 7,
    },
    uses: {
      color: sourceDomainColor,
      width: 2,
      dasharray: undefined as string | undefined,
      opacity: 0.6,
      hoverWidth: 3.5,
    },
    triggers: {
      color: "#f59e0b",
      width: 5,
      dasharray: "12 6",
      opacity: 1,
      hoverWidth: 7,
    },
  }[connType];

  return {
    stroke: base.color,
    strokeWidth: isHovered ? base.hoverWidth : base.width,
    strokeDasharray: base.dasharray,
    opacity: isHovered ? 1 : base.opacity,
  };
}

export function DependencyEdge({
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  data,
  selected,
}: EdgeProps<DependencyEdgeType>) {
  const edgeData = data as DependencyEdgeData;
  const [hovered, setHovered] = useState(false);
  const [tooltipPos, setTooltipPos] = useState({ x: 0, y: 0 });

  const connType = edgeData.connectionType ?? "depends_on";
  const isBundle = (edgeData.bundleCount ?? 1) > 1;
  const style = getTubeStyle(connType, edgeData.sourceDomainColor, hovered);
  // Thicker stroke for bundled edges
  if (isBundle) {
    style.strokeWidth = hovered ? 10 : 8;
  }

  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    borderRadius: 8,
  });

  const handleMouseEnter = useCallback((e: React.MouseEvent) => {
    setHovered(true);
    setTooltipPos({ x: e.clientX, y: e.clientY });
  }, []);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    setTooltipPos({ x: e.clientX, y: e.clientY });
  }, []);

  const handleMouseLeave = useCallback(() => {
    setHovered(false);
  }, []);

  const tooltipData: EdgeTooltipData = {
    sourceName: edgeData.sourceName,
    targetName: edgeData.targetName,
    connectionType: connType,
    label: edgeData.label,
    sourceInterfaces: edgeData.sourceInterfaces ?? [],
    targetInterfaces: edgeData.targetInterfaces ?? [],
    sourceDomainColor: edgeData.sourceDomainColor,
    targetDomainColor: edgeData.targetDomainColor,
    bundleCount: edgeData.bundleCount,
    bundledConnections: edgeData.bundledConnections,
  };

  return (
    <>
      {/* Wide invisible hit area for hover interaction */}
      <path
        d={edgePath}
        fill="none"
        stroke="transparent"
        strokeWidth={isBundle ? 32 : 24}
        onMouseEnter={handleMouseEnter}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
        className="cursor-pointer"
      />

      {/* Glow layer on hover */}
      {hovered && (
        <path
          d={edgePath}
          fill="none"
          stroke={style.stroke}
          strokeWidth={style.strokeWidth + 6}
          strokeOpacity={0.12}
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeDasharray={style.strokeDasharray}
        />
      )}

      {/* Main tube line */}
      <path
        d={edgePath}
        fill="none"
        stroke={style.stroke}
        strokeWidth={style.strokeWidth}
        strokeDasharray={style.strokeDasharray}
        strokeLinecap="round"
        strokeLinejoin="round"
        opacity={hovered || selected ? 1 : style.opacity}
        style={{
          transition: "stroke-width 0.15s ease, opacity 0.15s ease",
        }}
      />

      {/* Bundle count badge at edge midpoint */}
      {isBundle && (
        <EdgeLabelRenderer>
          <div
            className="nodrag nopan pointer-events-none absolute"
            style={{
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
            }}
          >
            <div
              className="flex items-center justify-center rounded-full text-[10px] font-bold text-white shadow-md border border-white/20"
              style={{
                background: style.stroke,
                minWidth: 22,
                height: 22,
                padding: "0 5px",
              }}
            >
              {edgeData.bundleCount}
            </div>
          </div>
        </EdgeLabelRenderer>
      )}

      {/* Tooltip on hover */}
      {hovered && (
        <EdgeLabelRenderer>
          <EdgeTooltip data={tooltipData} x={tooltipPos.x} y={tooltipPos.y} />
        </EdgeLabelRenderer>
      )}
    </>
  );
}
