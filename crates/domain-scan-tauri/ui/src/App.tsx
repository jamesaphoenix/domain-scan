import { useEffect, useCallback, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useScan } from "./hooks/useScan";
import { useTreeState, extractChildren } from "./hooks/useTreeState";
import { useKeyboard } from "./hooks/useKeyboard";
import { EntityTree } from "./components/EntityTree";
import { MonacoPreview, type OpenTab } from "./components/MonacoPreview";
import { DetailsPanel } from "./components/DetailsPanel";
import { FilterBar } from "./components/FilterBar";
import { TabBar, type Tab } from "./components/TabBar";
import { TubeMapView } from "./components/TubeMapView";
import { useToast } from "./hooks/useToast";
import type { Entity, Language, FilterParams } from "./types";

interface EntityPathScope {
  subsystemId: string;
  name: string;
  prefix: string;
}

function App() {
  const scan = useScan();
  const tree = useTreeState();
  const { addToast } = useToast();

  const [activeTab, setActiveTab] = useState<Tab>("tube-map");
  const [selectedDetail, setSelectedDetail] = useState<Entity | null>(null);
  const [sourceCode, setSourceCode] = useState<string | null>(null);
  const [highlightLine, setHighlightLine] = useState<number | null>(null);
  const [highlightEndLine, setHighlightEndLine] = useState<number | null>(null);
  const [selectedChildIdx, setSelectedChildIdx] = useState<number | null>(null);
  const [openTabs, setOpenTabs] = useState<OpenTab[]>([]);
  const [activeTabIdx, setActiveTabIdx] = useState(0);
  const [currentFilters, setCurrentFilters] = useState<FilterParams>({});
  const [entityPathScope, setEntityPathScope] = useState<EntityPathScope | null>(null);

  // Client-side file source cache — survives tab switches without IPC round-trips
  const fileSourceCache = useRef<Map<string, string>>(new Map());
  const [promptOutput, setPromptOutput] = useState<string | null>(null);
  const [exportOutput, setExportOutput] = useState<string | null>(null);
  const [exportFormat, setExportFormat] = useState<"json" | "csv" | "markdown">("json");

  const MAX_OPEN_TABS = 10;

  const searchInputRef = useRef<HTMLInputElement | null>(null);

  // Sync entities from scan hook to tree state
  useEffect(() => {
    tree.setEntities(scan.entities);
  }, [scan.entities]); // eslint-disable-line react-hooks/exhaustive-deps

  // Helper: build a tab label from a file path (parent/filename for disambiguation)
  const makeTabLabel = useCallback((filePath: string): string => {
    const parts = filePath.split("/");
    if (parts.length >= 2) {
      return `${parts[parts.length - 2]}/${parts[parts.length - 1]}`;
    }
    return parts[parts.length - 1] ?? filePath;
  }, []);

  // Helper: get file source from client cache or IPC (and cache the result)
  const getCachedFileSource = useCallback(
    async (filePath: string): Promise<string> => {
      const cached = fileSourceCache.current.get(filePath);
      if (cached !== undefined) return cached;
      const src = await scan.getFileSource(filePath);
      fileSourceCache.current.set(filePath, src);
      return src;
    },
    [scan],
  );

  // Load entity detail + source when selection changes.
  // Detail and source are fetched IN PARALLEL for speed.
  // Also populates children so tree expand works on first click.
  useEffect(() => {
    const entity = tree.selectedEntity;
    if (!entity) {
      setSelectedDetail(null);
      setSourceCode(null);
      setHighlightLine(null);
      setHighlightEndLine(null);
      return;
    }

    let cancelled = false;
    const filePath = entity.file;

    (async () => {
      try {
        // Fetch detail and file source IN PARALLEL
        const [detail, src] = await Promise.all([
          scan.getEntityDetail(entity.name, entity.file),
          getCachedFileSource(filePath),
        ]);
        if (cancelled) return;

        setSelectedDetail(detail);
        setSelectedChildIdx(null); // Reset child selection on parent change

        // Populate children so the tree can expand them
        const children = extractChildren(detail);
        const idx = tree.nodes.findIndex(
          (n) =>
            n.entity.name === entity.name && n.entity.file === entity.file,
        );
        if (idx >= 0) {
          tree.updateNodeChildren(idx, children, children.length > 0);
        }

        // Manage tabs
        const existingTabIdx = openTabs.findIndex((t) => t.file === filePath);

        if (existingTabIdx >= 0) {
          setActiveTabIdx(existingTabIdx);
        } else {
          const newTab: OpenTab = {
            file: filePath,
            label: makeTabLabel(filePath),
          };
          setOpenTabs((prev) => {
            const next = [...prev, newTab];
            if (next.length > MAX_OPEN_TABS) {
              return next.slice(1);
            }
            return next;
          });
          setActiveTabIdx(
            Math.min(openTabs.length, MAX_OPEN_TABS - 1),
          );
        }

        // Set source + highlight (already available — no second await)
        const span = getSpanFromDetail(detail);
        setSourceCode(src);
        if (span) {
          setHighlightLine(span.start_line);
          setHighlightEndLine(span.end_line);
        } else {
          setHighlightLine(entity.line);
          setHighlightEndLine(null);
        }
      } catch {
        if (!cancelled) {
          setSelectedDetail(null);
          setSourceCode(null);
          setHighlightLine(null);
          setHighlightEndLine(null);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [tree.selectedIndex, tree.selectedEntity?.name, tree.selectedEntity?.file]); // eslint-disable-line react-hooks/exhaustive-deps

  // Apply filters
  const applyFilters = useCallback(
    (updates: Partial<FilterParams>) => {
      const next: FilterParams = { ...currentFilters, ...updates };
      if (!next.name_pattern) {
        delete next.name_pattern;
      }
      if (!next.path_prefix) {
        delete next.path_prefix;
      }
      setCurrentFilters(next);
      scan.filterEntities(next);
    },
    [currentFilters, scan],
  );

  // Handle search (uses search_entities for fuzzy matching)
  const handleSearch = useCallback(
    (query: string) => {
      applyFilters({ name_pattern: query.trim() || undefined });
    },
    [applyFilters],
  );

  const clearEntityPathScope = useCallback(() => {
    setEntityPathScope(null);
    const next: FilterParams = { ...currentFilters };
    delete next.path_prefix;
    setCurrentFilters(next);
    scan.filterEntities(next);
  }, [currentFilters, scan]);

  // Scan on open
  const handleOpenDirectory = useCallback(async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      try {
        await scan.scanDirectory(selected as string);
        setCurrentFilters({});
        setEntityPathScope(null);
        addToast("Scan complete", "success");
      } catch {
        addToast("Scan failed", "error");
      }
    }
  }, [scan, addToast]);

  const handleTabChange = useCallback(
    (nextTab: Tab) => {
      if (nextTab === "entities" && entityPathScope) {
        const nextFilters: FilterParams = {
          ...currentFilters,
          path_prefix: entityPathScope.prefix,
        };
        setCurrentFilters(nextFilters);
        scan.filterEntities(nextFilters);
      }
      setActiveTab(nextTab);
    },
    [currentFilters, entityPathScope, scan],
  );

  // Prompt generation
  const handleGeneratePrompt = useCallback(
    async (entityIds: string[]) => {
      try {
        const result = await scan.generatePrompt(entityIds, 3);
        setPromptOutput(result);
      } catch (e) {
        setPromptOutput(`Error: ${e}`);
      }
    },
    [scan],
  );

  // Export
  const handleExport = useCallback(
    async (format: string) => {
      try {
        const result = await scan.exportEntities(format, currentFilters);
        setExportOutput(result);
      } catch (e) {
        setExportOutput(`Error: ${e}`);
      }
    },
    [scan, currentFilters],
  );

  // Tab management — uses client cache for instant switches
  const handleTabSelect = useCallback(
    (index: number) => {
      setActiveTabIdx(index);
      const tab = openTabs[index];
      if (tab) {
        getCachedFileSource(tab.file).then((src) => {
          setSourceCode(src);
        }).catch(() => {});
      }
    },
    [openTabs, getCachedFileSource],
  );

  const handleTabClose = useCallback(
    (index: number) => {
      setOpenTabs((prev) => {
        const next = prev.filter((_, i) => i !== index);
        setActiveTabIdx((prevIdx) => {
          if (next.length === 0) {
            setSourceCode(null);
            setHighlightLine(null);
            setHighlightEndLine(null);
            return 0;
          }
          if (prevIdx >= next.length) return next.length - 1;
          if (index < prevIdx) return prevIdx - 1;
          if (index === prevIdx && prevIdx < next.length) {
            const tab = next[prevIdx];
            if (tab) {
              getCachedFileSource(tab.file).then((src) => {
                setSourceCode(src);
              }).catch(() => {});
            }
          }
          return prevIdx;
        });
        return next;
      });
    },
    [getCachedFileSource],
  );

  // Close other tabs / close all / close to the right
  const handleCloseOtherTabs = useCallback(
    (keepIndex: number) => {
      setOpenTabs((prev) => {
        const kept = prev[keepIndex];
        if (!kept) return prev;
        setActiveTabIdx(0);
        return [kept];
      });
    },
    [],
  );

  const handleCloseAllTabs = useCallback(() => {
    setOpenTabs([]);
    setActiveTabIdx(0);
    setSourceCode(null);
    setHighlightLine(null);
    setHighlightEndLine(null);
  }, []);

  const handleCloseTabsToRight = useCallback(
    (index: number) => {
      setOpenTabs((prev) => {
        const next = prev.slice(0, index + 1);
        setActiveTabIdx((prevIdx) =>
          prevIdx > index ? index : prevIdx,
        );
        return next;
      });
    },
    [],
  );

  // Open in editor
  const handleOpenInEditor = useCallback(
    async (file: string, line: number) => {
      const fileName = file.split("/").pop() ?? file;
      try {
        await scan.openInEditor("cursor", file, line);
        addToast(`Opened ${fileName} in Cursor`, "success");
      } catch {
        try {
          await scan.openInEditor("code", file, line);
          addToast(`Opened ${fileName} in VS Code`, "success");
        } catch {
          addToast(`No editor available for ${fileName}`, "error");
        }
      }
    },
    [scan, addToast],
  );

  // Child row selection — clicking a method/property/field/route scrolls Monaco to its line
  const handleSelectChild = useCallback(
    (nodeIndex: number, childIndex: number) => {
      const node = tree.nodes[nodeIndex];
      if (!node) return;
      const child = node.children[childIndex];
      if (!child) return;

      // If parent not already selected, select it first (triggers entity load)
      if (nodeIndex !== tree.selectedIndex) {
        tree.select(nodeIndex);
      }

      setSelectedChildIdx(childIndex);

      // Scroll Monaco to the child's line
      if (child.line > 0) {
        setHighlightLine(child.line);
        setHighlightEndLine(null);
      }
    },
    [tree],
  );

  // Keyboard navigation (only fires on entities tab)
  useKeyboard(
    {
      onMoveUp: tree.moveUp,
      onMoveDown: tree.moveDown,
      onExpandOrSelect: () => {
        const node = tree.nodes[tree.selectedIndex];
        if (node) {
          tree.toggleExpand(tree.selectedIndex);
        }
      },
      onCollapse: tree.collapseSelected,
      onSearch: () => {
        searchInputRef.current?.focus();
      },
      onPrompt: () => {
        const entity = tree.selectedEntity;
        if (entity) {
          handleGeneratePrompt([entity.name]);
        }
      },
      onExport: () => {
        handleExport(exportFormat);
      },
    },
    activeTab,
  );

  // Available languages from scan stats
  const availableLanguages: Language[] = scan.stats
    ? (Object.keys(scan.stats.files_by_language) as Language[])
    : [];

  // Scan progress percentage (simple: just show scanning state)
  const scanProgress = scan.scanning ? "Scanning..." : null;

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-gray-100">
      {/* Status bar */}
      <div className="flex items-center justify-between px-4 py-2 bg-gray-800 border-b border-gray-700 text-sm flex-shrink-0">
        <div className="flex items-center gap-3">
          <span className="font-semibold">domain-scan</span>
          <button
            className="text-xs text-blue-400 hover:text-blue-300"
            onClick={handleOpenDirectory}
          >
            Open Directory
          </button>
        </div>

        <div className="flex items-center gap-3">
          {scanProgress && (
            <span className="text-yellow-400 text-xs">{scanProgress}</span>
          )}
          {scan.error && (
            <span className="text-red-400 text-xs truncate max-w-xs">
              {scan.error}
            </span>
          )}
          {scan.stats && !scan.scanning && (
            <>
              <span className="text-gray-400 text-xs">
                {scan.stats.total_files} files | {scan.stats.total_interfaces}{" "}
                interfaces | {scan.stats.total_services} services |{" "}
                {scan.stats.total_schemas} schemas |{" "}
                {scan.stats.parse_duration_ms}ms
              </span>
              <div className="flex items-center gap-1">
                <select
                  className="bg-gray-700 text-gray-300 text-xs border border-gray-600 rounded px-1 py-0.5"
                  value={exportFormat}
                  onChange={(e) => setExportFormat(e.target.value as "json" | "csv" | "markdown")}
                >
                  <option value="json">JSON</option>
                  <option value="csv">CSV</option>
                  <option value="markdown">Markdown</option>
                </select>
                <button
                  className="text-xs text-blue-400 hover:text-blue-300"
                  onClick={() => handleExport(exportFormat)}
                >
                  Export All
                </button>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Tab bar */}
      <TabBar activeTab={activeTab} onTabChange={handleTabChange} />

      {/* Tab content */}
      {activeTab === "entities" ? (
        <div className="flex-1 flex overflow-hidden">
          {/* Left: Entity Tree + Filter Bar */}
          <div className="w-72 border-r border-gray-700 flex flex-col flex-shrink-0">
            <div className="flex-1 overflow-y-auto p-1">
              <EntityTree
                nodes={tree.nodes}
                selectedIndex={tree.selectedIndex}
                selectedChildIndex={selectedChildIdx}
                onSelect={(index) => {
                  setSelectedChildIdx(null);
                  if (index === tree.selectedIndex) {
                    tree.toggleExpand(index);
                  } else {
                    tree.select(index);
                  }
                }}
                onSelectChild={handleSelectChild}
              />
            </div>
            <FilterBar
              onSearch={handleSearch}
              onFilterKind={(kinds) => applyFilters({ kind: kinds })}
              onFilterLanguage={(langs) => applyFilters({ languages: langs })}
              availableLanguages={availableLanguages}
              searchInputRef={searchInputRef}
              pathScope={
                currentFilters.path_prefix
                  ? {
                      prefix: currentFilters.path_prefix,
                      label: entityPathScope?.name,
                    }
                  : null
              }
              onClearPathScope={clearEntityPathScope}
            />
          </div>

          {/* Center: Source Preview */}
          <div className="flex-1 overflow-hidden">
            {promptOutput ? (
              <div className="h-full flex flex-col">
                <div className="flex items-center justify-between px-3 py-1.5 bg-gray-800/50 border-b border-gray-700 flex-shrink-0">
                  <span className="text-xs text-gray-400">
                    Generated Prompt
                  </span>
                  <div className="flex gap-2">
                    <button
                      className="text-xs text-blue-400 hover:text-blue-300"
                      onClick={() => navigator.clipboard.writeText(promptOutput)}
                    >
                      Copy
                    </button>
                    <button
                      className="text-xs text-gray-500 hover:text-gray-300"
                      onClick={() => setPromptOutput(null)}
                    >
                      Close
                    </button>
                  </div>
                </div>
                <pre className="flex-1 overflow-auto p-3 text-xs text-gray-300 font-mono whitespace-pre-wrap">
                  {promptOutput}
                </pre>
              </div>
            ) : exportOutput ? (
              <div className="h-full flex flex-col">
                <div className="flex items-center justify-between px-3 py-1.5 bg-gray-800/50 border-b border-gray-700 flex-shrink-0">
                  <span className="text-xs text-gray-400">Export Output</span>
                  <div className="flex gap-2">
                    <button
                      className="text-xs text-blue-400 hover:text-blue-300"
                      onClick={() => navigator.clipboard.writeText(exportOutput)}
                    >
                      Copy
                    </button>
                    <button
                      className="text-xs text-gray-500 hover:text-gray-300"
                      onClick={() => setExportOutput(null)}
                    >
                      Close
                    </button>
                  </div>
                </div>
                <pre className="flex-1 overflow-auto p-3 text-xs text-gray-300 font-mono whitespace-pre-wrap">
                  {exportOutput}
                </pre>
              </div>
            ) : (
              <MonacoPreview
                source={sourceCode}
                file={tree.selectedEntity?.file ?? null}
                language={tree.selectedEntity?.language ?? null}
                highlightLine={highlightLine}
                highlightEndLine={highlightEndLine}
                tabs={openTabs}
                activeTabIndex={activeTabIdx}
                onTabSelect={handleTabSelect}
                onTabClose={handleTabClose}
                onCloseOtherTabs={handleCloseOtherTabs}
                onCloseAllTabs={handleCloseAllTabs}
                onCloseTabsToRight={handleCloseTabsToRight}
              />
            )}
          </div>

          {/* Right: Details Panel */}
          <div className="w-80 border-l border-gray-700 overflow-y-auto p-4 flex-shrink-0">
            <DetailsPanel
              entity={tree.selectedEntity}
              detail={selectedDetail}
              onOpenInEditor={handleOpenInEditor}
            />
          </div>
        </div>
      ) : (
        <TubeMapView onSelectedPathContextChange={setEntityPathScope} />
      )}
    </div>
  );
}

/** Extract Span from any Entity variant */
function getSpanFromDetail(
  detail: Entity,
): { start_line: number; end_line: number; byte_range: [number, number] } | null {
  if ("Interface" in detail) return detail.Interface.span;
  if ("Service" in detail) return detail.Service.span;
  if ("Class" in detail) return detail.Class.span;
  if ("Function" in detail) return detail.Function.span;
  if ("Schema" in detail) return detail.Schema.span;
  if ("Impl" in detail) return detail.Impl.span;
  if ("TypeAlias" in detail) return detail.TypeAlias.span;
  return null;
}

export default App;
