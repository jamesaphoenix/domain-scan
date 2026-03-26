interface CoverageOverlayProps {
  coveragePercent: number;
  unmatchedCount: number;
  totalEntities: number;
  matchedEntities: number;
}

/** Clamp coverage percentage to [0, 100] for progress bar width. */
export function clampCoverage(coveragePercent: number): number {
  return Math.min(coveragePercent, 100);
}

/** Return the Tailwind bar color class based on clamped percentage. */
export function getBarColorClass(clampedPct: number): string {
  if (clampedPct >= 80) return "bg-emerald-500";
  if (clampedPct >= 50) return "bg-amber-500";
  return "bg-red-500";
}

/** Determine whether the overlay should be visible. */
export function shouldShowOverlay(totalEntities: number, unmatchedCount: number): boolean {
  return !(totalEntities === 0 && unmatchedCount === 0);
}

export function CoverageOverlay({
  coveragePercent,
  unmatchedCount,
  totalEntities,
  matchedEntities,
}: CoverageOverlayProps) {
  if (!shouldShowOverlay(totalEntities, unmatchedCount)) return null;

  const pct = clampCoverage(coveragePercent);
  const barColor = getBarColorClass(pct);

  return (
    <div className="absolute top-3 right-3 z-10 bg-slate-900/90 backdrop-blur-sm border border-slate-700/50 rounded-lg p-3 w-56">
      <div className="text-[11px] text-slate-400 uppercase tracking-wider font-medium mb-2">
        Match Coverage
      </div>

      {/* Progress bar */}
      <div className="h-2 bg-slate-800 rounded-full overflow-hidden mb-2">
        <div
          className={`h-full rounded-full transition-all duration-500 ${barColor}`}
          style={{ width: `${pct}%` }}
        />
      </div>

      {/* Stats */}
      <div className="flex items-center justify-between text-xs">
        <span className="text-slate-300 font-medium tabular-nums">
          {coveragePercent.toFixed(1)}%
        </span>
        <span className="text-slate-500">
          {matchedEntities} / {totalEntities} entities
        </span>
      </div>

      {/* Unmatched count */}
      {unmatchedCount > 0 && (
        <div className="mt-2 flex items-center gap-1.5 text-xs text-amber-400">
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
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z"
            />
          </svg>
          <span>{unmatchedCount} unmatched entities</span>
        </div>
      )}
    </div>
  );
}
