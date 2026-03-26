/**
 * Tauri IPC mock utilities for Playwright E2E tests.
 *
 * Since tests run against the Vite dev server (not the Tauri webview),
 * we inject mock implementations of `window.__TAURI_INTERNALS__` so
 * that `@tauri-apps/api/core` invoke() calls resolve with test data.
 */

import type { Page } from "@playwright/test";
import type { ScanStats, EntitySummary, TubeMapData, TubeMapSubsystem, SystemManifest } from "../src/types";

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
// Tube Map mock data factories
// ---------------------------------------------------------------------------

function makeSubsystem(overrides: Partial<TubeMapSubsystem> & { id: string; name: string; domain: string }): TubeMapSubsystem {
  return {
    status: "built",
    description: "",
    file_path: `src/${overrides.id}/`,
    matched_entity_count: 0,
    interface_count: 0,
    operation_count: 0,
    table_count: 0,
    event_count: 0,
    has_children: false,
    child_count: 0,
    dependency_count: 0,
    ...overrides,
  };
}

/** TubeMapData matching minimal.json: 1 domain, 2 subsystems, 1 connection */
export const MOCK_MINIMAL_TUBE_MAP: TubeMapData = {
  meta: { name: "Minimal Test App", version: "1.0", description: "Minimal manifest for smoke testing" },
  domains: { core: { label: "Core", color: "#3b82f6" } },
  subsystems: [
    makeSubsystem({ id: "auth", name: "Authentication", domain: "core", matched_entity_count: 1, interface_count: 1, operation_count: 1, table_count: 1 }),
    makeSubsystem({ id: "api", name: "API Gateway", domain: "core", matched_entity_count: 1, interface_count: 1, operation_count: 1, dependency_count: 1 }),
  ],
  connections: [{ from: "api", to: "auth", label: "validates identity", type: "depends_on" }],
  coverage_percent: 0,
  unmatched_count: 0,
};

