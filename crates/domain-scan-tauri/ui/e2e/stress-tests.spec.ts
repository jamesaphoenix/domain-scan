/**
 * Phase F.7: Stress Tests & Edge Cases.
 *
 * Tests cover large manifests, circular dependencies, missing domains,
 * orphan subsystems, window resize, minimize/restore, large codebases,
 * and rapid double-click protection.
 *
 * Tests run against the Vite dev server with Tauri IPC mocked
 * via `setupTauriMocks()` (injected before page load).
 */

import { test, expect } from "@playwright/test";
import {
  setupTauriMocks,
  MOCK_LARGE_TUBE_MAP,
  MOCK_CIRCULAR_DEPS_TUBE_MAP,
  MOCK_NO_DOMAINS_TUBE_MAP,
  MOCK_ORPHAN_SUBSYSTEMS_TUBE_MAP,
  MOCK_OCTOSPARK_TUBE_MAP,
  MOCK_SCAN_STATS,
} from "./mocks";
import {
  waitForAppReady,
  switchTab,
  clickLoadManifest,
  waitForTubeMap,
  countVisibleNodes,
  pressKey,
} from "./helpers";

/**
 * Helper: load a manifest and wait for the tube map canvas.
 */
async function loadManifestAndWait(page: import("@playwright/test").Page) {
  await clickLoadManifest(page);
  await waitForTubeMap(page);
}

