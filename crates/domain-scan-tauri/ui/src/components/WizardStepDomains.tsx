import { useCallback } from "react";
import type { DomainDef, ManifestMeta } from "../types";

const DEFAULT_COLORS = [
  "#3b82f6", "#22c55e", "#f97316", "#a855f7",
  "#ef4444", "#eab308", "#06b6d4", "#ec4899",
  "#14b8a6", "#f59e0b", "#6366f1", "#84cc16",
];

interface WizardStepDomainsProps {
  domains: Record<string, DomainDef>;
  meta: ManifestMeta;
  onDomainsChange: (domains: Record<string, DomainDef>) => void;
  onMetaChange: (meta: ManifestMeta) => void;
  onBootstrap: () => Promise<void>;
  bootstrapping: boolean;
  hasData: boolean;
}

export function WizardStepDomains({
  domains,
  meta,
  onDomainsChange,
  onMetaChange,
  onBootstrap,
  bootstrapping,
  hasData,
}: WizardStepDomainsProps) {
  const domainEntries = Object.entries(domains);

  const handleAddDomain = useCallback(() => {
    const idx = domainEntries.length;
    const id = `domain-${idx + 1}`;
    onDomainsChange({
      ...domains,
      [id]: {
        label: `Domain ${idx + 1}`,
        color: DEFAULT_COLORS[idx % DEFAULT_COLORS.length],
      },
    });
  }, [domains, domainEntries.length, onDomainsChange]);

  const handleRemoveDomain = useCallback(
    (id: string) => {
      const next = { ...domains };
      delete next[id];
      onDomainsChange(next);
    },
    [domains, onDomainsChange],
  );

  const handleUpdateDomain = useCallback(
    (oldId: string, newId: string, def: DomainDef) => {
      const entries = Object.entries(domains);
      const next: Record<string, DomainDef> = {};
      for (const [k, v] of entries) {
        if (k === oldId) {
          next[newId] = def;
        } else {
          next[k] = v;
        }
      }
      onDomainsChange(next);
    },
    [domains, onDomainsChange],
  );

  return (
    <div className="p-6 max-w-3xl mx-auto">
      {/* Bootstrap CTA */}
      <div className="mb-8 p-4 rounded-lg border border-slate-700/50 bg-slate-800/30">
        <h3 className="text-sm font-medium text-slate-200 mb-2">
          Auto-detect from scan
        </h3>
        <p className="text-xs text-slate-400 mb-3">
          Run heuristic analysis on the scanned codebase to infer domains,
          subsystems, and connections automatically. You can then refine the
          results.
        </p>
        <button
          onClick={onBootstrap}
          disabled={bootstrapping}
          className="px-4 py-1.5 rounded text-xs font-medium bg-blue-600 hover:bg-blue-500 disabled:bg-blue-600/50 text-white transition-colors"
        >
          {bootstrapping
            ? "Analyzing..."
            : hasData
              ? "Re-analyze codebase"
              : "Analyze codebase"}
        </button>
      </div>

      {/* Project metadata */}
      <div className="mb-6">
        <h3 className="text-sm font-medium text-slate-200 mb-3">
          Project Info
        </h3>
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="block text-xs text-slate-400 mb-1">Name</label>
            <input
              type="text"
              value={meta.name}
              onChange={(e) => onMetaChange({ ...meta, name: e.target.value })}
              placeholder="my-project"
              className="w-full px-3 py-1.5 rounded bg-slate-800 border border-slate-700 text-sm text-slate-200 placeholder-slate-600 focus:border-blue-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="block text-xs text-slate-400 mb-1">
              Version
            </label>
            <input
              type="text"
              value={meta.version}
              onChange={(e) =>
                onMetaChange({ ...meta, version: e.target.value })
              }
              placeholder="1.0.0"
              className="w-full px-3 py-1.5 rounded bg-slate-800 border border-slate-700 text-sm text-slate-200 placeholder-slate-600 focus:border-blue-500 focus:outline-none"
            />
          </div>
          <div className="col-span-2">
            <label className="block text-xs text-slate-400 mb-1">
              Description
            </label>
            <input
              type="text"
              value={meta.description}
              onChange={(e) =>
                onMetaChange({ ...meta, description: e.target.value })
              }
              placeholder="Brief project description"
              className="w-full px-3 py-1.5 rounded bg-slate-800 border border-slate-700 text-sm text-slate-200 placeholder-slate-600 focus:border-blue-500 focus:outline-none"
            />
          </div>
        </div>
      </div>

      {/* Domain list */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-medium text-slate-200">
            Domains ({domainEntries.length})
          </h3>
          <button
            onClick={handleAddDomain}
            className="text-xs text-blue-400 hover:text-blue-300 transition-colors"
          >
            + Add domain
          </button>
        </div>

        {domainEntries.length === 0 ? (
          <p className="text-xs text-slate-500 py-4 text-center">
            No domains yet. Click "Analyze codebase" to auto-detect, or add
            manually.
          </p>
        ) : (
          <div className="space-y-2">
            {domainEntries.map(([id, def]) => (
              <div
                key={id}
                className="flex items-center gap-3 p-2.5 rounded-lg border border-slate-700/50 bg-slate-800/20"
              >
                {/* Color swatch */}
                <input
                  type="color"
                  value={def.color}
                  onChange={(e) =>
                    handleUpdateDomain(id, id, {
                      ...def,
                      color: e.target.value,
                    })
                  }
                  className="w-7 h-7 rounded cursor-pointer border-0 bg-transparent"
                />
                {/* ID */}
                <input
                  type="text"
                  value={id}
                  onChange={(e) => handleUpdateDomain(id, e.target.value, def)}
                  className="flex-1 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-300 font-mono focus:border-blue-500 focus:outline-none"
                  placeholder="domain-id"
                />
                {/* Label */}
                <input
                  type="text"
                  value={def.label}
                  onChange={(e) =>
                    handleUpdateDomain(id, id, {
                      ...def,
                      label: e.target.value,
                    })
                  }
                  className="flex-1 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-200 focus:border-blue-500 focus:outline-none"
                  placeholder="Display Name"
                />
                {/* Remove */}
                <button
                  onClick={() => handleRemoveDomain(id)}
                  className="text-xs text-slate-600 hover:text-red-400 transition-colors px-1"
                >
                  x
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
