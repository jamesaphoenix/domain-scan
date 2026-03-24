import type { DomainDef, TubeMapSubsystem } from "../types";

interface LegendProps {
  domains: Record<string, DomainDef>;
  subsystems: TubeMapSubsystem[];
  activeDomain: string;
  onDomainClick: (domain: string) => void;
}

function countByDomain(subsystems: TubeMapSubsystem[], domainId: string): number {
  return subsystems.filter((s) => s.domain === domainId).length;
}

export function Legend({
  domains,
  subsystems,
  activeDomain,
  onDomainClick,
}: LegendProps) {
  return (
    <div className="flex items-center gap-5 text-xs overflow-x-auto scrollbar-none">
      {/* Domain tube lines */}
      <div className="flex items-center gap-1 flex-shrink-0">
        {Object.entries(domains).map(([id, domain]) => {
          const count = countByDomain(subsystems, id);
          const isActive = activeDomain === id;
          const isDimmed = activeDomain !== "all" && !isActive;

          return (
            <button
              key={id}
              onClick={() => onDomainClick(isActive ? "all" : id)}
              className={`flex items-center gap-0 py-0.5 rounded transition-all duration-150
                cursor-pointer select-none group
                ${isDimmed ? "opacity-30" : "opacity-100"}
                ${isActive ? "bg-slate-800/60" : "hover:bg-slate-800/40"}
              `}
              title={`${domain.label} — ${count} subsystem${count !== 1 ? "s" : ""}`}
            >
              {/* Thick colored tube-line segment */}
              <span
                className="font-mono font-bold text-sm leading-none tracking-tighter"
                style={{
                  color: domain.color,
                  textShadow: isActive
                    ? `0 0 8px ${domain.color}88`
                    : undefined,
                }}
              >
                {"━━━"}
              </span>
              {/* Domain name + station count */}
              <span
                className={`whitespace-nowrap mx-1 transition-colors duration-150 ${
                  isActive
                    ? "text-slate-100 font-medium"
                    : "text-slate-400 group-hover:text-slate-300"
                }`}
              >
                {domain.label}
              </span>
              <span
                className={`tabular-nums transition-colors duration-150 ${
                  isActive ? "text-slate-300" : "text-slate-600"
                }`}
              >
                ({count})
              </span>
            </button>
          );
        })}
      </div>

      {/* Divider */}
      <span className="w-px h-4 bg-slate-700 flex-shrink-0" />

      {/* Connection types */}
      <div className="flex items-center gap-3 flex-shrink-0">
        <span className="text-slate-600 font-medium uppercase tracking-wider text-[10px]">
          Links
        </span>
        <span className="flex items-center gap-1.5">
          <span className="font-mono text-slate-400 text-sm leading-none">{"━━"}</span>
          <span className="text-slate-500">depends</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="font-mono text-slate-500 text-sm leading-none">{"╌╌"}</span>
          <span className="text-slate-500">uses</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="font-mono text-slate-500 text-sm leading-none">{"┈┈"}</span>
          <span className="text-slate-500">triggers</span>
        </span>
      </div>

      {/* Divider */}
      <span className="w-px h-4 bg-slate-700 flex-shrink-0" />

      {/* Status indicators */}
      <div className="flex items-center gap-3 flex-shrink-0">
        <span className="text-slate-600 font-medium uppercase tracking-wider text-[10px]">
          Status
        </span>
        <span className="flex items-center gap-1.5">
          <span className="text-emerald-400 text-sm leading-none">{"●"}</span>
          <span className="text-slate-500">built</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="text-amber-400 text-sm leading-none">{"○"}</span>
          <span className="text-slate-500">rebuild</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="text-sky-400 text-sm leading-none">{"✦"}</span>
          <span className="text-slate-500">new</span>
        </span>
      </div>
    </div>
  );
}
