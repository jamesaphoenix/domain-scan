import { useCallback, useEffect, useMemo, useState } from "react";
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
} from "@xyflow/react";
import { SubsystemNode } from "./SubsystemNode";
import { DependencyEdge } from "./DependencyEdge";
import { ManifestLoader } from "./ManifestLoader";
import { useTubeMapState } from "../hooks/useTubeMapState";
import { useTubeLayout } from "../hooks/useTubeLayout";

const nodeTypes = { subsystem: SubsystemNode };
const edgeTypes = { dependency: DependencyEdge };

function TubeMapInner() {
  const state = useTubeMapState();
  const [searchQuery, setSearchQuery] = useState("");
  const [domainFilter, setDomainFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");

  const reactFlowInstance = useReactFlow();

  const handleDrillIn = useCallback(
    (nodeId: string) => {
      const sub = state.tubeMapData?.subsystems.find(
        (s) => s.name === nodeId || s.id === nodeId,
      );
      if (sub?.has_children) {
        state.drillIn(sub.id, sub.name);
      }
    },
    [state],
  );

  const handleOpenFile = useCallback((_filePath: string) => {
    // Will be wired to open_in_editor in Phase D
  }, []);

  const handleFocusDependency = useCallback(
    (subsystemId: string) => {
      state.setFocusedSubsystemId(subsystemId);
    },
    [state],
  );

  const { nodes: layoutNodes, edges: layoutEdges } = useTubeLayout({
    tubeMapData: state.tubeMapData,
    searchQuery,
    domainFilter,
    statusFilter,
    activeChainIds: state.activeChainIds,
    activeEdgeKeys: state.activeEdgeKeys,
    onDrillIn: handleDrillIn,
    onOpenFile: handleOpenFile,
    onFocusDependency: handleFocusDependency,
  });

  const [nodes, setNodes, onNodesChange] = useNodesState(layoutNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(layoutEdges);

  // Sync when layout changes
  useMemo(() => {
    setNodes(layoutNodes);
    setEdges(layoutEdges);
  }, [layoutNodes, layoutEdges, setNodes, setEdges]);

  // Fit view on initial load and data changes
  useEffect(() => {
    if (layoutNodes.length === 0) return;
    const timer = setTimeout(() => {
      reactFlowInstance.fitView({ padding: 0.15, maxZoom: 1 });
    }, 100);
    return () => clearTimeout(timer);
  }, [reactFlowInstance, layoutNodes]);

  // If no manifest loaded, show loader
  if (!state.tubeMapData) {
    return (
      <ManifestLoader
        onLoadManifest={state.loadManifest}
        loading={state.loading}
        error={state.error}
      />
    );
  }

  return (
    <div className="flex-1 flex flex-col">
      {/* Tube map header bar */}
      <div className="flex items-center justify-between px-4 py-1.5 border-b border-slate-800/50 bg-slate-900/40 flex-shrink-0">
        <div className="flex items-center gap-3">
          <span className="text-xs text-slate-400">
            {state.tubeMapData.meta.name}
          </span>
          <span className="text-slate-700">|</span>
          <input
            type="text"
            placeholder="Search subsystems..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="bg-slate-800/60 border border-slate-700/50 rounded px-2 py-1 text-xs text-slate-300 placeholder-slate-500 focus:outline-none focus:border-slate-500 w-48"
          />
          <select
            value={domainFilter}
            onChange={(e) => setDomainFilter(e.target.value)}
            className="bg-slate-800/60 border border-slate-700/50 rounded px-2 py-1 text-xs text-slate-300 focus:outline-none focus:border-slate-500"
          >
            <option value="all">All domains</option>
            {Object.entries(state.tubeMapData.domains).map(([id, def]) => (
              <option key={id} value={id}>
                {def.label}
              </option>
            ))}
          </select>
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="bg-slate-800/60 border border-slate-700/50 rounded px-2 py-1 text-xs text-slate-300 focus:outline-none focus:border-slate-500"
          >
            <option value="all">All statuses</option>
            <option value="built">Built</option>
            <option value="rebuild">Rebuild</option>
            <option value="new">New</option>
            <option value="boilerplate">Boilerplate</option>
          </select>
        </div>
        <div className="flex items-center gap-3 text-xs text-slate-400">
          <span>
            {state.tubeMapData.subsystems.length} subsystems
          </span>
          {state.tubeMapData.coverage_percent > 0 && (
            <span className="text-emerald-400">
              {state.tubeMapData.coverage_percent.toFixed(1)}% coverage
            </span>
          )}
          {state.tubeMapData.unmatched_count > 0 && (
            <span className="text-amber-400">
              {state.tubeMapData.unmatched_count} unmatched
            </span>
          )}
          <button
            onClick={state.loadManifest}
            className="text-blue-400 hover:text-blue-300 transition-colors"
          >
            Reload
          </button>
        </div>
      </div>

      {/* React Flow canvas */}
      <div className="flex-1">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          fitView
          fitViewOptions={{ padding: 0.15, maxZoom: 1 }}
          minZoom={0.1}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
        >
          <Background
            variant={BackgroundVariant.Dots}
            gap={20}
            size={1}
            color="#1e293b"
          />
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
    </div>
  );
}

export function TubeMapView() {
  return (
    <ReactFlowProvider>
      <TubeMapInner />
    </ReactFlowProvider>
  );
}
