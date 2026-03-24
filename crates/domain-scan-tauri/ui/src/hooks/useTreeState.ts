import { useState, useCallback, useMemo } from "react";
import type { EntitySummary, TreeNode, TreeChild, Entity } from "../types";

export interface UseTreeStateReturn {
  nodes: TreeNode[];
  selectedIndex: number;
  selectedEntity: EntitySummary | null;
  toggleExpand: (index: number) => void;
  select: (index: number) => void;
  moveUp: () => void;
  moveDown: () => void;
  expandSelected: () => void;
  collapseSelected: () => void;
  setEntities: (entities: EntitySummary[]) => void;
  updateNodeChildren: (index: number, children: TreeChild[]) => void;
  /** Total number of visible rows (nodes + expanded children) */
  visibleRowCount: number;
}

export function useTreeState(): UseTreeStateReturn {
  const [nodes, setNodes] = useState<TreeNode[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);

  const setEntities = useCallback((entities: EntitySummary[]) => {
    setNodes(
      entities.map((entity) => ({
        entity,
        expanded: false,
        children: [],
      })),
    );
    setSelectedIndex(0);
  }, []);

  const toggleExpand = useCallback((index: number) => {
    setNodes((prev) =>
      prev.map((node, i) =>
        i === index ? { ...node, expanded: !node.expanded } : node,
      ),
    );
  }, []);

  const select = useCallback((index: number) => {
    setSelectedIndex(index);
  }, []);

  // Build flat list of visible rows for keyboard navigation
  const visibleRows = useMemo(() => {
    const rows: { nodeIndex: number; childIndex?: number }[] = [];
    for (let i = 0; i < nodes.length; i++) {
      rows.push({ nodeIndex: i });
      if (nodes[i].expanded) {
        for (let j = 0; j < nodes[i].children.length; j++) {
          rows.push({ nodeIndex: i, childIndex: j });
        }
      }
    }
    return rows;
  }, [nodes]);

  const moveUp = useCallback(() => {
    setSelectedIndex((prev) => Math.max(0, prev - 1));
  }, []);

  const moveDown = useCallback(() => {
    setSelectedIndex((prev) => Math.min(nodes.length - 1, prev + 1));
  }, [nodes.length]);

  const expandSelected = useCallback(() => {
    setNodes((prev) =>
      prev.map((node, i) =>
        i === selectedIndex ? { ...node, expanded: true } : node,
      ),
    );
  }, [selectedIndex]);

  const collapseSelected = useCallback(() => {
    setNodes((prev) =>
      prev.map((node, i) =>
        i === selectedIndex ? { ...node, expanded: false } : node,
      ),
    );
  }, [selectedIndex]);

  const updateNodeChildren = useCallback(
    (index: number, children: TreeChild[]) => {
      setNodes((prev) =>
        prev.map((node, i) =>
          i === index ? { ...node, children } : node,
        ),
      );
    },
    [],
  );

  const selectedEntity = nodes[selectedIndex]?.entity ?? null;

  return {
    nodes,
    selectedIndex,
    selectedEntity,
    toggleExpand,
    select,
    moveUp,
    moveDown,
    expandSelected,
    collapseSelected,
    setEntities,
    updateNodeChildren,
    visibleRowCount: visibleRows.length,
  };
}

/** Extract children from an Entity detail response */
export function extractChildren(entity: Entity): TreeChild[] {
  if ("Interface" in entity) {
    const iface = entity.Interface;
    const children: TreeChild[] = [];
    for (const m of iface.methods) {
      children.push({
        name: m.name,
        kind: "method",
        line: m.span.start_line,
        is_async: m.is_async,
        return_type: m.return_type,
      });
    }
    for (const p of iface.properties) {
      children.push({
        name: p.name,
        kind: "property",
        line: 0,
      });
    }
    return children;
  }
  if ("Service" in entity) {
    const svc = entity.Service;
    const children: TreeChild[] = [];
    for (const m of svc.methods) {
      children.push({
        name: m.name,
        kind: "method",
        line: m.span.start_line,
        is_async: m.is_async,
        return_type: m.return_type,
      });
    }
    for (const r of svc.routes) {
      children.push({
        name: `${r.method} ${r.path}`,
        kind: "route",
        line: 0,
      });
    }
    return children;
  }
  if ("Class" in entity) {
    const cls = entity.Class;
    const children: TreeChild[] = [];
    for (const m of cls.methods) {
      children.push({
        name: m.name,
        kind: "method",
        line: m.span.start_line,
        is_async: m.is_async,
        return_type: m.return_type,
      });
    }
    for (const p of cls.properties) {
      children.push({
        name: p.name,
        kind: "property",
        line: 0,
      });
    }
    return children;
  }
  if ("Schema" in entity) {
    const schema = entity.Schema;
    return schema.fields.map((f) => ({
      name: f.name,
      kind: "field" as const,
      line: 0,
    }));
  }
  if ("Impl" in entity) {
    return entity.Impl.methods.map((m) => ({
      name: m.name,
      kind: "method" as const,
      line: m.span.start_line,
      is_async: m.is_async,
      return_type: m.return_type,
    }));
  }
  return [];
}
