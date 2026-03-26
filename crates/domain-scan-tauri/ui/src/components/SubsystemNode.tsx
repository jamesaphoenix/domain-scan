import React, { memo, useCallback } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";

export interface SubsystemNodeData {
  label: string;
  description: string;
  domain: string;
  domainColor: string;
  status: string;
  interfaces: string[];
  tables: string[];
  operations: string[];
  events: string[];
  hasChildren: boolean;
  filePath: string;
  dependencyCount: number;
  matchedEntityCount: number;
  dimmed: boolean;
  childInterfaceCount: number;
  childTableCount: number;
  childOperationCount: number;
  childEventCount: number;
  onDrillIn: () => void;
  onOpenFile: (filePath: string) => void;
  onFocusDependency?: () => void;
  [key: string]: unknown;
}

const statusConfig: Record<
  string,
  { label: string; bg: string; text: string; icon: string; ring: string }
> = {
  built: {
    label: "Built",
    bg: "bg-emerald-950/80",
    text: "text-emerald-300",
    icon: "\u2713",
    ring: "ring-emerald-500/30",
  },
  rebuild: {
    label: "Rebuild",
    bg: "bg-amber-950/80",
    text: "text-amber-300",
    icon: "\u21BB",
    ring: "ring-amber-500/30",
  },
  new: {
    label: "New",
    bg: "bg-sky-950/80",
    text: "text-sky-300",
    icon: "\u2726",
    ring: "ring-sky-500/30",
  },
  boilerplate: {
    label: "Boilerplate",
    bg: "bg-slate-800/80",
    text: "text-slate-400",
    icon: "\u25A2",
    ring: "ring-slate-500/30",
  },
};

function DependencyIcon({ color }: { color: string }) {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="shrink-0"
    >
      <circle cx="12" cy="5" r="3" />
      <circle cx="5" cy="19" r="3" />
      <circle cx="19" cy="19" r="3" />
      <line x1="12" y1="8" x2="5" y2="16" />
      <line x1="12" y1="8" x2="19" y2="16" />
    </svg>
  );
}

function ChevronRightIcon({ color }: { color: string }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <polyline points="9 18 15 12 9 6" />
    </svg>
  );
}

function EditorIcon({ color }: { color: string }) {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="shrink-0"
    >
      <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
      <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
    </svg>
  );
}

