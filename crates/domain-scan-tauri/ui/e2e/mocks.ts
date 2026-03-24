/**
 * Tauri IPC mock utilities for Playwright E2E tests.
 *
 * Since tests run against the Vite dev server (not the Tauri webview),
 * we inject mock implementations of `window.__TAURI_INTERNALS__` so
 * that `@tauri-apps/api/core` invoke() calls resolve with test data.
 */

import type { Page } from "@playwright/test";
import type { ScanStats, EntitySummary } from "../src/types";

// ---------------------------------------------------------------------------
// Mock data factories
// ---------------------------------------------------------------------------

export const MOCK_SCAN_STATS: ScanStats = {
  total_files: 42,
  files_by_language: { TypeScript: 30, Rust: 12 },
  total_interfaces: 15,
  total_services: 8,
  total_classes: 5,
  total_methods: 120,
  total_functions: 25,
  total_schemas: 10,
  total_type_aliases: 7,
  total_implementations: 3,
  parse_duration_ms: 150,
  cache_hits: 0,
  cache_misses: 42,
};

export const MOCK_ENTITIES: EntitySummary[] = [
  {
    name: "AuthProvider",
    kind: "interface",
    file: "src/auth/provider.ts",
    line: 5,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "UserService",
    kind: "service",
    file: "src/services/user.ts",
    line: 10,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "ApiRouter",
    kind: "class",
    file: "src/api/router.ts",
    line: 1,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
];

// ---------------------------------------------------------------------------
// Mock IPC injection
// ---------------------------------------------------------------------------

export interface MockIPCOptions {
  /** Path the dialog:open mock returns (null = user cancelled). */
  dialogResult?: string | null;
  /** Stats returned by scan_directory. Null = throw error. */
  scanStats?: ScanStats | null;
  /** Error message thrown by scan_directory (when scanStats is null). */
  scanError?: string;
  /** Entities returned by filter_entities / search_entities. */
  entities?: EntitySummary[];
}

/**
 * Inject Tauri IPC mocks into the page BEFORE the React app loads.
 *
 * Must be called before `page.goto()`.
 */
export async function setupTauriMocks(
  page: Page,
  options: MockIPCOptions = {},
): Promise<void> {
  const dialogResult = options.dialogResult !== undefined ? options.dialogResult : "/mock/test-project";
  const scanStats = options.scanStats !== undefined ? options.scanStats : MOCK_SCAN_STATS;
  const scanError = options.scanError !== undefined ? options.scanError : "Scan failed";
  const entities = options.entities !== undefined ? options.entities : MOCK_ENTITIES;

  // Serialize data for injection into browser context
  const serialized = JSON.stringify({
    dialogResult,
    scanStats,
    scanError,
    entities,
  });

  await page.addInitScript((data: string) => {
    const config = JSON.parse(data);

    // Initialize __TAURI_INTERNALS__ (same structure as @tauri-apps/api/mocks)
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w = window as any;
    w.__TAURI_INTERNALS__ = w.__TAURI_INTERNALS__ ?? {};
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ =
      w.__TAURI_EVENT_PLUGIN_INTERNALS__ ?? {};

    const internals = w.__TAURI_INTERNALS__ as Record<string, unknown>;
    const callbacks = new Map<number, (data: unknown) => unknown>();

    internals.invoke = async (cmd: string, args?: Record<string, unknown>) => {
      // Dialog plugin: open file/directory picker
      if (cmd === "plugin:dialog|open") {
        return config.dialogResult;
      }

      // Scan directory
      if (cmd === "scan_directory") {
        if (config.scanStats === null) {
          throw new Error(config.scanError);
        }
        return config.scanStats;
      }

      // Filter entities
      if (cmd === "filter_entities") {
        return config.entities;
      }

      // Search entities
      if (cmd === "search_entities") {
        const query = (args?.query as string ?? "").toLowerCase();
        return config.entities.filter(
          (e: { name: string }) => e.name.toLowerCase().includes(query),
        );
      }

      // Get entity detail — return a stub
      if (cmd === "get_entity_detail") {
        const name = args?.name as string;
        return {
          Interface: {
            name,
            file: args?.file ?? "unknown.ts",
            span: { start_line: 1, start_col: 0, end_line: 10, end_col: 0, byte_range: [0, 100] },
            visibility: "public",
            generics: [],
            extends: [],
            methods: [],
            properties: [],
            language_kind: "interface",
            decorators: [],
          },
        };
      }

      // Get entity source
      if (cmd === "get_entity_source") {
        return "// mock source code\nexport interface MockEntity {}";
      }

      // Build status
      if (cmd === "get_build_status") {
        return {};
      }

      // Check editors
      if (cmd === "check_editors_available") {
        return { cursor: false, code: false };
      }

      // Generate prompt
      if (cmd === "generate_prompt") {
        return "Mock prompt output";
      }

      // Export entities
      if (cmd === "export_entities") {
        return JSON.stringify(config.entities);
      }

      // Tube map commands — return empty defaults
      if (cmd === "load_manifest") {
        return {
          meta: { name: "Test", version: "1.0", description: "Test manifest" },
          domains: {},
          subsystems: [],
          connections: [],
        };
      }
      if (cmd === "match_manifest") {
        return { matched: [], unmatched: [], coverage_percent: 0 };
      }
      if (cmd === "get_tube_map_data") {
        return {
          meta: { name: "Test", version: "1.0", description: "Test" },
          domains: {},
          subsystems: [],
          connections: [],
          coverage_percent: 0,
          unmatched_count: 0,
        };
      }

      // Default: return null for unknown commands
      return null;
    };

    internals.transformCallback = (
      callback: (data: unknown) => unknown,
      once = false,
    ) => {
      const id =
        globalThis.crypto.getRandomValues(new Uint32Array(1))[0];
      callbacks.set(id, (data: unknown) => {
        if (once) callbacks.delete(id);
        return callback?.(data);
      });
      return id;
    };

    internals.unregisterCallback = (id: number) => {
      callbacks.delete(id);
    };

    internals.runCallback = (id: number, data: unknown) => {
      const cb = callbacks.get(id);
      if (cb) cb(data);
    };

    internals.callbacks = callbacks;

    internals.convertFileSrc = (filePath: string, protocol = "asset") => {
      const path = encodeURIComponent(filePath);
      return `${protocol}://localhost/${path}`;
    };

    // Mock metadata so window/webview APIs don't crash
    internals.metadata = {
      currentWindow: { label: "main" },
      currentWebview: { windowLabel: "main", label: "main" },
    };

    // Plugin path internals
    internals.plugins = { path: { sep: "/", delimiter: ":" } };
  }, serialized);
}
