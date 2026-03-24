import type { ConnectionType } from "../types";
import type { BundledConnection } from "./DependencyEdge";

export interface EdgeTooltipData {
  sourceName: string;
  targetName: string;
  connectionType: ConnectionType;
  label: string;
  sourceInterfaces: string[];
  targetInterfaces: string[];
  sourceDomainColor: string;
  targetDomainColor: string;
  bundleCount?: number;
  bundledConnections?: BundledConnection[];
}

interface EdgeTooltipProps {
  data: EdgeTooltipData;
  x: number;
  y: number;
}

const typeBadgeConfig: Record<
  ConnectionType,
  { label: string; bg: string; text: string }
> = {
  depends_on: {
    label: "depends on",
    bg: "bg-slate-600/80",
    text: "text-slate-100",
  },
  uses: {
    label: "uses",
    bg: "bg-sky-800/80",
    text: "text-sky-100",
  },
  triggers: {
    label: "triggers",
    bg: "bg-amber-800/80",
    text: "text-amber-100",
  },
};

export function EdgeTooltip({ data, x, y }: EdgeTooltipProps) {
  const isBundle = (data.bundleCount ?? 1) > 1;

  if (isBundle && data.bundledConnections) {
    return (
      <div
        className="pointer-events-none fixed z-50"
        style={{
          left: x + 14,
          top: y - 6,
        }}
      >
        <div
          className="rounded-md border border-slate-600/60 bg-slate-900/95 backdrop-blur-sm
                      shadow-lg shadow-black/50 px-2.5 py-1.5 max-w-[320px]"
        >
          <div className="flex items-center gap-1.5 mb-1.5">
            <span
              className="text-[11px] font-semibold"
              style={{ color: data.sourceDomainColor }}
            >
              {data.sourceName}
            </span>
            <span className="text-[10px] text-slate-500">{"\u2192"}</span>
            <span
              className="text-[11px] font-semibold"
              style={{ color: data.targetDomainColor }}
            >
              {data.targetName}
            </span>
            <span className="text-[9px] font-mono font-medium px-1.5 py-px rounded bg-slate-600/80 text-slate-100">
              {data.bundleCount} connections
            </span>
          </div>
          <div className="max-h-[120px] overflow-y-auto space-y-0.5">
            {data.bundledConnections.map((conn, i) => {
              const connBadge = typeBadgeConfig[conn.type];
              return (
                <div
                  key={i}
                  className="flex items-center gap-1 text-[10px] text-slate-300"
                >
                  <span className="truncate max-w-[80px]">{conn.fromName}</span>
                  <span className="text-slate-500">{"\u2192"}</span>
                  <span className="truncate max-w-[80px]">{conn.toName}</span>
                  <span
                    className={`text-[8px] font-mono px-1 py-px rounded ${connBadge.bg} ${connBadge.text}`}
                  >
                    {connBadge.label}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    );
  }

  const badge = typeBadgeConfig[data.connectionType];

  return (
    <div
      className="pointer-events-none fixed z-50"
      style={{
        left: x + 14,
        top: y - 6,
      }}
    >
      <div
        className="rounded-md border border-slate-600/60 bg-slate-900/95 backdrop-blur-sm
                    shadow-lg shadow-black/50 px-2.5 py-1.5 max-w-[280px]"
      >
        <div className="flex items-center gap-1.5 flex-wrap">
          <span
            className="text-[11px] font-semibold truncate max-w-[90px]"
            style={{ color: data.sourceDomainColor }}
          >
            {data.sourceName}
          </span>
          <span className="text-[10px] text-slate-500">{"\u2192"}</span>
          <span
            className="text-[11px] font-semibold truncate max-w-[90px]"
            style={{ color: data.targetDomainColor }}
          >
            {data.targetName}
          </span>
          <span
            className={`text-[9px] font-mono font-medium px-1.5 py-px rounded ${badge.bg} ${badge.text}`}
          >
            {badge.label}
          </span>
        </div>

        {data.label && (
          <p className="text-[10px] text-slate-400 leading-snug mt-1 truncate">
            {data.label}
          </p>
        )}
      </div>
    </div>
  );
}
