import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { normalizeOrphanDomains } from "../layout/tubeMap";
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
  MiniMap,
  BackgroundVariant,
  useNodesState,
  useEdgesState,
  useReactFlow,
  type Viewport,
} from "@xyflow/react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { SubsystemNode } from "./SubsystemNode";
import { DependencyEdge } from "./DependencyEdge";
import { ManifestLoader } from "./ManifestLoader";
import { ManifestWizard } from "./ManifestWizard";
import { TubeMapSearchBar } from "./TubeMapSearchBar";
import { Legend } from "./Legend";
import { Breadcrumbs } from "./Breadcrumbs";
import { TubeMapStatusBar } from "./TubeMapStatusBar";
import { SubsystemDrillIn } from "./SubsystemDrillIn";
import { CoverageOverlay } from "./CoverageOverlay";
import { ShortcutHelp } from "./ShortcutHelp";
import { TubeLineStripes } from "./TubeLineStripes";
import { useTubeMapState } from "../hooks/useTubeMapState";
import { useTubeLayout } from "../hooks/useTubeLayout";
import { useToast } from "../hooks/useToast";
import type { ScanStats, TubeMapData } from "../types";

const nodeTypes = { subsystem: SubsystemNode };
const edgeTypes = { dependency: DependencyEdge };

interface TubeMapViewProps {
  onSelectedPathContextChange?: (
    scope: { subsystemId: string; name: string; prefix: string } | null,
  ) => void;
}

