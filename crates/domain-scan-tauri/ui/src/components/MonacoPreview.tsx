import { useEffect, useRef, useCallback, useState } from "react";
import Editor, { type OnMount } from "@monaco-editor/react";
import type { editor } from "monaco-editor";

// Map domain-scan language names to Monaco language IDs
const LANGUAGE_MAP: Record<string, string> = {
  TypeScript: "typescript",
  Python: "python",
  Rust: "rust",
  Go: "go",
  Java: "java",
  Kotlin: "kotlin",
  CSharp: "csharp",
  Swift: "swift",
  PHP: "php",
  Ruby: "ruby",
  Scala: "scala",
  Cpp: "cpp",
};

// Extension fallback for language detection
const EXT_MAP: Record<string, string> = {
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  py: "python",
  rs: "rust",
  go: "go",
  java: "java",
  kt: "kotlin",
  cs: "csharp",
  swift: "swift",
  php: "php",
  rb: "ruby",
  scala: "scala",
  cpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  c: "c",
  h: "c",
  hpp: "cpp",
  json: "json",
  yaml: "yaml",
  yml: "yaml",
  md: "markdown",
  html: "html",
  css: "css",
  scss: "scss",
  toml: "ini",
  sql: "sql",
  sh: "shell",
  bash: "shell",
  zsh: "shell",
};

function detectLanguage(
  language: string | null,
  file: string | null,
): string {
  if (language && LANGUAGE_MAP[language]) {
    return LANGUAGE_MAP[language];
  }
  if (file) {
    const ext = file.split(".").pop()?.toLowerCase() ?? "";
    if (EXT_MAP[ext]) {
      return EXT_MAP[ext];
    }
  }
  return "plaintext";
}

export interface OpenTab {
  file: string;
  label: string;
}

interface MonacoPreviewProps {
  source: string | null;
  file: string | null;
  language: string | null;
  highlightLine: number | null;
  highlightEndLine: number | null;
  tabs: OpenTab[];
  activeTabIndex: number;
  onTabSelect: (index: number) => void;
  onTabClose: (index: number) => void;
  onCloseOtherTabs?: (keepIndex: number) => void;
  onCloseAllTabs?: () => void;
  onCloseTabsToRight?: (index: number) => void;
}

const MAX_LINES_WARNING = 10000;

// ---------------------------------------------------------------------------
// Tab context menu
// ---------------------------------------------------------------------------

interface ContextMenuState {
  x: number;
  y: number;
  tabIndex: number;
}