/** TubeMapData matching octospark-system.json: 7 domains, 18 subsystems, 50 connections */
export const MOCK_OCTOSPARK_TUBE_MAP: TubeMapData = (() => {
  const domains: TubeMapData["domains"] = {
    "platform-core": { label: "Platform Core", color: "#3b82f6" },
    "media-storage": { label: "Media & Storage", color: "#22c55e" },
    "services": { label: "Services", color: "#f97316" },
    "experimentation": { label: "Experimentation", color: "#a855f7" },
    "workflows": { label: "Workflows", color: "#ef4444" },
    "frontends": { label: "Frontends", color: "#6b7280" },
    "post-launch": { label: "Post-Launch", color: "#9ca3af" },
  };
  const subsystems: TubeMapSubsystem[] = [
    makeSubsystem({ id: "auth", name: "Auth & Identity", domain: "platform-core", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "org-team", name: "Org & Team Management", domain: "platform-core", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "billing", name: "Billing & Subscriptions", domain: "platform-core", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "credit-service", name: "Credit Service", domain: "platform-core", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "notifications", name: "Notifications", domain: "platform-core", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "webhooks", name: "Webhooks", domain: "platform-core", has_children: true, child_count: 2 }),
    makeSubsystem({ id: "asset-management", name: "Asset Management", domain: "media-storage", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "media-uploader", name: "Media Uploader", domain: "media-storage", has_children: true, child_count: 2 }),
    makeSubsystem({ id: "media-enrichment", name: "Media Enrichment", domain: "media-storage", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "retention-cleaner", name: "Retention Cleaner", domain: "media-storage", has_children: true, child_count: 2 }),
    makeSubsystem({ id: "social-oauth", name: "Social Media OAuth", domain: "services", has_children: true, child_count: 5 }),
    makeSubsystem({ id: "publisher", name: "Social Media Publisher", domain: "services", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "analytics-collector", name: "Analytics Collector", domain: "services", has_children: true, child_count: 2 }),
    makeSubsystem({ id: "ai-generation", name: "AI Generation", domain: "experimentation", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "prompt-bandit", name: "Contextual Prompt Bandit", domain: "experimentation", has_children: true, child_count: 7 }),
    makeSubsystem({ id: "workflows", name: "Workflows", domain: "workflows", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "api-cli-sdk", name: "API / CLI / SDK", domain: "frontends", has_children: true, child_count: 3 }),
    makeSubsystem({ id: "agency-onboarding", name: "Agency Client Onboarding", domain: "post-launch", has_children: true, child_count: 3 }),
  ];
  // Generate 50 connections (representative sample of cross-subsystem dependencies)
  const connections: TubeMapData["connections"] = [];
  const pairs: [string, string, string][] = [
    ["org-team", "auth", "authenticates via"], ["billing", "auth", "validates identity"],
    ["billing", "org-team", "bills per org"], ["credit-service", "billing", "consumes credits"],
    ["notifications", "auth", "sends to user"], ["notifications", "org-team", "org notifications"],
    ["webhooks", "auth", "validates webhook"], ["webhooks", "notifications", "triggers notification"],
    ["asset-management", "auth", "asset ownership"], ["media-uploader", "asset-management", "stores assets"],
    ["media-uploader", "auth", "upload auth"], ["media-enrichment", "asset-management", "enriches assets"],
    ["media-enrichment", "ai-generation", "AI tagging"], ["retention-cleaner", "asset-management", "purges assets"],
    ["social-oauth", "auth", "OAuth flow"], ["publisher", "social-oauth", "posts via OAuth"],
    ["publisher", "media-uploader", "attaches media"], ["publisher", "asset-management", "reads assets"],
    ["publisher", "credit-service", "costs credits"], ["analytics-collector", "social-oauth", "fetches metrics"],
    ["analytics-collector", "publisher", "tracks posts"], ["ai-generation", "credit-service", "AI costs"],
    ["ai-generation", "auth", "AI auth"], ["prompt-bandit", "ai-generation", "generates variants"],
    ["prompt-bandit", "analytics-collector", "reward signal"], ["prompt-bandit", "publisher", "A/B publish"],
    ["workflows", "publisher", "scheduled posts"], ["workflows", "media-enrichment", "media pipeline"],
    ["workflows", "notifications", "workflow alerts"], ["workflows", "ai-generation", "AI in workflows"],
    ["api-cli-sdk", "auth", "API auth"], ["api-cli-sdk", "org-team", "org scoping"],
    ["api-cli-sdk", "billing", "usage tracking"], ["api-cli-sdk", "publisher", "publish API"],
    ["api-cli-sdk", "analytics-collector", "analytics API"], ["api-cli-sdk", "asset-management", "asset API"],
    ["agency-onboarding", "auth", "onboarding auth"], ["agency-onboarding", "org-team", "creates org"],
    ["agency-onboarding", "social-oauth", "connects socials"], ["agency-onboarding", "billing", "setup billing"],
    ["credit-service", "notifications", "low balance alert"], ["webhooks", "billing", "payment events"],
    ["media-enrichment", "notifications", "enrichment done"], ["retention-cleaner", "notifications", "purge report"],
    ["publisher", "notifications", "publish status"], ["analytics-collector", "notifications", "report ready"],
    ["prompt-bandit", "credit-service", "bandit costs"], ["workflows", "credit-service", "workflow costs"],
    ["agency-onboarding", "notifications", "welcome email"], ["agency-onboarding", "asset-management", "brand assets"],
  ];
  for (const [from, to, label] of pairs) {
    connections.push({ from, to, label, type: "depends_on" });
  }
  return {
    meta: { name: "Octospark", version: "1.0", description: "Autonomous social growth platform" },
    domains,
    subsystems,
    connections,
    coverage_percent: 85,
    unmatched_count: 12,
  };
})();

