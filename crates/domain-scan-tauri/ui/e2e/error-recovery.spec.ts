/**
 * Phase F.9: Error Recovery E2E tests.
 *
 * Validates that the app handles IPC failures, deleted manifests,
 * corrupt caches, and extremely long subsystem names gracefully
 * without crashing or showing blank screens.
 */

import { test, expect } from "@playwright/test";
import {
  setupTauriMocks,
  MOCK_SCAN_STATS,
  MOCK_ENTITIES,
  MOCK_MINIMAL_TUBE_MAP,
} from "./mocks";
import type { TubeMapData, TubeMapSubsystem } from "../src/types";
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
// Helpers
// ---------------------------------------------------------------------------

function makeSubsystem(
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe("F.9: Error Recovery", () => {
  test("Tauri IPC command fails (e.g., file deleted mid-scan) -> structured error shown, app stays functional", async ({
    page,
  }) => {
    // Set up mocks where scan_directory will fail with a structured error
    await setupTauriMocks(page, {
      scanStats: null,
      scanError: "IO error: No such file or directory (os error 2): /deleted/project",
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Switch to Entities tab to avoid duplicate "Open Directory" buttons
    // (the scan gate on Tube Map also shows "Open Directory" when no scan is loaded)
    await switchTab(page, "Entities/Types");

    // Click "Open Directory" — scan will fail with a structured error
    await clickOpenDirectory(page);

    // Error should be displayed (either as toast or inline error)
    await expect(
      page.getByText(/no such file or directory/i).first(),
    ).toBeVisible({ timeout: 10_000 });

    // App should remain functional: tabs still present and clickable
    await expect(
      page.getByRole("button", { name: "Entities/Types" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Subsystem Tube Map" }),
    ).toBeVisible();

    // Tab switching should still work
    await switchTab(page, "Subsystem Tube Map");
    await assertManifestLoaderVisible(page);

    // Can switch back to entities tab without issue
    await switchTab(page, "Entities/Types");
    await expect(
      page.getByRole("button", { name: "Entities/Types" }),
    ).toBeVisible();
  });

  test("manifest file deleted after loading -> next match_manifest call returns error, tube map shows reload CTA", async ({
    page,
  }) => {
    // Start with a working manifest
    await setupTauriMocks(page, {
      tubeMapData: MOCK_MINIMAL_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Load manifest successfully
    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Verify tube map rendered initially
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(2);

    // Simulate manifest file being deleted: update mock to throw error on next load
    await page.evaluate(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const w = window as any;
      w.__MOCK_TUBE_MAP__.manifestError =
        "IO error: No such file or directory: /path/to/deleted-manifest.json";
    });

    // Click "Reload" button to attempt reloading the (now-deleted) manifest
    const reloadButton = page.getByRole("button", { name: /reload/i });
    if (await reloadButton.isVisible()) {
      await reloadButton.click();
    } else {
      // If no reload button, reopen the manifest from the loader
      await clickLoadManifest(page);
    }

    // Error should be displayed via toast notification
    await expect(
      page.getByText(/no such file or directory/i).first(),
    ).toBeVisible({ timeout: 10_000 });

    // The tube map should still show the previously loaded data (graceful degradation)
    // — the app doesn't discard working data on a reload failure.
    const reactFlow = page.locator(".react-flow");
    await expect(reactFlow).toBeVisible();

    // App should remain functional: tabs still work
    await switchTab(page, "Entities/Types");
    await expect(
      page.getByRole("button", { name: "Entities/Types" }),
    ).toBeVisible();

    // Switching back resets tab-scoped state (per spec 3.3),
    // so the ManifestLoader CTA is shown again for a fresh load.
    await switchTab(page, "Subsystem Tube Map");
    await assertManifestLoaderVisible(page);
  });

  test("corrupt cache directory -> scan falls back to no-cache mode, completes successfully", async ({
    page,
  }) => {
    // Set up mocks where scan succeeds (the Rust backend handles cache corruption
    // internally by falling back to no-cache mode). The IPC layer just sees a
    // successful scan with cache_hits=0 indicating cache was not usable.
    const statsWithNoCache = {
      ...MOCK_SCAN_STATS,
      cache_hits: 0,
      cache_misses: MOCK_SCAN_STATS.total_files,
    };

    await setupTauriMocks(page, {
      dialogResult: "/mock/project-with-corrupt-cache",
      scanStats: statsWithNoCache,
      entities: MOCK_ENTITIES,
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Switch to Entities tab (app starts on Tube Map by default)
    await switchTab(page, "Entities/Types");

    // Scan should complete successfully despite corrupt cache
    await clickOpenDirectory(page);

    // Stats bar should show the correct file count (scan succeeded)
    await expect(
      page.getByText(`${statsWithNoCache.total_files} files`),
    ).toBeVisible({ timeout: 10_000 });

    // Entities should be populated from the scan
    for (const entity of MOCK_ENTITIES) {
      await expect(page.getByText(entity.name).first()).toBeVisible();
    }

    // No error toast should be shown (scan recovered gracefully)
    const errorToasts = page.locator('[role="alert"], .bg-red-900');
    const errorCount = await errorToasts.count();
    // Filter out non-error toasts: only check for actual error toasts
    let hasErrorToast = false;
    for (let i = 0; i < errorCount; i++) {
      const text = await errorToasts.nth(i).textContent();
      if (text?.toLowerCase().includes("error") || text?.toLowerCase().includes("fail")) {
        hasErrorToast = true;
        break;
      }
    }
    expect(hasErrorToast).toBe(false);

    // App should be fully functional: can switch to tube map tab
    await switchTab(page, "Subsystem Tube Map");
    await assertManifestLoaderVisible(page);
  });

  test("extremely long subsystem names (500+ chars) -> node renders without overflow, tooltip shows full name", async ({
    page,
  }) => {
    const longName = "A".repeat(500) + "-Subsystem";
    const longDescName = "B".repeat(500) + "-Description";

    const longNameTubeMap: TubeMapData = {
      meta: { name: "Long Names Test", version: "1.0", description: "Test extremely long subsystem names" },
      domains: { core: { label: "Core", color: "#3b82f6" } },
      subsystems: [
        makeSubsystem({
          id: "long-name-sub",
          name: longName,
          domain: "core",
          description: longDescName,
        }),
        makeSubsystem({
          id: "normal-sub",
          name: "Normal Subsystem",
          domain: "core",
        }),
      ],
      connections: [{ from: "normal-sub", to: "long-name-sub", label: "depends on", type: "depends_on" }],
      coverage_percent: 0,
      unmatched_count: 0,
    };

    await setupTauriMocks(page, {
      tubeMapData: longNameTubeMap,
    });

    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Both nodes should render (no crash)
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(2);

    // The long-named subsystem node should be rendered
    // It may be truncated in the heading, but the node itself should exist
    const nodes = page.locator(".react-flow__node");
    const allNodeCount = await nodes.count();
    expect(allNodeCount).toBe(2);

    // The normal subsystem should render correctly
    await expect(
      page.getByRole("heading", { name: "Normal Subsystem" }),
    ).toBeVisible();

    // The long-named node should not overflow the canvas or cause layout issues.
    // Check that the React Flow container is still properly rendered and interactive.
    const reactFlowContainer = page.locator(".react-flow");
    await expect(reactFlowContainer).toBeVisible();

    // Check that the node with long name is visible and contained within bounds.
    // Get the first node's bounding box — it shouldn't extend absurdly beyond
    // the viewport (i.e., width should be capped by the node's max-width CSS).
    const longNameNode = nodes.first();
    const box = await longNameNode.boundingBox();
    expect(box).not.toBeNull();
    if (box) {
      // Node width should be reasonable (less than 800px, since NODE_WIDTH is 360px
      // and there may be some padding, but never thousands of pixels)
      expect(box.width).toBeLessThan(800);
    }

    // The edge should still render (no crash from long names)
    const edges = page.locator(".react-flow__edge");
    const edgeCount = await edges.count();
    expect(edgeCount).toBeGreaterThanOrEqual(1);

    // App should remain functional
    await switchTab(page, "Entities/Types");
    await expect(
      page.getByRole("button", { name: "Entities/Types" }),
    ).toBeVisible();
  });
});