function TabContextMenu({
  state,
  tabCount,
  onClose,
  onCloseTab,
  onCloseOthers,
  onCloseAll,
  onCloseToRight,
}: {
  state: ContextMenuState;
  tabCount: number;
  onClose: () => void;
  onCloseTab: () => void;
  onCloseOthers: () => void;
  onCloseAll: () => void;
  onCloseToRight: () => void;
}) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEsc);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEsc);
    };
  }, [onClose]);

  const items = [
    { label: "Close", action: onCloseTab, enabled: true },
    {
      label: "Close Others",
      action: onCloseOthers,
      enabled: tabCount > 1,
    },
    {
      label: "Close to the Right",
      action: onCloseToRight,
      enabled: state.tabIndex < tabCount - 1,
    },
    { label: "Close All", action: onCloseAll, enabled: tabCount > 0 },
  ];

  return (
    <div
      ref={menuRef}
      className="fixed z-50 bg-gray-800 border border-gray-600 rounded-md shadow-xl py-1 min-w-[160px]"
      style={{ left: state.x, top: state.y }}
    >
      {items.map((item) => (
        <button
          key={item.label}
          className={`w-full text-left px-3 py-1.5 text-xs ${
            item.enabled
              ? "text-gray-200 hover:bg-gray-700"
              : "text-gray-600 cursor-default"
          }`}
          disabled={!item.enabled}
          onClick={() => {
            item.action();
            onClose();
          }}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function MonacoPreview({
  source,
  file,
  language,
  highlightLine,
  highlightEndLine,
  tabs,
  activeTabIndex,
  onTabSelect,
  onTabClose,
  onCloseOtherTabs,
  onCloseAllTabs,
  onCloseTabsToRight,
}: MonacoPreviewProps) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const decorationsRef = useRef<editor.IEditorDecorationsCollection | null>(
    null,
  );
  const [largeFileWarning, setLargeFileWarning] = useState(false);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(
    null,
  );
  const tabBarRef = useRef<HTMLDivElement>(null);

  const handleEditorMount: OnMount = useCallback((ed) => {
    editorRef.current = ed;
    decorationsRef.current = ed.createDecorationsCollection([]);
  }, []);

  // Scroll to highlighted line and apply decorations when they change
  useEffect(() => {
    const ed = editorRef.current;
    if (!ed || !highlightLine) return;

    ed.revealLineInCenter(highlightLine);

    const startLine = highlightLine;
    const endLine = highlightEndLine ?? highlightLine;

    if (decorationsRef.current) {
      decorationsRef.current.set([
        {
          range: {
            startLineNumber: startLine,
            startColumn: 1,
            endLineNumber: endLine,
            endColumn: ed.getModel()?.getLineMaxColumn(endLine) ?? 1,
          },
          options: {
            isWholeLine: true,
            className: "monaco-entity-highlight",
            overviewRuler: {
              color: "#3b82f680",
              position: 1,
            },
          },
        },
      ]);
    }
  }, [highlightLine, highlightEndLine, source]);

  // Check large file warning
  useEffect(() => {
    if (source) {
      const lineCount = source.split("\n").length;
      setLargeFileWarning(lineCount > MAX_LINES_WARNING);
    } else {
      setLargeFileWarning(false);
    }
  }, [source]);

  // Scroll active tab into view when it changes
  useEffect(() => {
    const container = tabBarRef.current;
    if (!container) return;
    const activeEl = container.children[activeTabIndex] as HTMLElement | undefined;
    if (activeEl) {
      activeEl.scrollIntoView({ behavior: "smooth", block: "nearest", inline: "nearest" });
    }
  }, [activeTabIndex]);

  const handleTabContextMenu = useCallback(
    (e: React.MouseEvent, index: number) => {
      e.preventDefault();
      setContextMenu({ x: e.clientX, y: e.clientY, tabIndex: index });
    },
    [],
  );

  // Tab scroll buttons
  const scrollTabs = useCallback((direction: "left" | "right") => {
    const container = tabBarRef.current;
    if (!container) return;
    const scrollAmount = 200;
    container.scrollBy({
      left: direction === "left" ? -scrollAmount : scrollAmount,
      behavior: "smooth",
    });
  }, []);

  if (!source && tabs.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-600 text-sm">
        Select an entity to view source
      </div>
    );
  }

  const monacoLanguage = detectLanguage(language, file);
  const hasOverflow =
    tabBarRef.current
      ? tabBarRef.current.scrollWidth > tabBarRef.current.clientWidth
      : tabs.length > 6;

  return (
    <div className="h-full flex flex-col">
      {/* Tab bar with scroll */}
      {tabs.length > 0 && (
        <div className="flex items-center bg-gray-800/80 border-b border-gray-700 flex-shrink-0">
          {/* Scroll left button */}
          {hasOverflow && (
            <button
              className="px-1.5 py-1.5 text-gray-500 hover:text-gray-300 flex-shrink-0"
              onClick={() => scrollTabs("left")}
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5">
                <path d="M8 2L4 6l4 4" />
              </svg>
            </button>
          )}

          {/* Scrollable tab container */}
          <div
            ref={tabBarRef}
            className="flex items-center overflow-x-auto flex-1 scrollbar-none"
            style={{ scrollbarWidth: "none" }}
          >
            {tabs.map((tab, i) => (
              <div
                key={tab.file}
                className={`flex items-center gap-1.5 px-3 py-1.5 text-xs cursor-pointer
                           border-r border-gray-700 flex-shrink-0 ${
                  i === activeTabIndex
                    ? "bg-gray-900 text-white"
                    : "text-gray-400 hover:text-gray-200 hover:bg-gray-800"
                }`}
                onClick={() => onTabSelect(i)}
                onContextMenu={(e) => handleTabContextMenu(e, i)}
                onMouseDown={(e) => {
                  if (e.button === 1) {
                    e.preventDefault();
                    onTabClose(i);
                  }
                }}
              >
                <span className="truncate max-w-[160px]">{tab.label}</span>
                <button
                  className="ml-1 text-gray-500 hover:text-gray-200 flex-shrink-0"
                  onClick={(e) => {
                    e.stopPropagation();
                    onTabClose(i);
                  }}
                >
                  &times;
                </button>
              </div>
            ))}
          </div>

          {/* Scroll right button */}
          {hasOverflow && (
            <button
              className="px-1.5 py-1.5 text-gray-500 hover:text-gray-300 flex-shrink-0"
              onClick={() => scrollTabs("right")}
            >
              <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5">
                <path d="M4 2l4 4-4 4" />
              </svg>
            </button>
          )}
        </div>
      )}

      {/* Context menu */}
      {contextMenu && (
        <TabContextMenu
          state={contextMenu}
          tabCount={tabs.length}
          onClose={() => setContextMenu(null)}
          onCloseTab={() => onTabClose(contextMenu.tabIndex)}
          onCloseOthers={() => onCloseOtherTabs?.(contextMenu.tabIndex)}
          onCloseAll={() => onCloseAllTabs?.()}
          onCloseToRight={() => onCloseTabsToRight?.(contextMenu.tabIndex)}
        />
      )}

      {/* Large file warning */}
      {largeFileWarning && (
        <div className="px-3 py-1 bg-yellow-900/30 border-b border-yellow-700/50 text-xs text-yellow-400 flex-shrink-0">
          Large file ({source ? source.split("\n").length.toLocaleString() : 0}{" "}
          lines) — editor performance may be affected
        </div>
      )}

      {/* Monaco Editor */}
      {source ? (
        <div className="flex-1 overflow-hidden">
          <Editor
            value={source}
            language={monacoLanguage}
            theme="vs-dark"
            onMount={handleEditorMount}
            options={{
              readOnly: true,
              minimap: { enabled: true },
              scrollBeyondLastLine: false,
              fontSize: 12,
              lineNumbers: "on",
              renderLineHighlight: "all",
              wordWrap: "off",
              automaticLayout: true,
              domReadOnly: true,
            }}
          />
        </div>
      ) : (
        <div className="flex items-center justify-center flex-1 text-gray-600 text-sm">
          Select an entity to view source
        </div>
      )}

      {/* Language tag */}
      {file && (
        <div className="px-3 py-1 bg-gray-800/50 border-t border-gray-700 text-[10px] text-gray-500 flex-shrink-0 flex justify-between">
          <span className="truncate">{file}</span>
          <span>{monacoLanguage}</span>
        </div>
      )}
    </div>
  );
}