function SubsystemNodeComponent({ data }: NodeProps) {
  const d = data as SubsystemNodeData;
  const status = statusConfig[d.status] ?? statusConfig.new;
  const isDimmed = d.dimmed;

  const handleBodyClick = useCallback(() => {
    if (isDimmed && d.onFocusDependency) {
      d.onFocusDependency();
      return;
    }
    d.onDrillIn();
  }, [d, isDimmed]);

  return (
    <div
      className={`group relative max-w-[340px] transition-all duration-300 ease-out
                 ${isDimmed ? "hover:scale-100" : "hover:scale-[1.02]"}`}
      style={
        {
          "--domain-color": d.domainColor,
          "--domain-color-20": `${d.domainColor}33`,
          "--domain-color-40": `${d.domainColor}66`,
          opacity: isDimmed ? 0.2 : 1,
          transition: "opacity 0.3s ease, transform 0.15s ease",
          filter: isDimmed ? "saturate(0.3)" : "none",
        } as React.CSSProperties
      }
    >
      <div
        className="relative rounded-lg border border-slate-700/80 overflow-hidden cursor-pointer
                   transition-colors duration-150 ease-out
                   hover:border-[var(--domain-color-40)]"
        style={{
          borderLeft: `4px solid ${d.domainColor}`,
          background: `linear-gradient(160deg, ${d.domainColor}08 0%, #0f172a 30%, #1e293b 100%)`,
        }}
        onClick={handleBodyClick}
      >
        <Handle
          type="target"
          position={Position.Top}
          className="!w-3 !h-1.5 !rounded-sm !border-0 !-top-[2px]"
          style={{ background: d.domainColor }}
        />

        <div className="px-3 pt-3 pb-2">
          <div className="flex items-center justify-between mb-2">
            <div
              className={`inline-flex items-center gap-1 text-[10px] font-semibold px-2 py-0.5 rounded ring-1 ${status.bg} ${status.text} ${status.ring}`}
            >
              <span className="text-[10px] leading-none">{status.icon}</span>
              {status.label}
            </div>

            <span
              className="text-[9px] font-medium tracking-wide uppercase px-1.5 py-0.5 rounded"
              style={{
                color: d.domainColor,
                background: `${d.domainColor}15`,
              }}
            >
              {d.domain}
            </span>
          </div>

          <h3
            className="text-[14px] font-bold text-slate-50 leading-tight mb-0.5 tracking-[-0.01em]
                       cursor-copy nodrag nopan"
            onClick={(e: React.MouseEvent) => {
              e.stopPropagation();
              navigator.clipboard.writeText(d.label);
            }}
            title={`${d.label} (click to copy)`}
          >
            {d.label}
          </h3>

          <p className="text-[11px] text-slate-400 leading-snug">
            {d.description}
          </p>

          {d.matchedEntityCount > 0 && (
            <div className="mt-1.5">
              <span className="text-[10px] font-medium px-1.5 py-[1px] rounded bg-slate-800 text-slate-300 border border-slate-600/50">
                {d.matchedEntityCount}{" "}
                {d.matchedEntityCount === 1 ? "entity" : "entities"} matched
              </span>
            </div>
          )}

          {d.hasChildren &&
            (d.childInterfaceCount > 0 ||
              d.childOperationCount > 0 ||
              d.childTableCount > 0 ||
              d.childEventCount > 0) && (
              <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
                {d.childInterfaceCount > 0 && (
                  <span
                    className="text-[10px] font-medium px-1.5 py-[1px] rounded"
                    style={{
                      color: d.domainColor,
                      background: `${d.domainColor}15`,
                      border: `1px solid ${d.domainColor}20`,
                    }}
                  >
                    {d.childInterfaceCount}{" "}
                    {d.childInterfaceCount === 1 ? "interface" : "interfaces"}
                  </span>
                )}
                {d.childOperationCount > 0 && (
                  <>
                    <span className="text-[10px] text-slate-600">
                      {"\u00B7"}
                    </span>
                    <span className="text-[10px] font-medium px-1.5 py-[1px] rounded bg-emerald-950/60 text-emerald-400 border border-emerald-500/20">
                      {d.childOperationCount}{" "}
                      {d.childOperationCount === 1
                        ? "operation"
                        : "operations"}
                    </span>
                  </>
                )}
                {d.childTableCount > 0 && (
                  <>
                    <span className="text-[10px] text-slate-600">
                      {"\u00B7"}
                    </span>
                    <span className="text-[10px] font-medium px-1.5 py-[1px] rounded bg-amber-950/60 text-amber-400 border border-amber-500/20">
                      {d.childTableCount}{" "}
                      {d.childTableCount === 1 ? "table" : "tables"}
                    </span>
                  </>
                )}
                {d.childEventCount > 0 && (
                  <>
                    <span className="text-[10px] text-slate-600">
                      {"\u00B7"}
                    </span>
                    <span className="text-[10px] font-medium px-1.5 py-[1px] rounded bg-purple-950/60 text-purple-400 border border-purple-500/20">
                      {d.childEventCount}{" "}
                      {d.childEventCount === 1 ? "event" : "events"}
                    </span>
                  </>
                )}
              </div>
            )}
        </div>

        {d.interfaces.length > 0 && (
          <div className="px-3 pb-2 nodrag nopan">
            <div className="text-[9px] font-semibold uppercase tracking-wider text-slate-500 mb-1">
              Interfaces
            </div>
            <div className="flex flex-wrap gap-1">
              {d.interfaces.map((iface) => (
                <span
                  key={iface}
                  className="text-[10px] font-mono font-medium px-1.5 py-[2px] rounded
                             transition-colors duration-100 cursor-pointer pointer-events-auto"
                  style={{
                    color: d.domainColor,
                    background: `${d.domainColor}15`,
                    border: `1px solid ${d.domainColor}25`,
                  }}
                  onMouseEnter={(e: React.MouseEvent<HTMLSpanElement>) => {
                    const el = e.currentTarget;
                    el.style.background = `${d.domainColor}30`;
                    el.style.borderColor = `${d.domainColor}50`;
                    el.style.color = "#f1f5f9";
                  }}
                  onMouseLeave={(e: React.MouseEvent<HTMLSpanElement>) => {
                    const el = e.currentTarget;
                    el.style.background = `${d.domainColor}15`;
                    el.style.borderColor = `${d.domainColor}25`;
                    el.style.color = d.domainColor;
                  }}
                  onClick={(e: React.MouseEvent<HTMLSpanElement>) => {
                    e.stopPropagation();
                    navigator.clipboard.writeText(iface);
                  }}
                  title={`${iface} (click to copy)`}
                >
                  {iface}
                </span>
              ))}
            </div>
          </div>
        )}

        {d.operations.length > 0 && (
          <div className="px-3 pb-2 nodrag nopan">
            <div className="text-[9px] font-semibold uppercase tracking-wider text-slate-500 mb-1">
              Operations
            </div>
            <div className="flex flex-wrap gap-1">
              {d.operations.map((op) => (
                <span
                  key={op}
                  className="text-[10px] font-mono font-medium px-1.5 py-[2px] rounded
                             bg-emerald-950/60 text-emerald-400 border border-emerald-500/25
                             transition-colors duration-100 cursor-pointer pointer-events-auto
                             hover:text-emerald-200 hover:border-emerald-400/50 hover:bg-emerald-900/60"
                  onClick={(e: React.MouseEvent<HTMLSpanElement>) => {
                    e.stopPropagation();
                    navigator.clipboard.writeText(op);
                  }}
                  title={`${op} (click to copy)`}
                >
                  {op}()
                </span>
              ))}
            </div>
          </div>
        )}

        {d.tables.length > 0 && (
          <div className="px-3 pb-2 nodrag nopan">
            <div className="text-[9px] font-semibold uppercase tracking-wider text-slate-500 mb-1">
              Tables
            </div>
            <div className="flex flex-wrap gap-1">
              {d.tables.map((table) => (
                <span
                  key={table}
                  className="text-[10px] font-mono px-1.5 py-[2px] rounded
                             bg-amber-950/60 text-amber-400 border border-amber-500/25
                             transition-colors duration-100 cursor-pointer pointer-events-auto
                             hover:text-amber-200 hover:border-amber-400/50 hover:bg-amber-900/60"
                  onClick={(e: React.MouseEvent<HTMLSpanElement>) => {
                    e.stopPropagation();
                    navigator.clipboard.writeText(table);
                  }}
                  title={`${table} (click to copy)`}
                >
                  {table}
                </span>
              ))}
            </div>
          </div>
        )}

        {d.events.length > 0 && (
          <div className="px-3 pb-2 nodrag nopan">
            <div className="text-[9px] font-semibold uppercase tracking-wider text-slate-500 mb-1">
              Events
            </div>
            <div className="flex flex-wrap gap-1">
              {d.events.map((evt) => (
                <span
                  key={evt}
                  className="text-[10px] font-mono font-medium px-1.5 py-[2px] rounded
                             bg-purple-950/60 text-purple-400 border border-purple-500/25
                             transition-colors duration-100 cursor-pointer pointer-events-auto
                             hover:text-purple-200 hover:border-purple-400/50 hover:bg-purple-900/60"
                  onClick={(e: React.MouseEvent<HTMLSpanElement>) => {
                    e.stopPropagation();
                    navigator.clipboard.writeText(evt);
                  }}
                  title={`${evt} (click to copy)`}
                >
                  {evt}
                </span>
              ))}
            </div>
          </div>
        )}

        {d.dependencyCount > 0 && (
          <div className="px-3 pb-2">
            <div className="flex items-center gap-1.5 text-[11px] text-slate-500">
              <DependencyIcon color="#64748b" />
              <span>
                Depends on{" "}
                <span className="text-slate-400 font-medium">
                  {d.dependencyCount}
                </span>{" "}
                {d.dependencyCount === 1 ? "subsystem" : "subsystems"}
              </span>
            </div>
          </div>
        )}

        <div className="mx-3 border-t border-slate-700/50" />

        <div className="px-3 py-2 flex items-center justify-between nodrag nopan">
          {d.filePath ? (
            <button
              className="flex items-center gap-1 text-[11px] text-slate-500 hover:text-slate-300
                         transition-colors duration-150"
              onClick={(e: React.MouseEvent<HTMLButtonElement>) => {
                e.stopPropagation();
                d.onOpenFile(d.filePath);
              }}
              title={`Open ${d.filePath} in editor`}
            >
              <EditorIcon color="currentColor" />
              <span>Open in Editor</span>
            </button>
          ) : (
            <div />
          )}
          <div
            className="flex items-center gap-1 text-[11px] font-medium transition-transform duration-150
                       group-hover:translate-x-0.5"
            style={{ color: d.domainColor }}
          >
            {d.hasChildren ? "Drill in" : "View details"}
            <ChevronRightIcon color={d.domainColor} />
          </div>
        </div>

        <Handle
          type="source"
          position={Position.Bottom}
          className="!w-3 !h-1.5 !rounded-sm !border-0 !-bottom-[2px]"
          style={{ background: d.domainColor }}
        />
      </div>
    </div>
  );
}

export const SubsystemNode = memo(SubsystemNodeComponent);
