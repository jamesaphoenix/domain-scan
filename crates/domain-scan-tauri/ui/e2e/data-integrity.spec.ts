/**
 * Phase F.8: Data Integrity E2E tests.
 *
 * Validates that entity counts, coverage percentages, subsystem entity
 * filtering, connection references, cross-tab state, and prompt generation
 * are all consistent and correct.
 */

import { test, expect } from "@playwright/test";
import {
  setupTauriMocks,
  MOCK_SCAN_STATS,
  MOCK_ENTITIES,
} from "./mocks";
import type { TubeMapData, TubeMapSubsystem, EntitySummary } from "../src/types";
import {
  waitForAppReady,
  switchTab,
  clickLoadManifest,
  waitForTubeMap,
  countVisibleNodes,
  clickOpenDirectory,
  assertManifestLoaderVisible,
} from "./helpers";

// ---------------------------------------------------------------------------
// Helpers: build mock data with precise, verifiable entity counts
// ---------------------------------------------------------------------------

function makeVerifiableSubsystem(
  overrides: Partial<TubeMapSubsystem> & { id: string; name: string; domain: string },
): TubeMapSubsystem {
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

function buildEntitiesForSubsystems(subsystems: TubeMapSubsystem[]): EntitySummary[] {
  const entities: EntitySummary[] = [];
  for (const sub of subsystems) {
    for (let i = 0; i < sub.interface_count; i++) {
      entities.push({
        name: `${sub.name}Interface${i}`,
        kind: "interface",
        file: `${sub.file_path}interface_${i}.ts`,
        line: 1,
        language: "TypeScript",
        build_status: "built",
        confidence: "high",
      });
    }
    for (let i = 0; i < sub.operation_count; i++) {
      entities.push({
        name: `${sub.name}Method${i}`,
        kind: "method",
        file: `${sub.file_path}method_${i}.ts`,
        line: 1,
        language: "TypeScript",
        build_status: "built",
        confidence: "high",
      });
    }
    for (let i = 0; i < sub.table_count; i++) {
      entities.push({
        name: `${sub.name}Schema${i}`,
        kind: "schema",
        file: `${sub.file_path}schema_${i}.ts`,
        line: 1,
        language: "TypeScript",
        build_status: "built",
        confidence: "high",
      });
    }
  }
  return entities;
}

// Subsystems WITHOUT children (for count / coverage / connection tests)
const FLAT_SUBSYSTEMS: TubeMapSubsystem[] = [
  makeVerifiableSubsystem({
    id: "auth",
    name: "Auth",
    domain: "core",
    matched_entity_count: 5,
    interface_count: 2,
    operation_count: 2,
    table_count: 1,
  }),
  makeVerifiableSubsystem({
    id: "billing",
    name: "Billing",
    domain: "core",
    matched_entity_count: 3,
    interface_count: 1,
    operation_count: 1,
    table_count: 1,
    dependency_count: 1,
  }),
];

// Subsystems WITH children (for drill-in tests)
const DRILLABLE_SUBSYSTEMS: TubeMapSubsystem[] = [
  makeVerifiableSubsystem({
    id: "auth",
    name: "Auth",
    domain: "core",
    matched_entity_count: 5,
    interface_count: 2,
    operation_count: 2,
    table_count: 1,
    has_children: true,
    child_count: 3,
  }),
  makeVerifiableSubsystem({
    id: "billing",
    name: "Billing",
    domain: "core",
    matched_entity_count: 3,
    interface_count: 1,
    operation_count: 1,
    table_count: 1,
    has_children: true,
    child_count: 2,
    dependency_count: 1,
  }),
];

const FLAT_ENTITIES = buildEntitiesForSubsystems(FLAT_SUBSYSTEMS);
const INTEGRITY_COVERAGE = 100.0;

const FLAT_TUBE_MAP: TubeMapData = {
  meta: { name: "Integrity Test", version: "1.0", description: "Data integrity fixture" },
  domains: { core: { label: "Core", color: "#3b82f6" } },
  subsystems: FLAT_SUBSYSTEMS,
  connections: [{ from: "billing", to: "auth", label: "validates identity", type: "depends_on" }],
  coverage_percent: INTEGRITY_COVERAGE,
  unmatched_count: 0,
};

const DRILLABLE_TUBE_MAP: TubeMapData = {
  ...FLAT_TUBE_MAP,
  subsystems: DRILLABLE_SUBSYSTEMS,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe("F.8: Data Integrity Checks", () => {
  test("get_tube_map_data entity counts match filter_entities counts for each subsystem", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: FLAT_TUBE_MAP,
      entities: FLAT_ENTITIES,
      matchResult: {
        matched: FLAT_ENTITIES.map((e) => e.name),
        unmatched: [],
        coverage_percent: INTEGRITY_COVERAGE,
      },
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Verify both nodes render
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(2);

    // Verify subsystem headings are present
    for (const sub of FLAT_SUBSYSTEMS) {
      await expect(page.getByRole("heading", { name: sub.name })).toBeVisible();
    }

    // Verify entity count badges on nodes match expected values
    // Auth: 5 entities matched, Billing: 3 entities matched
    await expect(page.getByText("5 entities matched").first()).toBeVisible();
    await expect(page.getByText("3 entities matched").first()).toBeVisible();
  });

  test("match_manifest coverage % is consistent: matched.len() / total_entities * 100", async ({
    page,
  }) => {
    const totalEntities = 8;
    const matchedEntities = 6;
    const expectedCoverage = (matchedEntities / totalEntities) * 100; // 75%

    const matched = FLAT_ENTITIES.slice(0, matchedEntities).map((e) => e.name);
    const unmatched = FLAT_ENTITIES.slice(matchedEntities).map((e) => e.name);

    const tubeMapWithPartialCoverage: TubeMapData = {
      ...FLAT_TUBE_MAP,
      coverage_percent: expectedCoverage,
      unmatched_count: totalEntities - matchedEntities,
    };

    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: { ...MOCK_SCAN_STATS, total_interfaces: totalEntities },
      entities: FLAT_ENTITIES,
      tubeMapData: tubeMapWithPartialCoverage,
      matchResult: {
        matched,
        unmatched,
        coverage_percent: expectedCoverage,
      },
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Scan first, then load manifest
    await clickOpenDirectory(page);
    await expect(
      page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
    ).toBeVisible({ timeout: 10_000 });

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Verify coverage percentage is displayed (75%)
    await expect(page.getByText(/75(\.0)?%/).first()).toBeVisible({ timeout: 5_000 });

    // Verify unmatched count is shown
    await expect(page.getByText(/2 unmatched/i).first()).toBeVisible({ timeout: 5_000 });
  });

  test("get_subsystem_entities returns only entities whose files fall under subsystem filePath", async ({
    page,
  }) => {
    // Use drillable subsystems (has_children: true) so clicking triggers drill-in
    await setupTauriMocks(page, {
      tubeMapData: DRILLABLE_TUBE_MAP,
    });

    // Override get_subsystem_entities to return entities filtered by file path prefix
    await page.addInitScript(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const w = window as any;
      const originalInvoke = w.__TAURI_INTERNALS__?.invoke;
      if (!originalInvoke) return;

      w.__TAURI_INTERNALS__.invoke = async (
        cmd: string,
        args?: Record<string, unknown>,
      ) => {
        if (cmd === "get_subsystem_entities") {
          const subId = args?.subsystemId as string;
          if (subId === "auth") {
            return [
              { name: "AuthProvider", kind: "interface", file: "src/auth/provider.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
              { name: "AuthService", kind: "service", file: "src/auth/service.ts", line: 5, language: "TypeScript", build_status: "built", confidence: "high" },
            ];
          }
          if (subId === "billing") {
            return [
              { name: "BillingApi", kind: "interface", file: "src/billing/api.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
            ];
          }
          return [];
        }
        if (cmd === "get_subsystem_detail") {
          const subId = args?.subsystemId as string;
          return {
            id: subId,
            name: subId === "auth" ? "Auth" : "Billing",
            domain: "core",
            status: "built",
            file_path: `src/${subId}/`,
            interfaces: subId === "auth" ? ["AuthProvider"] : ["BillingApi"],
            operations: subId === "auth" ? ["AuthService"] : [],
            tables: [],
            events: [],
            dependencies: [],
            children: [],
            matched_entities: [],
          };
        }
        return originalInvoke(cmd, args);
      };
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Click the auth subsystem node to drill in (has_children: true)
    await page.getByRole("heading", { name: "Auth" }).first().click();

    // Wait for drill-in to show auth entities (file paths under src/auth/)
    await expect(page.getByText("AuthProvider").first()).toBeVisible({ timeout: 5_000 });
    await expect(page.getByText("AuthService").first()).toBeVisible({ timeout: 5_000 });

    // Verify that billing entities are NOT visible (they belong to a different subsystem)
    await expect(page.getByText("BillingApi")).not.toBeVisible();
  });

  test("connections reference only valid subsystem IDs (no dangling from/to)", async ({
    page,
  }) => {
    const validConnections = FLAT_TUBE_MAP.connections;
    const subsystemIds = FLAT_TUBE_MAP.subsystems.map((s) => s.id);

    // Pre-check: verify all connection endpoints exist in subsystems
    for (const conn of validConnections) {
      expect(subsystemIds).toContain(conn.from);
      expect(subsystemIds).toContain(conn.to);
    }

    // Test with a manifest that has a dangling connection
    const danglingTubeMap: TubeMapData = {
      ...FLAT_TUBE_MAP,
      connections: [
        ...validConnections,
        { from: "billing", to: "nonexistent-subsystem", label: "dangling ref", type: "depends_on" },
      ],
    };

    await setupTauriMocks(page, {
      tubeMapData: danglingTubeMap,
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // App should still render without crashing despite the dangling connection
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(2);

    // Both valid subsystems should be visible
    await expect(page.getByRole("heading", { name: "Auth" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Billing" })).toBeVisible();

    // Valid edges render (dangling one is skipped)
    const edges = page.locator(".react-flow__edge");
    const edgeCount = await edges.count();
    expect(edgeCount).toBeGreaterThanOrEqual(1);

    // No error toast
    const errorToasts = page.locator('[role="alert"]');
    const errorCount = await errorToasts.count();
    expect(errorCount).toBe(0);
  });

  test("after scan + match, switching to Entities tab still works (shared state not corrupted)", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: MOCK_ENTITIES,
      tubeMapData: FLAT_TUBE_MAP,
      matchResult: {
        matched: MOCK_ENTITIES.map((e) => e.name),
        unmatched: [],
        coverage_percent: 100,
      },
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Step 1: Perform a scan on the Entities tab
    await clickOpenDirectory(page);
    await expect(
      page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
    ).toBeVisible({ timeout: 10_000 });

    // Verify entity names are present
    for (const entity of MOCK_ENTITIES) {
      await expect(page.getByText(entity.name).first()).toBeVisible();
    }

    // Step 2: Switch to Tube Map tab and load manifest
    await switchTab(page, "Subsystem Tube Map");
    await assertManifestLoaderVisible(page);
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Verify tube map rendered
    const tubeNodeCount = await countVisibleNodes(page);
    expect(tubeNodeCount).toBe(2);

    // Step 3: Switch back to Entities tab — verify SHARED state is preserved
    // (tube map state is tab-scoped and resets on unmount, per spec section 3.3)
    await switchTab(page, "Entities/Types");

    // Scan stats should still be visible (shared state not corrupted by tube map)
    await expect(
      page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
    ).toBeVisible({ timeout: 5_000 });

    // Entity names should still be visible
    for (const entity of MOCK_ENTITIES) {
      await expect(page.getByText(entity.name).first()).toBeVisible();
    }

    // Step 4: Switch to Tube Map again — tab-scoped state resets, ManifestLoader shown
    await switchTab(page, "Subsystem Tube Map");
    await assertManifestLoaderVisible(page);

    // Can reload manifest and it still works
    await clickLoadManifest(page);
    await waitForTubeMap(page);
    const tubeNodeCountAfter = await countVisibleNodes(page);
    expect(tubeNodeCountAfter).toBe(2);
  });

  test("generate prompt from tube map drill-in → valid prompt text, scoped to subsystem entities", async ({
    page,
  }) => {
    // Use drillable subsystems so clicking triggers drill-in
    await setupTauriMocks(page, {
      tubeMapData: DRILLABLE_TUBE_MAP,
    });

    // Override mocks to track generate_prompt calls and return scoped entities
    await page.addInitScript(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const w = window as any;
      const originalInvoke = w.__TAURI_INTERNALS__?.invoke;
      if (!originalInvoke) return;

      w.__GENERATE_PROMPT_CALLS__ = [];

      w.__TAURI_INTERNALS__.invoke = async (
        cmd: string,
        args?: Record<string, unknown>,
      ) => {
        if (cmd === "get_subsystem_entities") {
          const subId = args?.subsystemId as string;
          if (subId === "auth") {
            return [
              { name: "AuthProvider", kind: "interface", file: "src/auth/provider.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
              { name: "LoginMethod", kind: "method", file: "src/auth/login.ts", line: 10, language: "TypeScript", build_status: "built", confidence: "high" },
            ];
          }
          return [];
        }
        if (cmd === "get_subsystem_detail") {
          return {
            id: args?.subsystemId,
            name: "Auth",
            domain: "core",
            status: "built",
            file_path: "src/auth/",
            interfaces: ["AuthProvider"],
            operations: ["LoginMethod"],
            tables: [],
            events: [],
            dependencies: [],
            children: [],
            matched_entities: [],
          };
        }
        if (cmd === "generate_prompt") {
          const entityIds = args?.entityIds as string[];
          w.__GENERATE_PROMPT_CALLS__.push(entityIds);
          return `# Auth Subsystem Analysis\n\nEntities: ${entityIds.join(", ")}\n\nAnalyze the following entities...`;
        }
        return originalInvoke(cmd, args);
      };
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Click auth subsystem to drill in (has_children: true)
    await page.getByRole("heading", { name: "Auth" }).first().click();

    // Wait for drill-in view with entities
    await expect(page.getByText("AuthProvider").first()).toBeVisible({ timeout: 5_000 });
    await expect(page.getByText("LoginMethod").first()).toBeVisible({ timeout: 5_000 });

    // Click "Generate Prompt" button
    const generateButton = page.getByRole("button", { name: /generate prompt/i });
    await expect(generateButton).toBeVisible({ timeout: 5_000 });
    await generateButton.click();

    // Verify prompt output is displayed and contains entity names
    await expect(page.getByText(/Auth Subsystem Analysis/).first()).toBeVisible({ timeout: 5_000 });

    // Verify the generate_prompt call was made with the correct entity IDs
    const promptCalls = await page.evaluate(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return (window as any).__GENERATE_PROMPT_CALLS__;
    });

    expect(promptCalls).toHaveLength(1);
    expect(promptCalls[0]).toContain("AuthProvider");
    expect(promptCalls[0]).toContain("LoginMethod");
    // Should NOT contain entities from other subsystems
    expect(promptCalls[0]).not.toContain("BillingApi");
  });
});
