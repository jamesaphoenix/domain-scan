import { memo } from "react";
import type { DomainDef } from "../types";

interface TubeMapStatusBarProps {
  zoom: number;
  visibleNodes: number;
  totalNodes: number;
  domainFilter: string;
  statusFilter: string;
  domains: Record<string, DomainDef>;
  coveragePercent: number;
  unmatchedCount: number;
  onToggleShortcuts: () => void;
}

function TubeMapStatusBarComponent({
  zoom,
  visibleNodes,
  totalNodes,
  domainFilter,
  statusFilter,
  domains,
  coveragePercent,
  unmatchedCount,
  onToggleShortcuts,
}: TubeMapStatusBarProps) {
  const zoomPercent = Math.round(zoom * 100);

  const filterParts: string[] = [];
  if (domainFilter !== "all") {
    const domainLabel = domains[domainFilter]?.label ?? domainFilter;
    filterParts.push(domainLabel);
  }
  if (statusFilter !== "all") {
    filterParts.push(
      statusFilter.charAt(0).toUpperCase() + statusFilter.slice(1),
    );
  }
  const filterText =
    filterParts.length > 0 ? filterParts.join(" + ") : "No filters";

  return (
    <div className="h-7 border-t border-slate-800 bg-slate-900/90 backdrop-blur-sm px-4 flex items-center justify-between text-[11px] text-slate-500 z-10 shrink-0">
      <div className="flex items-center gap-4">
        {/* Zoom */}
        <span className="flex items-center gap-1.5">
          <svg
            className="w-3 h-3"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v3m0 0v3m0-3h3m-3 0H7"
            />
          </svg>
          <span>{zoomPercent}%</span>
        </span>

        {/* Visible / Total */}
        <span className="flex items-center gap-1.5">
          <svg
            className="w-3 h-3"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4 6h16M4 12h16M4 18h16"
            />
          </svg>
          <span>
            {visibleNodes} / {totalNodes} nodes
          </span>
        </span>

        {/* Filter state */}
        <span className="flex items-center gap-1.5">
          <svg
            className="w-3 h-3"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"
            />
          </svg>
          <span className={filterParts.length > 0 ? "text-blue-400" : ""}>
            {filterText}
          </span>
        </span>

        {/* Coverage */}
        {coveragePercent > 0 && (
          <span className="flex items-center gap-1.5">
            <span className="text-emerald-400">
              {coveragePercent.toFixed(1)}% coverage
            </span>
          </span>
        )}

        {/* Unmatched */}
        {unmatchedCount > 0 && (
          <span className="text-amber-400">{unmatchedCount} unmatched</span>
        )}
      </div>

      <button
        onClick={onToggleShortcuts}
        className="text-slate-600 hover:text-slate-400 transition-colors cursor-pointer"
      >
        Press{" "}
        <kbd className="font-mono px-1 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-400 text-[10px]">
          ?
        </kbd>{" "}
        for shortcuts
      </button>
    </div>
  );
}

export const TubeMapStatusBar = memo(TubeMapStatusBarComponent);
