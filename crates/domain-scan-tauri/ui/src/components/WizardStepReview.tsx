import { useMemo } from "react";
import type {
  ManifestMeta,
  DomainDef,
  ManifestSubsystem,
  ManifestConnection,
} from "../types";

interface WizardStepReviewProps {
  meta: ManifestMeta;
  domains: Record<string, DomainDef>;
  subsystems: ManifestSubsystem[];
  connections: ManifestConnection[];
}

export function WizardStepReview({
  meta,
  domains,
  subsystems,
  connections,
}: WizardStepReviewProps) {
  const domainKeys = Object.keys(domains);
  const subsystemIds = new Set(subsystems.map((s) => s.id));

  const validation = useMemo(() => {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Check for duplicate subsystem IDs
    const idCounts: Record<string, number> = {};
    for (const s of subsystems) {
      idCounts[s.id] = (idCounts[s.id] ?? 0) + 1;
    }
    for (const [id, count] of Object.entries(idCounts)) {
      if (count > 1) errors.push(`Duplicate subsystem ID: "${id}" (${count}x)`);
    }

    // Check domain references
    for (const s of subsystems) {
      if (s.domain && !(s.domain in domains)) {
        errors.push(
          `Subsystem "${s.id}" references unknown domain "${s.domain}"`,
        );
      }
    }

    // Check connection references
    for (const c of connections) {
      if (!subsystemIds.has(c.from))
        errors.push(`Connection from unknown subsystem: "${c.from}"`);
      if (!subsystemIds.has(c.to))
        errors.push(`Connection to unknown subsystem: "${c.to}"`);
      if (c.from === c.to)
        warnings.push(`Self-referencing connection: "${c.from}"`);
    }

    // Check orphan domains
    const usedDomains = new Set(subsystems.map((s) => s.domain));
    for (const dk of domainKeys) {
      if (!usedDomains.has(dk))
        warnings.push(`Domain "${dk}" has no subsystems`);
    }

    // Check empty name
    if (!meta.name.trim()) warnings.push("Project name is empty");

    return { errors, warnings, valid: errors.length === 0 };
  }, [meta, domains, domainKeys, subsystems, connections, subsystemIds]);

  // Stats
  const statsByDomain = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const s of subsystems) {
      counts[s.domain] = (counts[s.domain] ?? 0) + 1;
    }
    return counts;
  }, [subsystems]);

  return (
    <div className="p-6 max-w-3xl mx-auto">
      <h3 className="text-sm font-medium text-slate-200 mb-4">
        Manifest Review
      </h3>

      {/* Summary cards */}
      <div className="grid grid-cols-4 gap-3 mb-6">
        <SummaryCard label="Domains" value={domainKeys.length} />
        <SummaryCard label="Subsystems" value={subsystems.length} />
        <SummaryCard label="Connections" value={connections.length} />
        <SummaryCard
          label="Status"
          value={validation.valid ? "Valid" : `${validation.errors.length} errors`}
          color={validation.valid ? "green" : "red"}
        />
      </div>

      {/* Domains breakdown */}
      <div className="mb-6">
        <h4 className="text-xs font-medium text-slate-400 mb-2 uppercase tracking-wider">
          Domains
        </h4>
        <div className="space-y-1">
          {domainKeys.map((dk) => {
            const def = domains[dk];
            const count = statsByDomain[dk] ?? 0;
            return (
              <div
                key={dk}
                className="flex items-center gap-2 text-xs text-slate-300"
              >
                <div
                  className="w-2.5 h-2.5 rounded-full"
                  style={{ backgroundColor: def.color }}
                />
                <span className="font-medium">{def.label}</span>
                <span className="text-slate-500">({dk})</span>
                <span className="text-slate-500 ml-auto">
                  {count} subsystem{count !== 1 ? "s" : ""}
                </span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Validation */}
      {validation.errors.length > 0 && (
        <div className="mb-4 p-3 rounded border border-red-800/50 bg-red-950/30">
          <h4 className="text-xs font-medium text-red-400 mb-1.5">Errors</h4>
          <ul className="space-y-0.5">
            {validation.errors.map((e, i) => (
              <li key={i} className="text-xs text-red-300">
                {e}
              </li>
            ))}
          </ul>
        </div>
      )}

      {validation.warnings.length > 0 && (
        <div className="mb-4 p-3 rounded border border-yellow-800/40 bg-yellow-950/20">
          <h4 className="text-xs font-medium text-yellow-400 mb-1.5">
            Warnings
          </h4>
          <ul className="space-y-0.5">
            {validation.warnings.map((w, i) => (
              <li key={i} className="text-xs text-yellow-300">
                {w}
              </li>
            ))}
          </ul>
        </div>
      )}

      {validation.valid && validation.warnings.length === 0 && (
        <div className="mb-4 p-3 rounded border border-green-800/40 bg-green-950/20">
          <p className="text-xs text-green-400">
            Manifest is valid. Click "Save Manifest" to save to disk and view
            the tube map.
          </p>
        </div>
      )}

      {/* JSON preview (collapsed) */}
      <details className="mt-4">
        <summary className="text-xs text-slate-500 cursor-pointer hover:text-slate-300">
          Preview JSON
        </summary>
        <pre className="mt-2 p-3 rounded bg-slate-800/50 border border-slate-700/50 text-[11px] text-slate-400 overflow-x-auto max-h-64 overflow-y-auto">
          {JSON.stringify({ meta, domains, subsystems, connections }, null, 2)}
        </pre>
      </details>
    </div>
  );
}

function SummaryCard({
  label,
  value,
  color,
}: {
  label: string;
  value: number | string;
  color?: "green" | "red";
}) {
  const valueColor =
    color === "green"
      ? "text-green-400"
      : color === "red"
        ? "text-red-400"
        : "text-slate-100";

  return (
    <div className="p-3 rounded-lg border border-slate-700/50 bg-slate-800/30">
      <div className={`text-lg font-semibold ${valueColor}`}>{value}</div>
      <div className="text-[10px] text-slate-500 uppercase tracking-wider">
        {label}
      </div>
    </div>
  );
}
