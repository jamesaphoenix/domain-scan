import { useMemo } from "react";
import type { Node, Edge } from "@xyflow/react";
import type { TubeMapData, TubeMapSubsystem } from "../types";
import type { SubsystemNodeData } from "../components/SubsystemNode";
import type { DependencyEdgeData, BundledConnection } from "../components/DependencyEdge";
import type { ComputedLine } from "../layout/types";
import { buildDynamicLayout, NODE_WIDTH } from "../layout/tubeMap";
import { assignDomainColors } from "../layout/colors";
import type { TubeMapConnection } from "../types";

/** Edges between the same two domains are bundled when count exceeds this */
const BUNDLE_THRESHOLD = 3;

/** Build an individual (non-bundled) edge from a connection */
function buildIndividualEdge(
  conn: TubeMapConnection,
  sourceSub: TubeMapSubsystem | undefined,
  targetSub: TubeMapSubsystem | undefined,
  domainColors: Map<string, string>,
): Edge {
  const sourceDomainColor =
    domainColors.get(sourceSub?.domain ?? "") ?? "#6b7280";
  const targetDomainColor =
    domainColors.get(targetSub?.domain ?? "") ?? "#6b7280";

  const edgeData: DependencyEdgeData = {
    connectionType: conn.type,
    label: conn.label,
    sourceName: sourceSub?.name ?? conn.from,
    targetName: targetSub?.name ?? conn.to,
    sourceInterfaces: [],
    targetInterfaces: [],
    sourceDomainColor,
    targetDomainColor,
  };

  return {
    id: `${conn.from}->${conn.to}`,
    source: conn.from,
    target: conn.to,
    type: "dependency",
    data: edgeData,
  };
}

interface UseTubeLayoutOptions {
  tubeMapData: TubeMapData | null;
  searchQuery: string;
  domainFilter: string;
  statusFilter: string;
  activeChainIds: Set<string> | null;
  activeEdgeKeys: Set<string> | null;
  onDrillIn: (nodeId: string) => void;
  onOpenFile: (filePath: string) => void;
  onFocusDependency: (subsystemId: string) => void;
}

export interface UseTubeLayoutReturn {
  nodes: Node[];
  edges: Edge[];
  tubeLines: ComputedLine[];
  stationPositions: Map<string, { x: number; y: number }>;
}

