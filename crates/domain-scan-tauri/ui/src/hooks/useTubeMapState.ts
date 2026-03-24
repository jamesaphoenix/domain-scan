import { useState, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  TubeMapData,
  SubsystemDetail,
  EntitySummary,
} from "../types";

export type DependencyDirection = "upstream" | "downstream" | "both";

export interface BreadcrumbItem {
  id: string;
  name: string;
}

export interface UseTubeMapStateReturn {
  tubeMapData: TubeMapData | null;
  loading: boolean;
  error: string | null;
  manifestPath: string | null;
  focusedSubsystemId: string | null;
  dependencyDirection: DependencyDirection;
  breadcrumbs: BreadcrumbItem[];
  activeChainIds: Set<string> | null;
  activeEdgeKeys: Set<string> | null;
  loadManifest: () => Promise<void>;
  matchManifest: () => Promise<void>;
  setFocusedSubsystemId: (id: string | null) => void;
  setDependencyDirection: (dir: DependencyDirection) => void;
  drillIn: (subsystemId: string, name: string) => void;
  navigateBreadcrumb: (index: number) => void;
  getSubsystemDetail: (id: string) => Promise<SubsystemDetail>;
  getSubsystemEntities: (id: string) => Promise<EntitySummary[]>;
}

export function useTubeMapState(): UseTubeMapStateReturn {
  const [tubeMapData, setTubeMapData] = useState<TubeMapData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [manifestPath, setManifestPath] = useState<string | null>(null);
  const [focusedSubsystemId, setFocusedSubsystemId] = useState<string | null>(
    null,
  );
  const [dependencyDirection, setDependencyDirection] =
    useState<DependencyDirection>("both");
  const [breadcrumbs, setBreadcrumbs] = useState<BreadcrumbItem[]>([]);

  const loadManifest = useCallback(async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!selected) return;

    const path = selected as string;
    setLoading(true);
    setError(null);

    try {
      await invoke("load_manifest", { path });
      setManifestPath(path);

      // Try matching if a scan is loaded
      try {
        await invoke("match_manifest");
      } catch {
        // No scan loaded yet — matching skipped
      }

      const data = await invoke<TubeMapData>("get_tube_map_data");
      setTubeMapData(data);
      setBreadcrumbs([{ id: "root", name: data.meta.name }]);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const matchManifest = useCallback(async () => {
    if (!manifestPath) return;
    setLoading(true);
    setError(null);
    try {
      await invoke("match_manifest");
      const data = await invoke<TubeMapData>("get_tube_map_data");
      setTubeMapData(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [manifestPath]);

  const drillIn = useCallback(
    (subsystemId: string, name: string) => {
      setBreadcrumbs((prev) => [...prev, { id: subsystemId, name }]);
    },
    [],
  );

  const navigateBreadcrumb = useCallback((index: number) => {
    setBreadcrumbs((prev) => prev.slice(0, index + 1));
  }, []);

  const getSubsystemDetail = useCallback(async (id: string) => {
    return invoke<SubsystemDetail>("get_subsystem_detail", {
      subsystemId: id,
    });
  }, []);

  const getSubsystemEntities = useCallback(async (id: string) => {
    return invoke<EntitySummary[]>("get_subsystem_entities", {
      subsystemId: id,
    });
  }, []);

  // Compute dependency chain
  const { activeChainIds, activeEdgeKeys } = useMemo(() => {
    if (!focusedSubsystemId || !tubeMapData) {
      return { activeChainIds: null, activeEdgeKeys: null };
    }

    const chainIds = new Set<string>();
    const edgeKeys = new Set<string>();
    chainIds.add(focusedSubsystemId);

    const upstreamMap = new Map<
      string,
      Array<{ target: string; edgeKey: string }>
    >();
    const downstreamMap = new Map<
      string,
      Array<{ source: string; edgeKey: string }>
    >();

    for (const conn of tubeMapData.connections) {
      if (!upstreamMap.has(conn.from))
        upstreamMap.set(conn.from, []);
      upstreamMap.get(conn.from)!.push({
        target: conn.to,
        edgeKey: `${conn.from}->${conn.to}`,
      });

      if (!downstreamMap.has(conn.to))
        downstreamMap.set(conn.to, []);
      downstreamMap.get(conn.to)!.push({
        source: conn.from,
        edgeKey: `${conn.from}->${conn.to}`,
      });
    }

    // Walk upstream
    if (
      dependencyDirection === "upstream" ||
      dependencyDirection === "both"
    ) {
      const queue = [focusedSubsystemId];
      const visited = new Set<string>([focusedSubsystemId]);
      while (queue.length > 0) {
        const current = queue.shift()!;
        const deps = upstreamMap.get(current) ?? [];
        for (const dep of deps) {
          edgeKeys.add(dep.edgeKey);
          chainIds.add(dep.target);
          if (!visited.has(dep.target)) {
            visited.add(dep.target);
            queue.push(dep.target);
          }
        }
      }
    }

    // Walk downstream
    if (
      dependencyDirection === "downstream" ||
      dependencyDirection === "both"
    ) {
      const queue = [focusedSubsystemId];
      const visited = new Set<string>([focusedSubsystemId]);
      while (queue.length > 0) {
        const current = queue.shift()!;
        const deps = downstreamMap.get(current) ?? [];
        for (const dep of deps) {
          edgeKeys.add(dep.edgeKey);
          chainIds.add(dep.source);
          if (!visited.has(dep.source)) {
            visited.add(dep.source);
            queue.push(dep.source);
          }
        }
      }
    }

    return { activeChainIds: chainIds, activeEdgeKeys: edgeKeys };
  }, [focusedSubsystemId, dependencyDirection, tubeMapData]);

  return {
    tubeMapData,
    loading,
    error,
    manifestPath,
    focusedSubsystemId,
    dependencyDirection,
    breadcrumbs,
    activeChainIds,
    activeEdgeKeys,
    loadManifest,
    matchManifest,
    setFocusedSubsystemId,
    setDependencyDirection,
    drillIn,
    navigateBreadcrumb,
    getSubsystemDetail,
    getSubsystemEntities,
  };
}
