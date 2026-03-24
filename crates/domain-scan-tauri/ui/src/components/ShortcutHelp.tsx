interface ShortcutHelpProps {
  onClose: () => void;
}

const SHORTCUTS = [
  { key: "f", description: "Fit view (zoom to show all nodes)" },
  { key: "i", description: "Toggle interface/entity side panel" },
  { key: "/", description: "Focus search input" },
  {
    key: "Esc",
    description: "Close panel / clear search / clear trace / clear filters / pop breadcrumb",
  },
  { key: "0", description: "Clear all filters" },
  { key: "1-9", description: "Toggle domain filter (by order)" },
  { key: "?", description: "Toggle this help overlay" },
];

export function ShortcutHelp({ onClose }: ShortcutHelpProps) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="bg-slate-900 border border-slate-700 rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-slate-200">
            Keyboard Shortcuts
          </h3>
          <button
            onClick={onClose}
            className="text-slate-500 hover:text-slate-300 transition-colors"
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
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        <div className="space-y-2">
          {SHORTCUTS.map((shortcut) => (
            <div
              key={shortcut.key}
              className="flex items-center justify-between py-1.5"
            >
              <span className="text-xs text-slate-400">
                {shortcut.description}
              </span>
              <kbd className="ml-4 font-mono text-[11px] px-2 py-1 rounded bg-slate-800 border border-slate-700 text-slate-300 shrink-0">
                {shortcut.key}
              </kbd>
            </div>
          ))}
        </div>

        <div className="mt-4 pt-3 border-t border-slate-800 text-[11px] text-slate-600 text-center">
          Press <kbd className="font-mono px-1 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-400">Esc</kbd> or <kbd className="font-mono px-1 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-400">?</kbd> to close
        </div>
      </div>
    </div>
  );
}