function TubeMapInner({ onSelectedPathContextChange }: TubeMapViewProps) {
  const state = useTubeMapState();
  const { addToast } = useToast();
  const [searchQuery, setSearchQuery] = useState("");
  const [domainFilter, setDomainFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const [zoom, setZoom] = useState(1);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [showWizard, setShowWizard] = useState(false);
  const [scanLoaded, setScanLoaded] = useState<boolean | null>(null);
  const [scanningDirectory, setScanningDirectory] = useState(false);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const reactFlowInstance = useReactFlow();

  // Normalize orphan domains → group under a gray "Unassigned" line
  const tubeMapData = useMemo(
    () => (state.tubeMapData ? normalizeOrphanDomains(state.tubeMapData) : null),
    [state.tubeMapData],
  );

  // Determine if we're in drill-in view (breadcrumbs.length > 1 means drilled in)
  const isDrilledIn = state.breadcrumbs.length > 1;
  const drilledInSubsystemId = isDrilledIn
    ? state.breadcrumbs[state.breadcrumbs.length - 1].id
    : null;

  useEffect(() => {
    let cancelled = false;
    invoke<ScanStats | null>("get_current_scan")
      .then((stats) => {
        if (!cancelled) setScanLoaded(stats !== null);
      })
      .catch(() => {
        if (!cancelled) setScanLoaded(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!onSelectedPathContextChange) return;
    if (!drilledInSubsystemId || !tubeMapData) {
      onSelectedPathContextChange(null);
      return;
    }

    const subsystem = tubeMapData.subsystems.find(
      (candidate) => candidate.id === drilledInSubsystemId,
    );
    if (!subsystem) return;

    onSelectedPathContextChange({
      subsystemId: subsystem.id,
      name: subsystem.name,
      prefix: subsystem.file_path,
    });
  }, [drilledInSubsystemId, onSelectedPathContextChange, tubeMapData]);

  const handleDrillIn = useCallback(
    (subsystemId: string) => {
      const sub = tubeMapData?.subsystems.find((s) => s.id === subsystemId);
      if (!sub) return;
      state.drillIn(sub.id, sub.name);
    },
    [tubeMapData, state.drillIn],
  );

  const handleOpenDirectory = useCallback(async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (!selected) return;
    setScanningDirectory(true);
    try {
      await invoke("scan_directory", { root: selected as string });
      setScanLoaded(true);
      addToast("Scan complete", "success");
    } catch {
      addToast("Scan failed", "error");
    } finally {
      setScanningDirectory(false);
    }
  }, [addToast]);

  const handleOpenFile = useCallback(
    async (filePath: string, line?: number) => {
      const fileName =
        filePath.split("/").filter(Boolean).pop() ?? filePath;
      try {
        await invoke("open_in_editor", {
          editor: "cursor",
          file: filePath,
          line: line ?? 1,
        });
        addToast(`Opened ${fileName} in Cursor`, "success");
      } catch {
        try {
          await invoke("open_in_editor", {
            editor: "code",
            file: filePath,
            line: line ?? 1,
          });
          addToast(`Opened ${fileName} in VS Code`, "success");
        } catch {
          addToast(`No editor available for ${fileName}`, "error");
        }
      }
    },
    [addToast],
  );

  const handleOpenFileForNode = useCallback(
    (filePath: string) => {
      handleOpenFile(filePath, 1);
    },
    [handleOpenFile],
  );

  const handleFocusDependency = useCallback(
    (subsystemId: string) => {
      state.setFocusedSubsystemId(subsystemId);
    },
    [state.setFocusedSubsystemId],
  );

  const handleGeneratePrompt = useCallback(
    async (entityNames: string[]): Promise<string> => {
      return invoke<string>("generate_prompt", {
        entityIds: entityNames,
        agents: 3,
      });
    },
    [],
  );

  const traceableSubsystems = useMemo(() => {
    if (!tubeMapData) return [];
    return tubeMapData.subsystems.filter((sub) => {
      if (domainFilter !== "all" && sub.domain !== domainFilter) return false;
      if (statusFilter !== "all" && sub.status !== statusFilter) return false;
      return true;
    });
  }, [tubeMapData, domainFilter, statusFilter]);

  useEffect(() => {
    if (!state.focusedSubsystemId) return;
    const isStillVisible = traceableSubsystems.some(
      (sub) => sub.id === state.focusedSubsystemId,
    );
    if (!isStillVisible) {
      state.setFocusedSubsystemId(null);
    }
  }, [
    state.focusedSubsystemId,
    state.setFocusedSubsystemId,
    traceableSubsystems,
  ]);

  const { nodes: layoutNodes, edges: layoutEdges, tubeLines, stationPositions } = useTubeLayout({
    tubeMapData,
    searchQuery,
    domainFilter,
    statusFilter,
    activeChainIds: state.activeChainIds,
    activeEdgeKeys: state.activeEdgeKeys,
    onDrillIn: handleDrillIn,
    onOpenFile: handleOpenFileForNode,
    onFocusDependency: handleFocusDependency,
  });

  const [nodes, setNodes, onNodesChange] = useNodesState(layoutNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(layoutEdges);

  // Sync when layout changes
  useEffect(() => {
    setNodes(layoutNodes);
    setEdges(layoutEdges);
  }, [layoutNodes, layoutEdges, setNodes, setEdges]);

  // Fit view on initial load and data changes
  useEffect(() => {
    if (layoutNodes.length === 0) return;
    const timer = setTimeout(() => {
      reactFlowInstance.fitView({ padding: 0.15, maxZoom: 1, duration: 300 });
    }, 100);
    return () => clearTimeout(timer);
  }, [reactFlowInstance, layoutNodes]);

  // Track zoom level
  const onViewportChange = useCallback((viewport: Viewport) => {
    setZoom(viewport.zoom);
  }, []);

  // Domain keys for number shortcuts (ordered by manifest domains)
  const domainKeys = useMemo(() => {
    if (!tubeMapData) return [];
    return Object.keys(tubeMapData.domains);
  }, [tubeMapData]);

  // Keyboard shortcuts for tube map
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const isInputFocused =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable;

      // Escape has special cascading behavior
      if (e.key === "Escape") {
        e.preventDefault();
        if (isInputFocused) {
          target.blur();
          return;
        }
        if (showShortcuts) {
          setShowShortcuts(false);
          return;
        }
        if (searchQuery) {
          setSearchQuery("");
          return;
        }
        if (state.focusedSubsystemId) {
          state.setFocusedSubsystemId(null);
          return;
        }
        if (domainFilter !== "all" || statusFilter !== "all") {
          setDomainFilter("all");
          setStatusFilter("all");
          return;
        }
        if (isDrilledIn) {
          state.navigateBreadcrumb(state.breadcrumbs.length - 2);
          return;
        }
        return;
      }

      // Skip other shortcuts when typing in inputs
      if (isInputFocused) return;

      switch (e.key) {
        case "f":
          e.preventDefault();
          reactFlowInstance.fitView({ padding: 0.15, maxZoom: 1, duration: 300 });
          break;
        case "/":
          e.preventDefault();
          searchInputRef.current?.focus();
          break;
        case "0":
          e.preventDefault();
          setDomainFilter("all");
          setStatusFilter("all");
          state.setFocusedSubsystemId(null);
          setSearchQuery("");
          break;
        case "?":
          e.preventDefault();
          setShowShortcuts((prev) => !prev);
          break;
        case "1":
        case "2":
        case "3":
        case "4":
        case "5":
        case "6":
        case "7":
        case "8":
        case "9": {
          e.preventDefault();
          const idx = parseInt(e.key) - 1;
          if (idx < domainKeys.length) {
            const domain = domainKeys[idx];
            setDomainFilter((prev) => (prev === domain ? "all" : domain));
          }
          break;
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    showShortcuts,
    searchQuery,
    domainFilter,
    statusFilter,
    state,
    isDrilledIn,
    reactFlowInstance,
    domainKeys,
  ]);

  const handleWizardComplete = useCallback(
    async (manifestPath: string) => {
      setShowWizard(false);
      // The manifest is already loaded into AppState by save_manifest.
      // Run matching if a scan is loaded, then fetch tube map data.
      try {
        try {
          await invoke("match_manifest");
        } catch {
          // No scan loaded — matching skipped
        }
        const data = await invoke<TubeMapData>("get_tube_map_data");
        state.setTubeMapDataDirectly(data);
        const fileName = manifestPath.split("/").pop() ?? manifestPath;
        addToast(
          `Manifest loaded: ${fileName} (${data.subsystems.length} subsystems)`,
          "success",
        );
      } catch (e) {
        addToast(`Failed to load saved manifest: ${String(e)}`, "error");
      }
    },
    [state, addToast],
  );

  // If wizard is active, show it
  if (showWizard) {
    return (
      <ManifestWizard
        onComplete={handleWizardComplete}
        onCancel={() => setShowWizard(false)}
      />
    );
  }

  // If no manifest loaded, show loader
  if (!tubeMapData) {
    return (
      <ManifestLoader
        onLoadManifest={state.loadManifest}
        onOpenDirectory={handleOpenDirectory}
        loading={state.loading}
        scanLoaded={scanLoaded}
        openDirectoryLoading={scanningDirectory}
        error={state.error}
        onStartWizard={() => setShowWizard(true)}
      />
    );
  }

  // If manifest loaded but has no subsystems, show empty state
  if (tubeMapData.subsystems.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center max-w-md">
          <p className="text-lg text-slate-300 mb-2">No subsystems found</p>
          <p className="text-sm text-slate-500 mb-4">
            The loaded manifest has no subsystems defined. Add subsystems to your manifest to see them on the tube map.
          </p>
          <button
            onClick={state.loadManifest}
            className="text-sm text-blue-400 hover:text-blue-300 transition-colors"
          >
            Load Different Manifest
          </button>
        </div>
      </div>
    );
  }

  const totalEntities = tubeMapData.subsystems.reduce(
    (sum, s) => sum + s.matched_entity_count,
    0,
  );

  const shouldVirtualize = nodes.length > 500;

  return (
    <div className="flex-1 flex flex-col">
      {/* Header: search, filters, trace */}
      <div className="flex items-center justify-between px-4 py-1.5 border-b border-slate-800/50 bg-slate-900/40 flex-shrink-0">
        <div className="flex items-center gap-3">
          <span className="text-xs text-slate-400">{tubeMapData.meta.name}</span>
          <span className="text-slate-700">|</span>
          <TubeMapSearchBar
            ref={searchInputRef}
            query={searchQuery}
            onQueryChange={setSearchQuery}
            domainFilter={domainFilter}
            onDomainFilterChange={setDomainFilter}
            statusFilter={statusFilter}
            onStatusFilterChange={setStatusFilter}
            domains={tubeMapData.domains}
            subsystems={traceableSubsystems}
            focusedSubsystemId={state.focusedSubsystemId}
            onFocusedSubsystemChange={state.setFocusedSubsystemId}
            dependencyDirection={state.dependencyDirection}
            onDependencyDirectionChange={state.setDependencyDirection}
          />
        </div>
        <div className="flex items-center gap-3 text-xs text-slate-400">
          <span>{tubeMapData.subsystems.length} subsystems</span>
          <button
            onClick={state.loadManifest}
            className="text-blue-400 hover:text-blue-300 transition-colors"
          >
            Reload
          </button>
        </div>
      </div>

      {/* Legend + Breadcrumbs */}
      <div className="flex items-center justify-between px-4 py-1 border-b border-slate-800/30 bg-slate-900/20 flex-shrink-0">
        <Legend
          domains={tubeMapData.domains}
          subsystems={tubeMapData.subsystems}
          activeDomain={domainFilter}
          onDomainClick={setDomainFilter}
        />
        {state.breadcrumbs.length > 1 && (
          <Breadcrumbs
            items={state.breadcrumbs}
            onNavigate={state.navigateBreadcrumb}
          />
        )}
      </div>

      {/* Main content: tube map canvas OR drill-in */}
      {isDrilledIn && drilledInSubsystemId ? (
        <SubsystemDrillIn
          subsystemId={drilledInSubsystemId}
          domains={tubeMapData.domains}
          onBack={() =>
            state.navigateBreadcrumb(state.breadcrumbs.length - 2)
          }
          onOpenFile={handleOpenFile}
          onGeneratePrompt={handleGeneratePrompt}
          getSubsystemDetail={state.getSubsystemDetail}
          getSubsystemEntities={state.getSubsystemEntities}
        />
      ) : (
        <div className="flex-1 relative">
          {/* Coverage overlay */}
          <CoverageOverlay
            coveragePercent={tubeMapData.coverage_percent}
            unmatchedCount={tubeMapData.unmatched_count}
            totalEntities={totalEntities + tubeMapData.unmatched_count}
            matchedEntities={totalEntities}
          />

          {/* React Flow canvas */}
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            fitView
            fitViewOptions={{ padding: 0.15, maxZoom: 1, duration: 300 }}
            minZoom={0.1}
            maxZoom={2}
            proOptions={{ hideAttribution: true }}
            onViewportChange={onViewportChange}
            onlyRenderVisibleElements={shouldVirtualize}
          >
            <Background
              variant={BackgroundVariant.Dots}
              gap={20}
              size={1}
              color="#1e293b"
            />
            <TubeLineStripes lines={tubeLines} positions={stationPositions} />
            <Controls showInteractive={false} />
            <MiniMap
              nodeColor={(node) => {
                const nodeData = node.data as { domainColor?: string };
                return nodeData.domainColor ?? "#6b7280";
              }}
              nodeStrokeColor={(node) => {
                const nodeData = node.data as { domainColor?: string };
                return nodeData.domainColor ?? "#6b7280";
              }}
              nodeStrokeWidth={2}
              maskColor="rgba(15, 23, 42, 0.7)"
              style={{
                background: "#0f172a",
                border: "1px solid #334155",
                borderRadius: "8px",
              }}
              pannable
              zoomable
            />
          </ReactFlow>
        </div>
      )}

      {/* Status bar */}
      <TubeMapStatusBar
        zoom={zoom}
        visibleNodes={nodes.length}
        totalNodes={tubeMapData.subsystems.length}
        domainFilter={domainFilter}
        statusFilter={statusFilter}
        domains={tubeMapData.domains}
        coveragePercent={tubeMapData.coverage_percent}
        unmatchedCount={tubeMapData.unmatched_count}
        onToggleShortcuts={() => setShowShortcuts((prev) => !prev)}
      />

      {/* Shortcut help overlay */}
      {showShortcuts && (
        <ShortcutHelp onClose={() => setShowShortcuts(false)} />
      )}
    </div>
  );
}

export function TubeMapView(props: TubeMapViewProps) {
  return (
    <ReactFlowProvider>
      <TubeMapInner {...props} />
    </ReactFlowProvider>
  );
}
