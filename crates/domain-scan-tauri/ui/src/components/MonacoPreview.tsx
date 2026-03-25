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
  /** Full file content to display */
  source: string | null;
  /** File path for the currently active tab */
  file: string | null;
  /** Language hint from domain-scan */
  language: string | null;
  /** Line to scroll to and highlight (1-based) */
  highlightLine: number | null;
  /** End line of the entity span for range decoration */
  highlightEndLine: number | null;
  /** Open tabs */
  tabs: OpenTab[];
  /** Index of the active tab */
  activeTabIndex: number;
  /** Called when user clicks a tab */
  onTabSelect: (index: number) => void;
  /** Called when user closes a tab */
  onTabClose: (index: number) => void;
}

const MAX_LINES_WARNING = 10000;

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
}: MonacoPreviewProps) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const decorationsRef = useRef<editor.IEditorDecorationsCollection | null>(
    null,
  );
  const [largeFileWarning, setLargeFileWarning] = useState(false);

  const handleEditorMount: OnMount = useCallback((ed) => {
    editorRef.current = ed;
    decorationsRef.current = ed.createDecorationsCollection([]);
  }, []);

  // Scroll to highlighted line and apply decorations when they change
  useEffect(() => {
    const ed = editorRef.current;
    if (!ed || !highlightLine) return;

    // Scroll to the highlight line
    ed.revealLineInCenter(highlightLine);

    // Apply highlight decoration for the entity span
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
              position: 1, // editor.OverviewRulerLane.Center
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

  if (!source && tabs.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-600 text-sm">
        Select an entity to view source
      </div>
    );
  }

  const monacoLanguage = detectLanguage(language, file);

  return (
    <div className="h-full flex flex-col">
      {/* Tab bar */}
      {tabs.length > 0 && (
        <div className="flex items-center bg-gray-800/80 border-b border-gray-700 overflow-x-auto flex-shrink-0">
          {tabs.map((tab, i) => (
            <div
              key={tab.file}
              className={`flex items-center gap-1.5 px-3 py-1.5 text-xs cursor-pointer border-r border-gray-700 min-w-0 ${
                i === activeTabIndex
                  ? "bg-gray-900 text-white"
                  : "text-gray-400 hover:text-gray-200 hover:bg-gray-800"
              }`}
              onClick={() => onTabSelect(i)}
              onMouseDown={(e) => {
                // Middle-click to close
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
