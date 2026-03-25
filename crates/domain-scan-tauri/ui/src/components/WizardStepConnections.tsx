import { useCallback } from "react";
import type {
  ManifestConnection,
  ManifestSubsystem,
  ConnectionType,
} from "../types";

interface WizardStepConnectionsProps {
  connections: ManifestConnection[];
  subsystems: ManifestSubsystem[];
  onConnectionsChange: (connections: ManifestConnection[]) => void;
}

export function WizardStepConnections({
  connections,
  subsystems,
  onConnectionsChange,
}: WizardStepConnectionsProps) {
  const subsystemIds = subsystems.map((s) => s.id);

  const handleAdd = useCallback(() => {
    const from = subsystemIds[0] ?? "";
    const to = subsystemIds[1] ?? subsystemIds[0] ?? "";
    onConnectionsChange([
      ...connections,
      { from, to, label: "", type: "depends_on" },
    ]);
  }, [connections, subsystemIds, onConnectionsChange]);

  const handleUpdate = useCallback(
    (index: number, updates: Partial<ManifestConnection>) => {
      const next = [...connections];
      next[index] = { ...next[index], ...updates };
      onConnectionsChange(next);
    },
    [connections, onConnectionsChange],
  );

  const handleRemove = useCallback(
    (index: number) => {
      onConnectionsChange(connections.filter((_, i) => i !== index));
    },
    [connections, onConnectionsChange],
  );

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-medium text-slate-200">
          Connections ({connections.length})
        </h3>
        <button
          onClick={handleAdd}
          disabled={subsystems.length < 2}
          className="text-xs text-blue-400 hover:text-blue-300 disabled:text-slate-600 transition-colors"
        >
          + Add connection
        </button>
      </div>

      {connections.length === 0 ? (
        <p className="text-xs text-slate-500 py-8 text-center">
          No connections. These are inferred from import graphs during
          bootstrap. You can also add them manually.
        </p>
      ) : (
        <div className="space-y-1.5">
          {/* Header */}
          <div className="flex items-center gap-2 px-2 text-[10px] text-slate-500 uppercase tracking-wider">
            <span className="w-40">From</span>
            <span className="w-8 text-center">-&gt;</span>
            <span className="w-40">To</span>
            <span className="w-28">Type</span>
            <span className="flex-1">Label</span>
            <span className="w-6" />
          </div>

          {connections.map((conn, i) => (
            <div
              key={i}
              className="flex items-center gap-2 p-2 rounded border border-slate-700/40 bg-slate-800/20"
            >
              {/* From */}
              <select
                value={conn.from}
                onChange={(e) => handleUpdate(i, { from: e.target.value })}
                className="w-40 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-300 focus:border-blue-500 focus:outline-none"
              >
                {subsystemIds.map((id) => (
                  <option key={id} value={id}>
                    {subsystems.find((s) => s.id === id)?.name ?? id}
                  </option>
                ))}
              </select>

              <span className="w-8 text-center text-slate-600 text-xs">
                -&gt;
              </span>

              {/* To */}
              <select
                value={conn.to}
                onChange={(e) => handleUpdate(i, { to: e.target.value })}
                className="w-40 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-300 focus:border-blue-500 focus:outline-none"
              >
                {subsystemIds.map((id) => (
                  <option key={id} value={id}>
                    {subsystems.find((s) => s.id === id)?.name ?? id}
                  </option>
                ))}
              </select>

              {/* Type */}
              <select
                value={conn.type}
                onChange={(e) =>
                  handleUpdate(i, {
                    type: e.target.value as ConnectionType,
                  })
                }
                className="w-28 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-300 focus:border-blue-500 focus:outline-none"
              >
                <option value="depends_on">depends_on</option>
                <option value="uses">uses</option>
                <option value="triggers">triggers</option>
              </select>

              {/* Label */}
              <input
                type="text"
                value={conn.label}
                onChange={(e) => handleUpdate(i, { label: e.target.value })}
                className="flex-1 px-2 py-1 rounded bg-slate-800 border border-slate-700 text-xs text-slate-300 focus:border-blue-500 focus:outline-none"
                placeholder="why this connection exists"
              />

              {/* Remove */}
              <button
                onClick={() => handleRemove(i)}
                className="text-xs text-slate-600 hover:text-red-400 transition-colors px-1"
              >
                x
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Validation warnings */}
      {connections.some((c) => c.from === c.to) && (
        <div className="mt-4 text-xs text-yellow-400 bg-yellow-950/30 border border-yellow-800/40 rounded px-3 py-2">
          Some connections point from a subsystem to itself. This is usually
          unintended.
        </div>
      )}
      {connections.some(
        (c) =>
          !subsystemIds.includes(c.from) || !subsystemIds.includes(c.to),
      ) && (
        <div className="mt-4 text-xs text-red-400 bg-red-950/30 border border-red-800/40 rounded px-3 py-2">
          Some connections reference subsystem IDs that don't exist.
        </div>
      )}
    </div>
  );
}