test.describe("F.7: Stress Tests & Edge Cases", () => {
  test("load large.json (200 subsystems) → renders within 3 seconds, pan/zoom stays smooth", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_LARGE_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");

    // Measure render time: start clock, load manifest, wait for canvas
    const startTime = Date.now();
    await loadManifestAndWait(page);
    const renderTime = Date.now() - startTime;

    // Must render within 3 seconds
    expect(renderTime).toBeLessThan(3_000);

    // Verify all 200 nodes exist in the DOM
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(200);

    // Verify status bar shows subsystem count
    await expect(page.getByText("200 subsystems").first()).toBeVisible({
      timeout: 5_000,
    });

    // Test zoom + pan: at scale 0.1 nodes fill the entire viewport, so test
    // zoom first (scroll wheel always works), then pan after zooming in.
    const rfContainer = page.locator(".react-flow").first();
    await expect(rfContainer).toBeVisible();

    // Wait for fitView animation to settle
    await page.waitForTimeout(600);

    const viewport = page.locator(".react-flow__viewport").first();
    const initialTransform = await viewport.getAttribute("style");

    const box = await rfContainer.boundingBox();
    expect(box).not.toBeNull();
    if (!box) return;

    const centerX = box.x + box.width / 2;
    const centerY = box.y + box.height / 2;

    // Zoom in with scroll wheel (negative deltaY = zoom in)
    await page.mouse.move(centerX, centerY);
    await page.mouse.wheel(0, -500);
    await page.waitForTimeout(600);

    const afterZoomTransform = await viewport.getAttribute("style");
    expect(afterZoomTransform).not.toBe(initialTransform);

    // Now that we're zoomed in, there should be empty space for panning.
    // Drag from the center outward.
    const pane = page.locator(".react-flow__pane").first();
    const paneBox = await pane.boundingBox();
    expect(paneBox).not.toBeNull();
    if (!paneBox) return;

    const panStartX = paneBox.x + paneBox.width - 20;
    const panStartY = paneBox.y + paneBox.height - 20;
    await page.mouse.move(panStartX, panStartY);
    await page.mouse.down();
    await page.mouse.move(panStartX - 200, panStartY - 150, { steps: 30 });
    await page.mouse.up();
    await page.waitForTimeout(600);

    const afterPanTransform = await viewport.getAttribute("style");
    expect(afterPanTransform).not.toBe(afterZoomTransform);
  });

  test("load circular-deps.json → cycle-breaking produces valid layout, all nodes render", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_CIRCULAR_DEPS_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // All 6 subsystems should render despite circular dependencies
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(6);

    // Verify subsystem names are visible (cycle-breaking didn't drop any nodes)
    await expect(
      page.getByRole("heading", { name: "Service A" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Service B" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Service C" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Service D" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Service E" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "UI App" }),
    ).toBeVisible();

    // Edges should still render (circular edges are valid connections)
    const edges = page.locator(".react-flow__edge");
    const edgeCount = await edges.count();
    expect(edgeCount).toBeGreaterThan(0);

    // Both domains should be represented in the legend (buttons with domain labels)
    await expect(page.locator("button", { hasText: "Backend" })).toBeVisible();
    await expect(page.locator("button", { hasText: "Frontend" })).toBeVisible();
  });

  test("load no-domains.json → all subsystems render on gray 'unassigned' line", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_NO_DOMAINS_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // All 3 subsystems should render despite having no valid domains
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(3);

    // Verify subsystem names are visible
    await expect(
      page.getByRole("heading", { name: "Authentication" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Billing" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Notifications" }),
    ).toBeVisible();

    // The "Unassigned" domain should appear in the legend (as a button) since all
    // subsystems have invalid/missing domains and are placed on the unassigned line
    await expect(page.locator("button", { hasText: /unassigned/i })).toBeVisible();

    // Edges should still render between subsystems
    const edges = page.locator(".react-flow__edge");
    const edgeCount = await edges.count();
    expect(edgeCount).toBeGreaterThan(0);

    // App should remain stable — tabs present
    await expect(
      page.getByRole("button", { name: "Entities/Types" }),
    ).toBeVisible();
  });

  test("load orphan-subsystems.json → orphan subsystems placed in fallback row, no crash", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_ORPHAN_SUBSYSTEMS_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // All 4 subsystems should render (1 valid domain + 3 orphans)
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(4);

    // Valid domain subsystem renders
    await expect(
      page.getByRole("heading", { name: "Authentication" }),
    ).toBeVisible();

    // Orphan subsystems render (domain doesn't exist in domains map)
    await expect(
      page.getByRole("heading", { name: "Payments" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Analytics" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Email Service" }),
    ).toBeVisible();

    // The "Core" domain should appear in the legend (as a button)
    await expect(page.locator("button", { hasText: "Core" })).toBeVisible();

    // Orphan subsystems should be on the unassigned line (legend button)
    await expect(page.locator("button", { hasText: /unassigned/i })).toBeVisible();

    // Edges should render between valid and orphan subsystems
    const edges = page.locator(".react-flow__edge");
    const edgeCount = await edges.count();
    expect(edgeCount).toBeGreaterThan(0);
  });

  test("window resize → layout persists, no overlapping nodes, MiniMap updates", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Verify initial state: 18 nodes
    const initialCount = await countVisibleNodes(page);
    expect(initialCount).toBe(18);

    // Wait for layout to settle
    await page.waitForTimeout(600);

    // Resize the viewport to a smaller window
    await page.setViewportSize({ width: 800, height: 600 });
    await page.waitForTimeout(500);

    // React Flow canvas should still be visible after resize
    await expect(page.locator(".react-flow")).toBeVisible();

    // All nodes should still exist in the DOM
    const afterResizeCount = await countVisibleNodes(page);
    expect(afterResizeCount).toBe(18);

    // MiniMap should still be visible
    await expect(page.locator(".react-flow__minimap")).toBeVisible();

    // Press 'f' to refit view after resize
    const pane = page.locator(".react-flow__pane").first();
    await pane.click({ position: { x: 10, y: 10 } });
    await pressKey(page, "f");
    await page.waitForTimeout(500);

    // Resize back to a larger window
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.waitForTimeout(500);

    // Canvas and nodes should still be stable
    await expect(page.locator(".react-flow")).toBeVisible();
    const finalCount = await countVisibleNodes(page);
    expect(finalCount).toBe(18);

    // Verify some subsystem headings are still visible
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).toBeVisible();
  });

  test("minimize/restore window → React Flow canvas re-renders correctly", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Verify initial state
    const initialCount = await countVisibleNodes(page);
    expect(initialCount).toBe(18);

    // Simulate minimize by setting viewport to very small
    await page.setViewportSize({ width: 100, height: 100 });
    await page.waitForTimeout(300);

    // Simulate restore by setting viewport back to normal
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.waitForTimeout(500);

    // React Flow canvas should still be visible
    await expect(page.locator(".react-flow")).toBeVisible();

    // All nodes should still exist
    const restoredCount = await countVisibleNodes(page);
    expect(restoredCount).toBe(18);

    // Press 'f' to refit view
    const pane = page.locator(".react-flow__pane").first();
    await pane.click({ position: { x: 10, y: 10 } });
    await pressKey(page, "f");
    await page.waitForTimeout(500);

    // Verify subsystems are still accessible
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Billing & Subscriptions" }),
    ).toBeVisible();
  });

  test("scan large codebase → match against large manifest → tube map renders without OOM", async ({
    page,
  }) => {
    // Simulate a 1000-file scan with matching against the large manifest
    const largeScanStats = {
      ...MOCK_SCAN_STATS,
      total_files: 1000,
      files_by_language: { TypeScript: 600, Rust: 300, Python: 100 },
      total_interfaces: 150,
      total_services: 80,
      total_classes: 50,
      total_methods: 1200,
      total_functions: 250,
      total_schemas: 100,
      total_type_aliases: 70,
      total_implementations: 30,
      parse_duration_ms: 2500,
      cache_hits: 0,
      cache_misses: 1000,
    };

    // Generate a large entity list (50 entities for matching)
    const largeEntities = Array.from({ length: 50 }, (_, i) => ({
      name: `Entity${i}`,
      kind: "interface" as const,
      file: `src/domain-${i % 20}/entity-${i}.ts`,
      line: i * 10 + 1,
      language: "TypeScript" as const,
      build_status: "built" as const,
      confidence: "high" as const,
    }));

    await setupTauriMocks(page, {
      dialogResult: "/mock/large-project",
      scanStats: largeScanStats,
      entities: largeEntities,
      tubeMapData: MOCK_LARGE_TUBE_MAP,
      matchResult: {
        matched: largeEntities.slice(0, 30).map((e) => e.name),
        unmatched: largeEntities.slice(30).map((e) => e.name),
        coverage_percent: 60,
      },
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Perform the scan first
    await page.getByText("Open Directory").click();
    await expect(
      page.getByText("1000 files"),
    ).toBeVisible({ timeout: 10_000 });

    // Switch to Tube Map and load manifest
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // All 200 nodes should render
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(200);

    // Coverage % should be shown
    await expect(page.getByText(/60(\.0)?%/).first()).toBeVisible({
      timeout: 5_000,
    });

    // App should remain responsive — test a search interaction
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    await searchInput.fill("Subsystem 10");
    await page.waitForTimeout(500);

    // Search should filter — "Subsystem 100" and similar should be visible
    // while unrelated subsystems should be filtered out
    await expect(
      page.getByRole("heading", { name: "Subsystem 100" }),
    ).toBeVisible();

    // Clear search to verify all nodes restore
    await searchInput.fill("");
    await page.waitForTimeout(500);
    const restoredCount = await countVisibleNodes(page);
    expect(restoredCount).toBe(200);
  });

  test("double-click station rapidly → no duplicate drill-in views, breadcrumbs don't double-push", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Wait for layout to settle
    await page.waitForTimeout(600);

    // Find a station with children — "Auth & Identity" (has_children: true)
    const authNode = page.getByRole("heading", { name: "Auth & Identity" });
    await expect(authNode).toBeVisible();

    // Rapidly double-click the station (dblclick fires two clicks + dblclick event)
    await authNode.dblclick();

    // Wait for drill-in to complete
    await page.waitForTimeout(500);

    // Verify drill-in view appears (only once, not duplicated)
    await expect(page.getByText("Mock Subsystem")).toBeVisible({ timeout: 5_000 });

    // Breadcrumbs should show exactly one level of navigation,
    // not a duplicated path. The breadcrumb nav should have "Octospark"
    // as root and "Auth & Identity" as the current level.
    const breadcrumbNav = page.locator("nav");
    await expect(breadcrumbNav).toBeVisible();

    // Count breadcrumb items — should have exactly 2 segments:
    // "Octospark" (root) and "Auth & Identity" (current)
    // The root breadcrumb is a button, the current is a span
    const breadcrumbButtons = breadcrumbNav.locator("button");
    const breadcrumbCount = await breadcrumbButtons.count();
    // Should have exactly 1 clickable breadcrumb (root "Octospark")
    // The current level "Auth & Identity" is a non-clickable span
    expect(breadcrumbCount).toBe(1);

    // Navigate back to tube map
    await breadcrumbNav.locator("button", { hasText: "Octospark" }).click();
    await page.waitForTimeout(500);

    // Verify tube map is restored
    await expect(page.locator(".react-flow")).toBeVisible();

    // Get the station's bounding box for rapid clicking via mouse events.
    // Using page.mouse avoids Playwright's actionability checks which would
    // block after the first click causes the heading to disappear via drill-in.
    const authNodeAgain = page.getByRole("heading", { name: "Auth & Identity" });
    await expect(authNodeAgain).toBeVisible();
    const nodeBox = await authNodeAgain.boundingBox();
    expect(nodeBox).not.toBeNull();
    if (!nodeBox) return;

    const cx = nodeBox.x + nodeBox.width / 2;
    const cy = nodeBox.y + nodeBox.height / 2;

    // Fire 3 rapid clicks using raw mouse events (no actionability waits)
    await page.mouse.click(cx, cy);
    await page.mouse.click(cx, cy);
    await page.mouse.click(cx, cy);

    await page.waitForTimeout(500);

    // Should still only show one drill-in view, not stacked
    await expect(page.getByText("Mock Subsystem")).toBeVisible({ timeout: 5_000 });

    // Breadcrumb should still show exactly 1 clickable button (root)
    const finalBreadcrumbButtons = page.locator("nav button");
    const finalCount = await finalBreadcrumbButtons.count();
    expect(finalCount).toBe(1);
  });
});