/** TubeMapData matching large.json: 20 domains, 200 subsystems, 500 connections */
export const MOCK_LARGE_TUBE_MAP: TubeMapData = (() => {
  const domainCount = 20;
  const subsystemsPerDomain = 10;
  const colors = [
    "#3b82f6", "#22c55e", "#f97316", "#a855f7", "#ef4444",
    "#eab308", "#06b6d4", "#ec4899", "#14b8a6", "#f59e0b",
    "#6366f1", "#84cc16", "#d946ef", "#0ea5e9", "#f43f5e",
    "#10b981", "#8b5cf6", "#fb923c", "#64748b", "#a3e635",
  ];
  const domains: TubeMapData["domains"] = {};
  for (let d = 0; d < domainCount; d++) {
    domains[`domain-${d}`] = { label: `Domain ${d}`, color: colors[d] };
  }
  const subsystems: TubeMapSubsystem[] = [];
  for (let d = 0; d < domainCount; d++) {
    for (let s = 0; s < subsystemsPerDomain; s++) {
      const idx = d * subsystemsPerDomain + s;
      subsystems.push(makeSubsystem({
        id: `subsystem-${idx}`,
        name: `Subsystem ${idx}`,
        domain: `domain-${d}`,
        matched_entity_count: Math.floor(Math.random() * 5),
        interface_count: Math.floor(Math.random() * 3),
        operation_count: Math.floor(Math.random() * 4),
        dependency_count: Math.floor(Math.random() * 3),
      }));
    }
  }
  // Generate 500 connections (cross-domain and intra-domain)
  const connections: TubeMapData["connections"] = [];
  for (let i = 0; i < 500; i++) {
    const fromIdx = i % 200;
    const toIdx = (fromIdx + 1 + (i * 7) % 199) % 200;
    if (fromIdx !== toIdx) {
      connections.push({
        from: `subsystem-${fromIdx}`,
        to: `subsystem-${toIdx}`,
        label: `dep-${i}`,
        type: "depends_on",
      });
    }
  }
  return {
    meta: { name: "Large Stress Test", version: "1.0", description: "Stress test: 20 domains, 200 subsystems, 500 connections" },
    domains,
    subsystems,
    connections,
    coverage_percent: 60,
    unmatched_count: 40,
  };
})();

/** TubeMapData matching circular-deps.json: 2 domains, 6 subsystems with circular dependencies */
export const MOCK_CIRCULAR_DEPS_TUBE_MAP: TubeMapData = {
  meta: { name: "Circular Dependencies", version: "1.0", description: "Subsystems with mutual circular dependencies" },
  domains: {
    backend: { label: "Backend", color: "#3b82f6" },
    frontend: { label: "Frontend", color: "#22c55e" },
  },
  subsystems: [
    makeSubsystem({ id: "service-a", name: "Service A", domain: "backend", description: "Service A depends on B" }),
    makeSubsystem({ id: "service-b", name: "Service B", domain: "backend", description: "Service B depends on A (circular)" }),
    makeSubsystem({ id: "service-c", name: "Service C", domain: "backend", status: "rebuild", description: "3-way cycle" }),
    makeSubsystem({ id: "service-d", name: "Service D", domain: "backend", description: "Part of 3-way cycle" }),
    makeSubsystem({ id: "service-e", name: "Service E", domain: "backend", status: "new", description: "Completes 3-way cycle" }),
    makeSubsystem({ id: "ui-app", name: "UI App", domain: "frontend", description: "Frontend depends on backend" }),
  ],
  connections: [
    { from: "service-a", to: "service-b", label: "calls service B", type: "depends_on" },
    { from: "service-b", to: "service-a", label: "calls service A back (circular)", type: "depends_on" },
    { from: "service-c", to: "service-d", label: "calls D", type: "depends_on" },
    { from: "service-d", to: "service-e", label: "calls E", type: "depends_on" },
    { from: "service-e", to: "service-c", label: "calls C (completes cycle)", type: "depends_on" },
    { from: "ui-app", to: "service-a", label: "fetches from A", type: "uses" },
    { from: "ui-app", to: "service-c", label: "fetches from C", type: "uses" },
  ],
  coverage_percent: 0,
  unmatched_count: 0,
};

