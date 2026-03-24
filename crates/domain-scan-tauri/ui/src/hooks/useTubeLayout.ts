import { useMemo } from "react";
import type { Node, Edge } from "@xyflow/react";
import type { TubeMapData, TubeMapSubsystem } from "../types";
import type { SubsystemNodeData } from "../components/SubsystemNode";
import type { DependencyEdgeData } from "../components/DependencyEdge";
import { buildDynamicLayout, NODE_WIDTH } from "../layout/tubeMap";
import { assignDomainColors } from "../layout/colors";

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
    if (!tubeMapData) return { nodes: [], edges: [] };

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

    // Build edges
    const edges: Edge[] = tubeMapData.connections
      .filter((conn) => {
        if (!visibleIds.has(conn.from) || !visibleIds.has(conn.to))
          return false;
        if (activeEdgeKeys !== null) {
          const key = `${conn.from}->${conn.to}`;
          return activeEdgeKeys.has(key);
        }
        return true;
      })
      .map((conn) => {
        const sourceSub = subsystemMap.get(conn.from);
        const targetSub = subsystemMap.get(conn.to);
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
      });

    return { nodes, edges };
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
