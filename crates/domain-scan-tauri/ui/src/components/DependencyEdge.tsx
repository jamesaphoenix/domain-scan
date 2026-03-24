import React, { useState, useCallback } from "react";
import {
  getSmoothStepPath,
  EdgeLabelRenderer,
  type EdgeProps,
  type Edge,
} from "@xyflow/react";
import type { ConnectionType } from "../types";
import { EdgeTooltip, type EdgeTooltipData } from "./EdgeTooltip";

export interface DependencyEdgeData extends Record<string, unknown> {
  connectionType: ConnectionType;
  label: string;
  sourceName: string;
  targetName: string;
  sourceInterfaces: string[];
  targetInterfaces: string[];
  sourceDomainColor: string;
  targetDomainColor: string;
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
  const style = getTubeStyle(connType, edgeData.sourceDomainColor, hovered);

  const [edgePath] = getSmoothStepPath({
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
  };

  return (
    <>
      {/* Wide invisible hit area for hover interaction */}
      <path
        d={edgePath}
        fill="none"
        stroke="transparent"
        strokeWidth={24}
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

      {/* Tooltip on hover */}
      {hovered && (
        <EdgeLabelRenderer>
          <EdgeTooltip data={tooltipData} x={tooltipPos.x} y={tooltipPos.y} />
        </EdgeLabelRenderer>
      )}
    </>
  );
}