/** TubeMapData matching no-domains.json: 3 subsystems with no domains defined */
export const MOCK_NO_DOMAINS_TUBE_MAP: TubeMapData = {
  meta: { name: "No Domains App", version: "1.0", description: "Manifest with subsystems but no domains field" },
  domains: {},
  subsystems: [
    makeSubsystem({ id: "auth", name: "Authentication", domain: "unknown-domain", description: "Auth without a declared domain" }),
    makeSubsystem({ id: "billing", name: "Billing", domain: "also-unknown", status: "new", description: "Billing without a declared domain" }),
    makeSubsystem({ id: "notifications", name: "Notifications", domain: "", status: "rebuild", description: "Notifications with empty domain" }),
  ],
  connections: [
    { from: "billing", to: "auth", label: "validates user", type: "depends_on" },
    { from: "notifications", to: "auth", label: "resolves recipient", type: "depends_on" },
  ],
  coverage_percent: 0,
  unmatched_count: 0,
};

/** TubeMapData matching orphan-subsystems.json: 4 subsystems, only 1 has a valid domain */
export const MOCK_ORPHAN_SUBSYSTEMS_TUBE_MAP: TubeMapData = {
  meta: { name: "Orphan Subsystems App", version: "1.0", description: "Subsystems whose domain doesn't exist in domains map" },
  domains: {
    core: { label: "Core", color: "#3b82f6" },
  },
  subsystems: [
    makeSubsystem({ id: "auth", name: "Authentication", domain: "core", description: "Belongs to a valid domain" }),
    makeSubsystem({ id: "payments", name: "Payments", domain: "billing-domain", description: "References a domain that doesn't exist" }),
    makeSubsystem({ id: "analytics", name: "Analytics", domain: "data-science", status: "new", description: "Another orphan domain reference" }),
    makeSubsystem({ id: "mailer", name: "Email Service", domain: "", status: "rebuild", description: "Empty domain string — should go to unassigned" }),
  ],
  connections: [
    { from: "payments", to: "auth", label: "validates identity", type: "depends_on" },
    { from: "mailer", to: "auth", label: "resolves recipient", type: "depends_on" },
  ],
  coverage_percent: 0,
  unmatched_count: 0,
};

/** SystemManifest returned by bootstrap_manifest: 2 domains, 3 subsystems, 2 connections */
export const MOCK_BOOTSTRAP_MANIFEST: SystemManifest = {
  meta: { name: "test-project", version: "1.0.0", description: "Auto-detected from scan" },
  domains: {
    core: { label: "Core", color: "#3b82f6" },
    services: { label: "Services", color: "#22c55e" },
  },
  subsystems: [
    {
      id: "auth",
      name: "Authentication",
      domain: "core",
      status: "built",
      filePath: "src/auth/",
      interfaces: ["AuthProvider"],
      operations: ["login", "logout"],
      tables: [],
      events: [],
      children: [],
      dependencies: [],
    },
    {
      id: "api",
      name: "API Gateway",
      domain: "core",
      status: "built",
      filePath: "src/api/",
      interfaces: ["ApiRouter"],
      operations: ["handleRequest"],
      tables: [],
      events: [],
      children: [],
      dependencies: ["auth"],
    },
    {
      id: "user-service",
      name: "User Service",
      domain: "services",
      status: "new",
      filePath: "src/services/user/",
      interfaces: ["UserService"],
      operations: ["getUser", "createUser"],
      tables: ["users"],
      events: [],
      children: [],
      dependencies: ["auth"],
    },
  ],
  connections: [
    { from: "api", to: "auth", label: "validates identity", type: "depends_on" },
    { from: "user-service", to: "auth", label: "authenticates users", type: "depends_on" },
  ],
};

