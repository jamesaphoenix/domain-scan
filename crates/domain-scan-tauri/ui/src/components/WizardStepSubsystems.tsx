import { useCallback, useMemo } from "react";
import type { DomainDef, ManifestSubsystem } from "../types";

interface WizardStepSubsystemsProps {
  subsystems: ManifestSubsystem[];
  domains: Record<string, DomainDef>;
  onSubsystemsChange: (subsystems: ManifestSubsystem[]) => void;
}

export function WizardStepSubsystems({
  subsystems,
  domains,
  onSubsystemsChange,
}: WizardStepSubsystemsProps) {
  const domainKeys = useMemo(() => Object.keys(domains), [domains]);

  // Group subsystems by domain
  const grouped = useMemo(() => {
    const groups: Record<string, ManifestSubsystem[]> = {};
    for (const key of domainKeys) {
      groups[key] = [];
    }
    groups["_unassigned"] = [];
    for (const sub of subsystems) {
      const bucket = sub.domain in domains ? sub.domain : "_unassigned";
      groups[bucket].push(sub);
    }
    return groups;
  }, [subsystems, domains, domainKeys]);

  const handleUpdate = useCallback(
    (index: number, updates: Partial<ManifestSubsystem>) => {
      const next = [...subsystems];
      next[index] = { ...next[index], ...updates };
      onSubsystemsChange(next);
    },
    [subsystems, onSubsystemsChange],
  );

  const handleRemove = useCallback(
    (index: number) => {
      onSubsystemsChange(subsystems.filter((_, i) => i !== index));
    },
    [subsystems, onSubsystemsChange],
  );

  const handleAdd = useCallback(() => {
    const defaultDomain = domainKeys[0] ?? "";
    onSubsystemsChange([
      ...subsystems,
      {
        id: `subsystem-${subsystems.length + 1}`,
        name: `New Subsystem`,
        domain: defaultDomain,
        status: "new",
        filePath: "",
        interfaces: [],
        operations: [],
        tables: [],
        events: [],
        children: [],
        dependencies: [],
      },
    ]);
  }, [subsystems, domainKeys, onSubsystemsChange]);

  const handleMoveDomain = useCallback(
    (subIndex: number, newDomain: string) => {
      handleUpdate(subIndex, { domain: newDomain });
    },
    [handleUpdate],
  );

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-medium text-slate-200">
          Subsystems ({subsystems.length})
        </h3>
        <button
          onClick={handleAdd}
          className="text-xs text-blue-400 hover:text-blue-300 transition-colors"
        >
          + Add subsystem
        </button>
      </div>

      {subsystems.length === 0 ? (
        <p className="text-xs text-slate-500 py-8 text-center">
          No subsystems. Go back to Domains and click "Analyze codebase" to
          auto-detect, or add manually.
        </p>
      ) : (
        <div className="space-y-6">
          {Object.entries(grouped).map(([domainId, subs]) => {
            if (subs.length === 0) return null;
            const domainDef = domains[domainId];
            const label = domainDef?.label ?? "Unassigned";
            const color = domainDef?.color ?? "#6b7280";

            return (
              <div key={domainId}>
                <div className="flex items-center gap-2 mb-2">
                  <div
                    className="w-3 h-3 rounded-full"
                    style={{ backgroundColor: color }}
                  />
                  <span className="text-xs font-medium text-slate-300">
                    {label}
                  </span>
                  <span className="text-xs text-slate-500">
                    ({subs.length})
                  </span>
                </div>
                <div className="space-y-1.5">
                  {subs.map((sub) => {
                    const globalIndex = subsystems.findIndex(
                      (s) => s.id === sub.id,
                    );
                    return (
                      <SubsystemRow
                        key={sub.id}
                        sub={sub}
                        index={globalIndex}
                        domainKeys={domainKeys}
                        domains={domains}
                        onUpdate={handleUpdate}
                        onRemove={handleRemove}
                        onMoveDomain={handleMoveDomain}
                      />
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

interface SubsystemRowProps {
  sub: ManifestSubsystem;
  index: number;
  domainKeys: string[];
  domains: Record<string, DomainDef>;
  onUpdate: (index: number, updates: Partial<ManifestSubsystem>) => void;
  onRemove: (index: number) => void;
  onMoveDomain: (index: number, domain: string) => void;
}

function SubsystemRow({
  sub,
  index,
  domainKeys,
  domains,
  onUpdate,
  onRemove,
  onMoveDomain,
}: SubsystemRowProps) {
  return (
    <div className="flex items-center gap-2 p-2 rounded border border-slate-700/40 bg-slate-800/20">
      {/* ID */}
      <input
        type="text"
        value={sub.id}
        onChange={(e) => onUpdate(index, { id: e.target.value })}
        className="w-36 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-300 font-mono focus:border-blue-500 focus:outline-none"
        placeholder="subsystem-id"
      />
      {/* Name */}
      <input
        type="text"
        value={sub.name}
        onChange={(e) => onUpdate(index, { name: e.target.value })}
        className="flex-1 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-200 focus:border-blue-500 focus:outline-none"
        placeholder="Display Name"
      />
      {/* Domain */}
      <select
        value={sub.domain}
        onChange={(e) => onMoveDomain(index, e.target.value)}
        className="w-32 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-300 focus:border-blue-500 focus:outline-none"
      >
        {domainKeys.map((dk) => (
          <option key={dk} value={dk}>
            {domains[dk]?.label ?? dk}
          </option>
        ))}
      </select>
      {/* Status */}
      <select
        value={sub.status}
        onChange={(e) =>
          onUpdate(index, {
            status: e.target.value as ManifestSubsystem["status"],
          })
        }
        className="w-24 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-300 focus:border-blue-500 focus:outline-none"
      >
        <option value="new">new</option>
        <option value="built">built</option>
        <option value="rebuild">rebuild</option>
        <option value="boilerplate">boilerplate</option>
      </select>
      {/* File path */}
      <input
        type="text"
        value={sub.filePath}
        onChange={(e) => onUpdate(index, { filePath: e.target.value })}
        className="w-48 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-400 font-mono focus:border-blue-500 focus:outline-none"
        placeholder="src/..."
      />
      {/* Remove */}
      <button
        onClick={() => onRemove(index)}
        className="text-xs text-slate-600 hover:text-red-400 transition-colors px-1"
      >
        x
      </button>
    </div>
  );
}
