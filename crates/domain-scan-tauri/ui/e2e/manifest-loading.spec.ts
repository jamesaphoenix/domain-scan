/**
 * Phase F.4: E2E tests for Manifest Loading & Matching.
 *
 * Tests run against the Vite dev server with Tauri IPC mocked
 * via `setupTauriMocks()` (injected before page load).
 */

import { test, expect } from "@playwright/test";
import {
  setupTauriMocks,
  MOCK_SCAN_STATS,
  MOCK_ENTITIES,
  MOCK_MINIMAL_TUBE_MAP,
  MOCK_OCTOSPARK_TUBE_MAP,
  MOCK_EMPTY_TUBE_MAP,
} from "./mocks";
import {
  waitForAppReady,
  switchTab,
  clickLoadManifest,
  waitForTubeMap,
  countVisibleNodes,
  assertManifestLoaderVisible,
  clickOpenDirectory,
} from "./helpers";

test.describe("F.4: Manifest Loading & Matching", () => {
  test("load minimal.json → 2 subsystem nodes render on canvas", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_MINIMAL_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Switch to Tube Map tab
    await switchTab(page, "Subsystem Tube Map");
    await assertManifestLoaderVisible(page);

    // Click "Load Manifest" — mock dialog returns a path, mock IPC returns data
    await clickLoadManifest(page);

    // Wait for the React Flow canvas to appear
    await waitForTubeMap(page);

    // Verify 2 nodes render (one per subsystem)
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(2);

    // Verify subsystem names are visible in node headings (not in dropdowns)
    await expect(
      page.getByRole("heading", { name: "Authentication" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "API Gateway" }),
    ).toBeVisible();

    // Verify the connection edge is rendered
    const edges = page.locator(".react-flow__edge");
    await expect(edges.first()).toBeVisible();
  });

  test("load octospark-system.json → 18 nodes render, edges visible", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Verify 18 nodes render
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(18);

    // Verify some subsystem names are visible in node headings
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Billing & Subscriptions" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Social Media Publisher" }),
    ).toBeVisible();

    // Verify edges are rendered (may be bundled, so check at least some exist)
    const edges = page.locator(".react-flow__edge");
    const edgeCount = await edges.count();
    expect(edgeCount).toBeGreaterThan(0);

    // Verify status bar shows subsystem count (use .first() since toast also contains this text)
    await expect(page.getByText("18 subsystems").first()).toBeVisible();
  });

  test("load empty.json → 'No subsystems found' message, no crash", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_EMPTY_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);

    // Should show "No subsystems found" message instead of an empty canvas
    await expect(page.getByText("No subsystems found")).toBeVisible({
      timeout: 10_000,
    });

    // Should still offer a way to load a different manifest
    await expect(page.getByText(/load different manifest/i)).toBeVisible();

    // App should remain stable — tabs present
    await expect(
      page.getByRole("button", { name: "Entities/Types" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Subsystem Tube Map" }),
    ).toBeVisible();
  });

  test("load malformed.json → structured error toast, tube map stays on loader view", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      manifestError:
        "Failed to parse manifest: expected value at line 1 column 3",
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await assertManifestLoaderVisible(page);

    // Click "Load Manifest" — mock will throw an error
    await clickLoadManifest(page);

    // Error should be displayed in the ManifestLoader error area (use first match
    // since the text also appears in the toast notification)
    await expect(
      page.getByText(/failed to parse manifest/i).first(),
    ).toBeVisible({ timeout: 10_000 });

    // Tube map should stay on loader view (no React Flow canvas)
    await expect(page.locator(".react-flow")).not.toBeVisible();

    // "Open Manifest" button should still be available for retry
    await expect(
      page.getByRole("button", { name: /open manifest/i }),
    ).toBeVisible();
  });

  test("load manifest before scan → matching skipped gracefully, entities show as unmatched", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      scanStats: null,
      tubeMapData: MOCK_MINIMAL_TUBE_MAP,
      matchResult: null,
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Load manifest without scanning first
    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);

    // Tube map should still render (matching is skipped gracefully)
    await waitForTubeMap(page);

    // Verify nodes render
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(2);

    // Subsystems should be visible in node headings
    await expect(
      page.getByRole("heading", { name: "Authentication" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "API Gateway" }),
    ).toBeVisible();
  });

  test("load manifest after scan → matching runs, coverage % shown", async ({
    page,
  }) => {
    const tubeMapWithCoverage = {
      ...MOCK_MINIMAL_TUBE_MAP,
      coverage_percent: 75,
      unmatched_count: 3,
      subsystems: MOCK_MINIMAL_TUBE_MAP.subsystems.map((s) => ({
        ...s,
        matched_entity_count: 2,
      })),
    };

    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: MOCK_ENTITIES,
      tubeMapData: tubeMapWithCoverage,
      matchResult: {
        matched: ["AuthProvider", "UserService"],
        unmatched: ["ApiRouter"],
        coverage_percent: 75,
      },
    });

    await page.goto("/");
    await waitForAppReady(page);

    // First, perform a scan
    await clickOpenDirectory(page);
    await expect(
      page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
    ).toBeVisible({ timeout: 10_000 });

    // Then switch to Tube Map and load manifest
    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Verify nodes render
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(2);

    // Verify coverage % is shown somewhere on the page
    // CoverageOverlay and TubeMapStatusBar both display coverage
    await expect(page.getByText(/75(\.0)?%/).first()).toBeVisible({
      timeout: 5_000,
    });
  });

  test("reload different manifest → old match results cleared, new data renders", async ({
    page,
  }) => {
    // Start with minimal manifest (2 subsystems)
    await setupTauriMocks(page, {
      tubeMapData: MOCK_MINIMAL_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Verify initial state: 2 nodes
    let nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(2);
    await expect(
      page.getByRole("heading", { name: "Authentication" }),
    ).toBeVisible();

    // Update mock data mid-test to simulate loading a different manifest
    await page.evaluate((newData) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (window as any).__MOCK_TUBE_MAP__.data = newData;
    }, {
      meta: {
        name: "Second Manifest",
        version: "2.0",
        description: "Different manifest",
      },
      domains: {
        backend: { label: "Backend", color: "#22c55e" },
        frontend: { label: "Frontend", color: "#3b82f6" },
      },
      subsystems: [
        {
          id: "server",
          name: "Server Core",
          domain: "backend",
          status: "built",
          description: "Main server",
          file_path: "src/server/",
          matched_entity_count: 0,
          interface_count: 0,
          operation_count: 0,
          table_count: 0,
          event_count: 0,
          has_children: false,
          child_count: 0,
          dependency_count: 0,
        },
        {
          id: "database",
          name: "Database Layer",
          domain: "backend",
          status: "built",
          description: "DB access",
          file_path: "src/db/",
          matched_entity_count: 0,
          interface_count: 0,
          operation_count: 0,
          table_count: 0,
          event_count: 0,
          has_children: false,
          child_count: 0,
          dependency_count: 0,
        },
        {
          id: "webapp",
          name: "Web Application",
          domain: "frontend",
          status: "built",
          description: "React app",
          file_path: "src/web/",
          matched_entity_count: 0,
          interface_count: 0,
          operation_count: 0,
          table_count: 0,
          event_count: 0,
          has_children: false,
          child_count: 0,
          dependency_count: 1,
        },
      ],
      connections: [
        {
          from: "webapp",
          to: "server",
          label: "calls API",
          type: "depends_on",
        },
      ],
      coverage_percent: 0,
      unmatched_count: 0,
    });

    // Click "Reload" button to load the new manifest
    await page.getByRole("button", { name: /reload/i }).click();

    // Wait for new data to render
    await page.waitForTimeout(500);

    // Verify new manifest renders: 3 nodes now
    nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(3);

    // Old subsystem names should be gone from node headings
    await expect(
      page.getByRole("heading", { name: "Authentication" }),
    ).not.toBeVisible();

    // New subsystem names should be visible in node headings
    await expect(
      page.getByRole("heading", { name: "Server Core" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Database Layer" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Web Application" }),
    ).toBeVisible();

    // Verify new manifest name is shown
    await expect(page.getByText("Second Manifest")).toBeVisible();
  });
});
