interface ManifestLoaderProps {
  onLoadManifest: () => void;
  loading: boolean;
  error: string | null;
}

export function ManifestLoader({
  onLoadManifest,
  loading,
  error,
}: ManifestLoaderProps) {
  return (
    <div className="flex-1 flex items-center justify-center">
      <div className="text-center max-w-md">
        <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-blue-500/20 to-purple-500/20 border border-slate-700/50 flex items-center justify-center">
          <svg
            width="32"
            height="32"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-slate-400"
          >
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="12" y1="18" x2="12" y2="12" />
            <line x1="9" y1="15" x2="15" y2="15" />
          </svg>
        </div>

        <h2 className="text-lg font-semibold text-slate-200 mb-2">
          Subsystem Tube Map
        </h2>
        <p className="text-sm text-slate-400 mb-6 leading-relaxed">
          Load a system manifest (JSON) to visualize your subsystems as a tube
          map. The manifest defines domains, subsystems, and their connections.
        </p>

        <button
          onClick={onLoadManifest}
          disabled={loading}
          className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg
                     bg-blue-600 hover:bg-blue-500 disabled:bg-blue-600/50
                     text-white text-sm font-medium
                     transition-colors duration-150"
        >
          {loading ? "Loading..." : "Load Manifest"}
        </button>

        {error && (
          <p className="mt-4 text-xs text-red-400 bg-red-950/50 border border-red-800/50 rounded-md px-3 py-2">
            {error}
          </p>
        )}
      </div>
    </div>
  );
}