/** TubeMapData matching empty.json: valid manifest with 0 subsystems */
export const MOCK_EMPTY_TUBE_MAP: TubeMapData = {
  meta: { name: "Empty App", version: "1.0", description: "Valid manifest with zero subsystems" },
  domains: {},
  subsystems: [],
  connections: [],
  coverage_percent: 0,
  unmatched_count: 0,
};

// ---------------------------------------------------------------------------
// Mock IPC injection
// ---------------------------------------------------------------------------

export interface MockIPCOptions {
  /** Path the dialog:open mock returns (null = user cancelled). */
  dialogResult?: string | null;
  /** Path the dialog:save mock returns (null = user cancelled). */
  saveDialogResult?: string | null;
  /** Stats returned by scan_directory. Null = throw error. */
  scanStats?: ScanStats | null;
  /** Error message thrown by scan_directory (when scanStats is null). */
  scanError?: string;
  /** Entities returned by filter_entities / search_entities. */
  entities?: EntitySummary[];
  /** TubeMapData returned by get_tube_map_data. Null uses default empty. */
  tubeMapData?: TubeMapData | null;
  /** Error thrown by load_manifest (if set, load_manifest rejects). */
  manifestError?: string;
  /** Match result returned by match_manifest. Null = throw (no scan loaded). */
  matchResult?: { matched: string[]; unmatched: string[]; coverage_percent: number } | null;
  /** SystemManifest returned by bootstrap_manifest. Null = throw (no scan loaded). */
  bootstrapResult?: SystemManifest | null;
  /** Error thrown by bootstrap_manifest (if set, rejects). */
  bootstrapError?: string;
  /** Error thrown by save_manifest (if set, rejects). */
  saveManifestError?: string;
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
  const saveDialogResult = options.saveDialogResult !== undefined ? options.saveDialogResult : "/mock/output/system.json";
  const scanStats = options.scanStats !== undefined ? options.scanStats : MOCK_SCAN_STATS;
  const scanError = options.scanError !== undefined ? options.scanError : "Scan failed";
  const entities = options.entities !== undefined ? options.entities : MOCK_ENTITIES;
  const tubeMapData = options.tubeMapData !== undefined ? options.tubeMapData : null;
  const manifestError = options.manifestError ?? null;
  const matchResult = options.matchResult !== undefined ? options.matchResult : { matched: [], unmatched: [], coverage_percent: 0 };
  const bootstrapResult = options.bootstrapResult !== undefined ? options.bootstrapResult : null;
  const bootstrapError = options.bootstrapError ?? null;
  const saveManifestError = options.saveManifestError ?? null;

  // Serialize data for injection into browser context
  const serialized = JSON.stringify({
    dialogResult,
    saveDialogResult,
    scanStats,
    scanError,
    entities,
    tubeMapData,
    manifestError,
    matchResult,
    bootstrapResult,
    bootstrapError,
    saveManifestError,
  });

