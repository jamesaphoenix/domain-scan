import { useEffect, useState, useCallback } from "react";
import type { SubsystemDetail, EntitySummary, DomainDef } from "../types";

interface SubsystemDrillInProps {
  subsystemId: string;
  domains: Record<string, DomainDef>;
  onBack: () => void;
  onOpenFile: (filePath: string, line: number) => void;
  getSubsystemDetail: (id: string) => Promise<SubsystemDetail>;
  getSubsystemEntities: (id: string) => Promise<EntitySummary[]>;
}

const KIND_BADGES: Record<string, { label: string; color: string }> = {
  interface: { label: "I", color: "bg-blue-500/20 text-blue-300 border-blue-500/30" },
  service: { label: "S", color: "bg-purple-500/20 text-purple-300 border-purple-500/30" },
  class: { label: "C", color: "bg-green-500/20 text-green-300 border-green-500/30" },
  function: { label: "F", color: "bg-yellow-500/20 text-yellow-300 border-yellow-500/30" },
  schema: { label: "D", color: "bg-orange-500/20 text-orange-300 border-orange-500/30" },
  impl: { label: "M", color: "bg-pink-500/20 text-pink-300 border-pink-500/30" },
  type_alias: { label: "T", color: "bg-cyan-500/20 text-cyan-300 border-cyan-500/30" },
  method: { label: "m", color: "bg-slate-500/20 text-slate-300 border-slate-500/30" },
};

const STATUS_COLORS: Record<string, string> = {
  built: "text-emerald-400",
  rebuild: "text-amber-400",
  new: "text-sky-400",
  boilerplate: "text-slate-400",
  unbuilt: "text-yellow-400",
  error: "text-red-400",
};

export function SubsystemDrillIn({
  subsystemId,
  domains,
  onBack,
  onOpenFile,
  getSubsystemDetail,
  getSubsystemEntities,
}: SubsystemDrillInProps) {
  const [detail, setDetail] = useState<SubsystemDetail | null>(null);
  const [entities, setEntities] = useState<EntitySummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    (async () => {
      try {
        const [d, e] = await Promise.all([
          getSubsystemDetail(subsystemId),
          getSubsystemEntities(subsystemId),
        ]);
        if (!cancelled) {
          setDetail(d);
          setEntities(e);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [subsystemId, getSubsystemDetail, getSubsystemEntities]);

  const handleEntityClick = useCallback(
    (entity: EntitySummary) => {
      onOpenFile(entity.file, entity.line);
    },
    [onOpenFile],
  );

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-slate-400 text-sm">Loading subsystem...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <p className="text-red-400 text-sm mb-3">{error}</p>
          <button
            onClick={onBack}
            className="text-xs text-blue-400 hover:text-blue-300"
          >
            Back to tube map
          </button>
        </div>
      </div>
    );
  }

  if (!detail) return null;

  const domainDef = domains[detail.domain];
  const domainColor = domainDef?.color ?? "#6b7280";

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Header */}
      <div
        className="px-6 py-4 border-b border-slate-800/50 flex-shrink-0"
        style={{
          background: `linear-gradient(135deg, ${domainColor}10, transparent)`,
        }}
      >
        <div className="flex items-center gap-3 mb-2">
          <button
            onClick={onBack}
            className="text-slate-400 hover:text-slate-200 transition-colors"
          >
            <svg
              className="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 19l-7-7 7-7"
              />
            </svg>
          </button>
          <h2 className="text-lg font-medium text-slate-100">{detail.name}</h2>
          <span
            className="text-xs px-2 py-0.5 rounded-full border"
            style={{
              borderColor: `${domainColor}40`,
              color: domainColor,
              backgroundColor: `${domainColor}10`,
            }}
          >
            {domainDef?.label ?? detail.domain}
          </span>
          <span
            className={`text-xs ${STATUS_COLORS[detail.status] ?? "text-slate-400"}`}
          >
            {detail.status}
          </span>
        </div>
        <div className="text-xs text-slate-500">
          <button
            onClick={() => onOpenFile(detail.file_path, 1)}
            className="hover:text-slate-300 transition-colors"
          >
            {detail.file_path}
          </button>
        </div>
        <div className="flex gap-4 mt-2 text-xs text-slate-400">
          {detail.interfaces.length > 0 && (
            <span>{detail.interfaces.length} interfaces</span>
          )}
          {detail.operations.length > 0 && (
            <span>{detail.operations.length} operations</span>
          )}
          {detail.tables.length > 0 && (
            <span>{detail.tables.length} tables</span>
          )}
          {detail.events.length > 0 && (
            <span>{detail.events.length} events</span>
          )}
          {detail.dependencies.length > 0 && (
            <span>{detail.dependencies.length} dependencies</span>
          )}
        </div>
      </div>

      {/* Entity grid */}
      <div className="flex-1 overflow-y-auto p-4">
        {entities.length === 0 ? (
          <div className="text-center text-slate-500 text-sm py-8">
            No matched entities found for this subsystem
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {entities.map((entity, idx) => {
              const badge = KIND_BADGES[entity.kind] ?? {
                label: "?",
                color: "bg-slate-500/20 text-slate-300 border-slate-500/30",
              };
              return (
                <button
                  key={`${entity.name}-${entity.file}-${idx}`}
                  onClick={() => handleEntityClick(entity)}
                  className="text-left p-3 rounded-lg border border-slate-700/50 bg-slate-800/40
                             hover:bg-slate-800/80 hover:border-slate-600/50 transition-all group"
                >
                  <div className="flex items-center gap-2 mb-1">
                    <span
                      className={`w-5 h-5 flex items-center justify-center rounded text-[10px] font-bold border ${badge.color}`}
                    >
                      {badge.label}
                    </span>
                    <span className="text-sm text-slate-200 font-medium truncate">
                      {entity.name}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 text-[11px] text-slate-500">
                    <span>{entity.language}</span>
                    <span className="text-slate-700">|</span>
                    <span className="truncate group-hover:text-slate-400 transition-colors">
                      {entity.file}:{entity.line}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 mt-1 text-[11px]">
                    <span
                      className={
                        STATUS_COLORS[entity.build_status] ?? "text-slate-400"
                      }
                    >
                      {entity.build_status}
                    </span>
                    <span className="text-slate-600">
                      {entity.confidence} confidence
                    </span>
                  </div>
                </button>
              );
            })}
          </div>
        )}

        {/* Children subsystems */}
        {detail.children.length > 0 && (
          <div className="mt-6">
            <h3 className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-3">
              Child Subsystems
            </h3>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {detail.children.map((child) => (
                <div
                  key={child.id}
                  className="p-3 rounded-lg border border-slate-700/50 bg-slate-800/40"
                >
                  <div className="text-sm text-slate-200 font-medium mb-1">
                    {child.name}
                  </div>
                  <div className="text-[11px] text-slate-500">
                    {child.interfaces.length > 0 && (
                      <span className="mr-2">
                        {child.interfaces.length} ifaces
                      </span>
                    )}
                    {child.operations.length > 0 && (
                      <span className="mr-2">
                        {child.operations.length} ops
                      </span>
                    )}
                    {child.tables.length > 0 && (
                      <span className="mr-2">
                        {child.tables.length} tables
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
