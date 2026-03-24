import { forwardRef } from "react";
import type { DomainDef, TubeMapSubsystem } from "../types";
import type { DependencyDirection } from "../hooks/useTubeMapState";

interface TubeMapSearchBarProps {
  query: string;
  onQueryChange: (query: string) => void;
  domainFilter: string;
  onDomainFilterChange: (domain: string) => void;
  statusFilter: string;
  onStatusFilterChange: (status: string) => void;
  domains: Record<string, DomainDef>;
  subsystems: TubeMapSubsystem[];
  focusedSubsystemId: string | null;
  onFocusedSubsystemChange: (id: string | null) => void;
  dependencyDirection: DependencyDirection;
  onDependencyDirectionChange: (direction: DependencyDirection) => void;
}

export const TubeMapSearchBar = forwardRef<
  HTMLInputElement,
  TubeMapSearchBarProps
>(function TubeMapSearchBar(
  {
    query,
    onQueryChange,
    domainFilter,
    onDomainFilterChange,
    statusFilter,
    onStatusFilterChange,
    domains,
    subsystems,
    focusedSubsystemId,
    onFocusedSubsystemChange,
    dependencyDirection,
    onDependencyDirectionChange,
  },
  ref,
) {
  return (
    <div className="flex items-center gap-3">
      {/* Search input */}
      <div className="relative">
        <svg
          className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-slate-500"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
          />
        </svg>
        <input
          ref={ref}
          type="text"
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          placeholder="Search subsystems...  (/)"
          className="bg-slate-800 border border-slate-700 rounded-md pl-8 pr-3 py-1.5 text-sm
                     text-slate-200 placeholder-slate-500 focus:outline-none focus:border-slate-500
                     focus:ring-1 focus:ring-slate-500 w-56"
        />
      </div>

      {/* Domain filter */}
      <select
        value={domainFilter}
        onChange={(e) => onDomainFilterChange(e.target.value)}
        className="bg-slate-800 border border-slate-700 rounded-md px-2.5 py-1.5 text-sm
                   text-slate-200 focus:outline-none focus:border-slate-500 cursor-pointer"
      >
        <option value="all">All Domains</option>
        {Object.entries(domains).map(([id, domain]) => (
          <option key={id} value={id}>
            {domain.label}
          </option>
        ))}
      </select>

      {/* Status filter */}
      <select
        value={statusFilter}
        onChange={(e) => onStatusFilterChange(e.target.value)}
        className="bg-slate-800 border border-slate-700 rounded-md px-2.5 py-1.5 text-sm
                   text-slate-200 focus:outline-none focus:border-slate-500 cursor-pointer"
      >
        <option value="all">All Statuses</option>
        <option value="built">Built</option>
        <option value="rebuild">Rebuild</option>
        <option value="new">New</option>
        <option value="boilerplate">Boilerplate</option>
      </select>

      <span className="text-slate-700">|</span>

      {/* Dependency trace: subsystem selector */}
      <select
        value={focusedSubsystemId ?? "none"}
        onChange={(e) => {
          const val = e.target.value;
          onFocusedSubsystemChange(val === "none" ? null : val);
        }}
        className={`border rounded-md px-2.5 py-1.5 text-sm
                   focus:outline-none cursor-pointer transition-colors duration-150
                   ${
                     focusedSubsystemId
                       ? "bg-purple-950/60 border-purple-500/50 text-purple-200 focus:border-purple-400"
                       : "bg-slate-800 border-slate-700 text-slate-200 focus:border-slate-500"
                   }`}
      >
        <option value="none">Dep. Trace: Off</option>
        {subsystems.map((s) => (
          <option key={s.id} value={s.id}>
            {s.name}
          </option>
        ))}
      </select>

      {/* Direction toggle - only visible when a subsystem is focused */}
      {focusedSubsystemId && (
        <div className="flex items-center rounded-md border border-purple-500/30 overflow-hidden">
          {(["upstream", "both", "downstream"] as DependencyDirection[]).map(
            (dir) => (
              <button
                key={dir}
                onClick={() => onDependencyDirectionChange(dir)}
                className={`px-2.5 py-1.5 text-[11px] font-medium transition-colors duration-150
                           ${
                             dependencyDirection === dir
                               ? "bg-purple-600/40 text-purple-200"
                               : "bg-slate-800/80 text-slate-400 hover:bg-slate-700/80 hover:text-slate-300"
                           }`}
              >
                {dir === "upstream"
                  ? "Upstream"
                  : dir === "downstream"
                    ? "Downstream"
                    : "Both"}
              </button>
            ),
          )}
        </div>
      )}
    </div>
  );
});