  await page.addInitScript((data: string) => {
    const config = JSON.parse(data);

    // Initialize __TAURI_INTERNALS__ (same structure as @tauri-apps/api/mocks)
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w = window as any;
    w.__TAURI_INTERNALS__ = w.__TAURI_INTERNALS__ ?? {};
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ =
      w.__TAURI_EVENT_PLUGIN_INTERNALS__ ?? {};

    // Mutable tube map state (can be updated mid-test via page.evaluate)
    w.__MOCK_TUBE_MAP__ = {
      data: config.tubeMapData,
      manifestError: config.manifestError,
      matchResult: config.matchResult,
      bootstrapResult: config.bootstrapResult,
      bootstrapError: config.bootstrapError,
      saveManifestError: config.saveManifestError,
      savedManifests: [] as Array<{ manifest: unknown; path: string }>,
    };

    const internals = w.__TAURI_INTERNALS__ as Record<string, unknown>;
    const callbacks = new Map<number, (data: unknown) => unknown>();

    internals.invoke = async (cmd: string, args?: Record<string, unknown>) => {
      // Dialog plugin: open file/directory picker
      if (cmd === "plugin:dialog|open") {
        return config.dialogResult;
      }

      // Dialog plugin: save file picker
      if (cmd === "plugin:dialog|save") {
        return config.saveDialogResult;
      }

      // Bootstrap manifest (wizard auto-detect)
      if (cmd === "bootstrap_manifest") {
        if (w.__MOCK_TUBE_MAP__?.bootstrapError) {
          throw new Error(w.__MOCK_TUBE_MAP__.bootstrapError);
        }
        if (w.__MOCK_TUBE_MAP__?.bootstrapResult) {
          return w.__MOCK_TUBE_MAP__.bootstrapResult;
        }
        throw new Error("No scan index loaded");
      }

      // Save manifest (wizard save)
      if (cmd === "save_manifest") {
        if (w.__MOCK_TUBE_MAP__?.saveManifestError) {
          throw new Error(w.__MOCK_TUBE_MAP__.saveManifestError);
        }
        const manifest = args?.manifestJson as Record<string, unknown> | undefined;
        // Store the saved manifest for test assertions
        w.__MOCK_TUBE_MAP__?.savedManifests?.push({
          manifest,
          path: args?.path,
        });
        // After save, update tube map data so get_tube_map_data returns the wizard output
        if (manifest) {
          const subs = (manifest.subsystems as Array<Record<string, unknown>>) ?? [];
          w.__MOCK_TUBE_MAP__.data = {
            meta: manifest.meta,
            domains: manifest.domains,
            subsystems: subs.map((s: Record<string, unknown>) => ({
              id: s.id, name: s.name, domain: s.domain, status: s.status ?? "new",
              description: "", file_path: s.filePath ?? "",
              matched_entity_count: 0, interface_count: ((s.interfaces as string[]) ?? []).length,
              operation_count: ((s.operations as string[]) ?? []).length,
              table_count: ((s.tables as string[]) ?? []).length,
              event_count: ((s.events as string[]) ?? []).length,
              has_children: ((s.children as unknown[]) ?? []).length > 0,
              child_count: ((s.children as unknown[]) ?? []).length,
              dependency_count: ((s.dependencies as string[]) ?? []).length,
            })),
            connections: manifest.connections,
            coverage_percent: 0,
            unmatched_count: 0,
          };
        }
        return null;
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

      // Get entity detail — return a stub with file-aware span
      if (cmd === "get_entity_detail") {
        const name = args?.name as string;
        const file = (args?.file as string) ?? "unknown.ts";
        // Return different spans per entity for testing scroll-to-line
        const spanMap: Record<string, { start_line: number; end_line: number }> = {
          AuthProvider: { start_line: 2, end_line: 6 },
          UserService: { start_line: 12, end_line: 16 },
          ApiRouter: { start_line: 1, end_line: 5 },
          getUser: { start_line: 8, end_line: 10 },
          UserSchema: { start_line: 1, end_line: 8 },
        };
        const span = spanMap[name] ?? { start_line: 1, end_line: 10 };
        return {
          Interface: {
            name,
            file,
            span: { start_line: span.start_line, start_col: 0, end_line: span.end_line, end_col: 0, byte_range: [0, 100] },
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

      // Get full file source (for Monaco editor)
      if (cmd === "get_file_source") {
        const file = args?.file as string ?? "unknown.ts";
        // Large file mock for stress testing (1500 lines)
        if (file.includes("large-file")) {
          return Array.from({ length: 1500 }, (_, i) =>
            `// line ${i + 1}: generated content for stress testing`,
          ).join("\n");
        }
        // Rust file mock
        if (file.endsWith(".rs")) {
          return [
            "use std::collections::HashMap;",
            "",
            "pub trait AuthProvider {",
            "    fn login(&self, username: &str, password: &str) -> Result<bool>;",
            "    fn logout(&mut self);",
            "    fn get_user(&self) -> Option<&User>;",
            "}",
            "",
            "pub struct User {",
            "    pub id: String,",
            "    pub name: String,",
            "}",
          ].join("\n");
        }
        // Default TypeScript file mock
        return [
          "// mock source code for " + file,
          "export interface MockEntity {",
          "  id: string;",
          "  name: string;",
          "  email: string;",
          "}",
          "",
          "export function getUser(): MockEntity | null {",
          "  return null;",
          "}",
          "",
          "export class UserService {",
          "  async findById(id: string): Promise<MockEntity | null> {",
          "    return null;",
          "  }",
          "}",
        ].join("\n");
      }

      // Get entity source (legacy byte-range extraction)
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

      // Tube map commands — use configurable mock data
      if (cmd === "load_manifest") {
        if (w.__MOCK_TUBE_MAP__?.manifestError) {
          throw new Error(w.__MOCK_TUBE_MAP__.manifestError);
        }
        const data = w.__MOCK_TUBE_MAP__?.data;
        return data
          ? { meta: data.meta, domains: data.domains, subsystems: data.subsystems, connections: data.connections }
          : { meta: { name: "Test", version: "1.0", description: "Test manifest" }, domains: {}, subsystems: [], connections: [] };
      }
      if (cmd === "match_manifest") {
        if (w.__MOCK_TUBE_MAP__?.matchResult === null) {
          throw new Error("No scan index loaded");
        }
        return w.__MOCK_TUBE_MAP__?.matchResult ?? { matched: [], unmatched: [], coverage_percent: 0 };
      }
      if (cmd === "get_tube_map_data") {
        return w.__MOCK_TUBE_MAP__?.data ?? {
          meta: { name: "Test", version: "1.0", description: "Test" },
          domains: {},
          subsystems: [],
          connections: [],
          coverage_percent: 0,
          unmatched_count: 0,
        };
      }
      if (cmd === "get_subsystem_detail") {
        return { id: args?.subsystemId, name: "Mock Subsystem", domain: "core", status: "built", file_path: "src/mock/", interfaces: [], operations: [], tables: [], events: [], dependencies: [], children: [], matched_entities: [] };
      }
      if (cmd === "get_subsystem_entities") {
        return [];
      }

      // Check if a scan is loaded (for the tube map scan gate)
      if (cmd === "get_current_scan") {
        return config.scanStats;
      }

      // Platform release info (for the agent prompt)
      if (cmd === "get_platform_release_info") {
        return {
          os: "darwin",
          arch: "aarch64",
          latest_tag: "v0.4.0",
          assets: [{ name: "domain-scan-darwin-aarch64.tar.gz", download_url: "https://example.com/domain-scan.tar.gz", size: 5000000 }],
          matching_asset: { name: "domain-scan-darwin-aarch64.tar.gz", download_url: "https://example.com/domain-scan.tar.gz", size: 5000000 },
          cargo_install_cmd: "cargo install --force domain-scan-cli",
          recommended_install_cmd: "curl -sL \"https://example.com/domain-scan.tar.gz\" -o /tmp/domain-scan.tar.gz\nmkdir -p ~/.local/bin",
          recommended_update_cmd: "curl -sL \"https://example.com/domain-scan.tar.gz\" -o /tmp/domain-scan.tar.gz\nmkdir -p ~/.local/bin",
          scanned_root: config.dialogResult,
          installed_path: "/Users/test/.local/bin/domain-scan",
          installed_version: "0.4.0",
          doctor_supported: true,
          update_available: false,
        };
      }

      // Open in editor
      if (cmd === "open_in_editor") {
        throw new Error("No editor available in test mode");
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
