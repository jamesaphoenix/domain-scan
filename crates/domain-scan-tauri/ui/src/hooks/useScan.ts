import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ScanStats,
  EntitySummary,
  FilterParams,
  Entity,
  BuildStatus,
} from "../types";

export interface UseScanReturn {
  stats: ScanStats | null;
  scanning: boolean;
  error: string | null;
  entities: EntitySummary[];
  scanDirectory: (root: string) => Promise<void>;
  filterEntities: (filters: FilterParams) => Promise<void>;
  searchEntities: (query: string) => Promise<void>;
  getEntityDetail: (name: string, file: string) => Promise<Entity>;
  getEntitySource: (
    file: string,
    startByte: number,
    endByte: number,
  ) => Promise<string>;
  getFileSource: (file: string) => Promise<string>;
  generatePrompt: (entityIds: string[], agents: number) => Promise<string>;
  exportEntities: (
    format: string,
    filters: FilterParams,
  ) => Promise<string>;
  getBuildStatus: () => Promise<Record<string, BuildStatus>>;
  openInEditor: (editor: string, file: string, line: number) => Promise<void>;
  checkEditorsAvailable: () => Promise<Record<string, boolean>>;
}

export function useScan(): UseScanReturn {
  const [stats, setStats] = useState<ScanStats | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [entities, setEntities] = useState<EntitySummary[]>([]);

  const scanDirectory = useCallback(async (root: string) => {
    setScanning(true);
    setError(null);
    try {
      const result = await invoke<ScanStats>("scan_directory", { root });
      setStats(result);
      // Load all entities after scan
      const allEntities = await invoke<EntitySummary[]>("filter_entities", {
        filters: {},
      });
      setEntities(allEntities);
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }, []);

  const filterEntities = useCallback(async (filters: FilterParams) => {
    try {
      const result = await invoke<EntitySummary[]>("filter_entities", {
        filters,
      });
      setEntities(result);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const searchEntities = useCallback(async (query: string) => {
    try {
      const result = await invoke<EntitySummary[]>("search_entities", {
        query,
      });
      setEntities(result);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const getEntityDetail = useCallback(
    async (name: string, file: string): Promise<Entity> => {
      return invoke<Entity>("get_entity_detail", { name, file });
    },
    [],
  );

  const getEntitySource = useCallback(
    async (
      file: string,
      startByte: number,
      endByte: number,
    ): Promise<string> => {
      return invoke<string>("get_entity_source", {
        file,
        startByte,
        endByte,
      });
    },
    [],
  );

  const getFileSource = useCallback(
    async (file: string): Promise<string> => {
      return invoke<string>("get_file_source", { file });
    },
    [],
  );

  const generatePrompt = useCallback(
    async (entityIds: string[], agents: number): Promise<string> => {
      return invoke<string>("generate_prompt", {
        entityIds,
        agents,
      });
    },
    [],
  );

  const exportEntities = useCallback(
    async (format: string, filters: FilterParams): Promise<string> => {
      return invoke<string>("export_entities", { format, filters });
    },
    [],
  );

  const getBuildStatus = useCallback(async () => {
    return invoke<Record<string, BuildStatus>>("get_build_status");
  }, []);

  const openInEditor = useCallback(
    async (editor: string, file: string, line: number) => {
      await invoke("open_in_editor", { editor, file, line });
    },
    [],
  );

  const checkEditorsAvailable = useCallback(async () => {
    return invoke<Record<string, boolean>>("check_editors_available");
  }, []);

  return {
    stats,
    scanning,
    error,
    entities,
    scanDirectory,
    filterEntities,
    searchEntities,
    getEntityDetail,
    getEntitySource,
    getFileSource,
    generatePrompt,
    exportEntities,
    getBuildStatus,
    openInEditor,
    checkEditorsAvailable,
  };
}