export function useTubeLayout({
  tubeMapData,
  searchQuery,
  domainFilter,
  statusFilter,
  activeChainIds,
  activeEdgeKeys,
  onDrillIn,
  onOpenFile,
  onFocusDependency,
}: UseTubeLayoutOptions): UseTubeLayoutReturn {
  return useMemo(() => {
    if (!tubeMapData) return { nodes: [], edges: [], tubeLines: [], stationPositions: new Map() };

    // Build layout
    const layout = buildDynamicLayout(tubeMapData);

    // Build domain color map
    const domainIds = [
      ...new Set(tubeMapData.subsystems.map((s) => s.domain)),
    ];
    const domainColors = assignDomainColors(tubeMapData.domains, domainIds);

    // Build subsystem lookup
    const subsystemMap = new Map<string, TubeMapSubsystem>();
    for (const sub of tubeMapData.subsystems) {
      subsystemMap.set(sub.id, sub);
    }

    // Filter subsystems
    const visibleSubsystems = tubeMapData.subsystems.filter((sub) => {
      if (domainFilter && domainFilter !== "all" && sub.domain !== domainFilter)
        return false;
      if (statusFilter && statusFilter !== "all" && sub.status !== statusFilter)
        return false;
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        if (
          !sub.name.toLowerCase().includes(q) &&
          !sub.id.toLowerCase().includes(q) &&
          !sub.description.toLowerCase().includes(q)
        )
          return false;
      }
      return true;
    });

    const visibleIds = new Set(visibleSubsystems.map((s) => s.id));

    // Count children's entity stats per parent subsystem
    const childStatsByParent = new Map<
      string,
      {
        interfaces: number;
        operations: number;
        tables: number;
        events: number;
      }
    >();
    for (const sub of tubeMapData.subsystems) {
      if (sub.has_children) {
        const children = tubeMapData.subsystems.filter(
          (s) => s.id !== sub.id && s.domain === sub.domain,
        );
        let ifaces = 0;
        let ops = 0;
        let tables = 0;
        let events = 0;
        for (const child of children) {
          ifaces += child.interface_count;
          ops += child.operation_count;
          tables += child.table_count;
          events += child.event_count;
        }
        childStatsByParent.set(sub.id, {
          interfaces: ifaces,
          operations: ops,
          tables: tables,
          events: events,
        });
      }
    }

    // Build nodes
    const nodes: Node[] = visibleSubsystems.map((sub) => {
      const pos = layout.positions.get(sub.id) ?? { x: 0, y: 0 };
      const domainColor = domainColors.get(sub.domain) ?? "#6b7280";
      const isDimmed =
        activeChainIds !== null && !activeChainIds.has(sub.id);
      const childStats = childStatsByParent.get(sub.id);

      const nodeData: SubsystemNodeData = {
        label: sub.name,
        description: sub.description,
        domain: sub.domain,
        domainColor,
        status: sub.status,
        interfaces: [],
        tables: [],
        operations: [],
        events: [],
        hasChildren: sub.has_children,
        filePath: sub.file_path,
        dependencyCount: sub.dependency_count,
        matchedEntityCount: sub.matched_entity_count,
        dimmed: isDimmed,
        childInterfaceCount: childStats?.interfaces ?? sub.interface_count,
        childOperationCount: childStats?.operations ?? sub.operation_count,
        childTableCount: childStats?.tables ?? sub.table_count,
        childEventCount: childStats?.events ?? sub.event_count,
        onDrillIn,
        onOpenFile,
        onFocusDependency: () => onFocusDependency(sub.id),
      };

      return {
        id: sub.id,
        type: "subsystem",
        position: pos,
        data: nodeData,
        width: NODE_WIDTH,
      };
    });

    // Filter visible connections
    const visibleConnections = tubeMapData.connections.filter((conn) => {
      if (!visibleIds.has(conn.from) || !visibleIds.has(conn.to)) return false;
      if (activeEdgeKeys !== null) {
        const key = `${conn.from}->${conn.to}`;
        return activeEdgeKeys.has(key);
      }
      return true;
    });

    // Build edges — bundle dense inter-domain connections when not tracing
    const edges: Edge[] = [];
    const isTracing = activeEdgeKeys !== null;

    if (isTracing) {
      // During dependency trace, show individual edges (no bundling)
      for (const conn of visibleConnections) {
        const sourceSub = subsystemMap.get(conn.from);
        const targetSub = subsystemMap.get(conn.to);
        edges.push(buildIndividualEdge(conn, sourceSub, targetSub, domainColors));
      }
    } else {
      // Group inter-domain connections by domain pair for bundling
      const domainPairGroups = new Map<string, typeof visibleConnections>();
      const intraDomainConns: typeof visibleConnections = [];

      for (const conn of visibleConnections) {
        const sourceSub = subsystemMap.get(conn.from);
        const targetSub = subsystemMap.get(conn.to);
        if (!sourceSub || !targetSub || sourceSub.domain === targetSub.domain) {
          // Same-domain edges are never bundled
          intraDomainConns.push(conn);
        } else {
          // Normalize key so A->B and B->A bundle separately
          const key = `${sourceSub.domain}::${targetSub.domain}`;
          const group = domainPairGroups.get(key);
          if (group) {
            group.push(conn);
          } else {
            domainPairGroups.set(key, [conn]);
          }
        }
      }

      // Add intra-domain edges as individual
      for (const conn of intraDomainConns) {
        const sourceSub = subsystemMap.get(conn.from);
        const targetSub = subsystemMap.get(conn.to);
        edges.push(buildIndividualEdge(conn, sourceSub, targetSub, domainColors));
      }

      // Process inter-domain groups — bundle if count > threshold
      for (const [pairKey, conns] of domainPairGroups) {
        if (conns.length > BUNDLE_THRESHOLD) {
          const [sourceDomainId, targetDomainId] = pairKey.split("::");
          const sourceDomainColor = domainColors.get(sourceDomainId) ?? "#6b7280";
          const targetDomainColor = domainColors.get(targetDomainId) ?? "#6b7280";

          // Pick center station from each domain as the bundle anchor
          const sourceStations = visibleSubsystems.filter(
            (s) => s.domain === sourceDomainId,
          );
          const targetStations = visibleSubsystems.filter(
            (s) => s.domain === targetDomainId,
          );
          const sourceAnchor =
            sourceStations[Math.floor(sourceStations.length / 2)];
          const targetAnchor =
            targetStations[Math.floor(targetStations.length / 2)];

          if (!sourceAnchor || !targetAnchor) continue;

          const sourceDomainLabel =
            tubeMapData.domains[sourceDomainId]?.label ?? sourceDomainId;
          const targetDomainLabel =
            tubeMapData.domains[targetDomainId]?.label ?? targetDomainId;

          const bundledConnections: BundledConnection[] = conns.map((c) => ({
            fromName: subsystemMap.get(c.from)?.name ?? c.from,
            toName: subsystemMap.get(c.to)?.name ?? c.to,
            label: c.label,
            type: c.type,
          }));

          const edgeData: DependencyEdgeData = {
            connectionType: "depends_on",
            label: `${conns.length} connections`,
            sourceName: sourceDomainLabel,
            targetName: targetDomainLabel,
            sourceInterfaces: [],
            targetInterfaces: [],
            sourceDomainColor,
            targetDomainColor,
            bundleCount: conns.length,
            bundledConnections,
          };

          edges.push({
            id: `bundle::${pairKey}`,
            source: sourceAnchor.id,
            target: targetAnchor.id,
            type: "dependency",
            data: edgeData,
          });
        } else {
          // Below threshold — render individual edges
          for (const conn of conns) {
            const sourceSub = subsystemMap.get(conn.from);
            const targetSub = subsystemMap.get(conn.to);
            edges.push(
              buildIndividualEdge(conn, sourceSub, targetSub, domainColors),
            );
          }
        }
      }
    }

    // Build filtered tube lines for stripe rendering (only lines with ≥2 visible stations)
    const tubeLines = layout.lines
      .map((line) => ({
        ...line,
        stationIds: line.stationIds.filter((id) => visibleIds.has(id)),
      }))
      .filter((line) => line.stationIds.length >= 2);

    return { nodes, edges, tubeLines, stationPositions: layout.positions };
  }, [
    tubeMapData,
    searchQuery,
    domainFilter,
    statusFilter,
    activeChainIds,
    activeEdgeKeys,
    onDrillIn,
    onOpenFile,
    onFocusDependency,
  ]);
}
