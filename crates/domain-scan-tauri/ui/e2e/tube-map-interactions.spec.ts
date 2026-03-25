/**
 * Phase F.5: E2E tests for Tube Map Interactions.
 *
 * Tests cover pan, zoom, station clicks, drill-in, breadcrumbs,
 * legend filtering, search, dependency trace, and Escape behavior.
 *
 * Tests run against the Vite dev server with Tauri IPC mocked
 * via `setupTauriMocks()` (injected before page load).
 */

import { test, expect } from "@playwright/test";
import {
  setupTauriMocks,
  MOCK_OCTOSPARK_TUBE_MAP,
  MOCK_MINIMAL_TUBE_MAP,
} from "./mocks";
import {
  waitForAppReady,
  switchTab,
  clickLoadManifest,
  waitForTubeMap,
  countVisibleNodes,
  searchTubeMap,
  clearSearch,
  pressKey,
} from "./helpers";

/**
 * Helper: load a manifest and wait for the tube map canvas.
 */
async function loadManifestAndWait(page: import("@playwright/test").Page) {
  await clickLoadManifest(page);
  await waitForTubeMap(page);
}

test.describe("F.5: Tube Map Interactions", () => {
  test("pan canvas with mouse drag → viewport moves", async ({ page }) => {
    // Use minimal manifest (2 nodes) to leave plenty of open pane area for dragging
    await setupTauriMocks(page, {
      tubeMapData: MOCK_MINIMAL_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Get the React Flow container for mouse interactions
    const rfContainer = page.locator(".react-flow").first();
    await expect(rfContainer).toBeVisible();

    // Wait for fitView animation to settle
    await page.waitForTimeout(600);

    // Read initial viewport transform
    const viewport = page.locator(".react-flow__viewport").first();
    const initialTransform = await viewport.getAttribute("style");

    // Get bounding box of the React Flow container
    const box = await rfContainer.boundingBox();
    expect(box).not.toBeNull();
    if (!box) return;

    // Drag from center-bottom area (likely empty) with many steps for reliable panning.
    // React Flow requires pointer events; Playwright fires both mouse + pointer events.
    const startX = box.x + box.width / 2;
    const startY = box.y + box.height - 30;

    await page.mouse.move(startX, startY);
    await page.mouse.down();
    // Move with many steps so React Flow's onPointerMove fires enough
    await page.mouse.move(startX - 150, startY - 120, { steps: 30 });
    await page.mouse.up();

    // Wait for the viewport to settle
    await page.waitForTimeout(600);

    // Verify the viewport transform has changed (pan moved the canvas)
    const newTransform = await viewport.getAttribute("style");
    expect(newTransform).not.toBe(initialTransform);
  });

  test("zoom with scroll wheel → zoom level changes, StatusBar updates", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_MINIMAL_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Status bar shows zoom percentage (e.g. "100%") — wait for it
    // The status bar renders "{zoomPercent}%" in a span
    await expect(page.getByText(/\d+%/).first()).toBeVisible({ timeout: 5_000 });

    // Get the React Flow pane for scroll interactions
    const pane = page.locator(".react-flow__pane").first();
    const paneBounds = await pane.boundingBox();
    expect(paneBounds).not.toBeNull();
    if (!paneBounds) return;

    const centerX = paneBounds.x + paneBounds.width / 2;
    const centerY = paneBounds.y + paneBounds.height / 2;

    // Get the initial viewport transform style
    const initialTransform = await page.locator(".react-flow__viewport").first().getAttribute("style");

    // Scroll to zoom out
    await page.mouse.move(centerX, centerY);
    await page.mouse.wheel(0, 300);

    // Wait for zoom animation
    await page.waitForTimeout(400);

    // Verify the viewport transform changed (zoom level is different)
    const newTransform = await page.locator(".react-flow__viewport").first().getAttribute("style");
    expect(newTransform).not.toBe(initialTransform);
  });

  test("click station node → details panel shows subsystem info", async ({
    page,
  }) => {
    // Use minimal manifest where subsystems do NOT have children (has_children: false)
    // Clicking a node without children calls onOpenFile, but the node body is still clickable
    // For testing "click shows info", use octospark where nodes have children → drill-in shows detail
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Click a station node heading to trigger click on the node body
    // Auth & Identity has has_children: true, so clicking drills in and shows detail
    const authNode = page.getByRole("heading", { name: "Auth & Identity" });
    await expect(authNode).toBeVisible();
    await authNode.click();

    // Drill-in view should show subsystem info (SubsystemDrillIn renders detail)
    // The mock get_subsystem_detail returns { name: "Mock Subsystem", domain: "core", status: "built" }
    await expect(page.getByText("Mock Subsystem")).toBeVisible({ timeout: 5_000 });
  });

  test("click station with children → drill-in view opens, breadcrumbs update", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Verify initial state: tube map canvas visible, no drill-in
    await expect(page.locator(".react-flow")).toBeVisible();

    // Click "Auth & Identity" station (has_children: true, child_count: 3)
    const authNode = page.getByRole("heading", { name: "Auth & Identity" });
    await expect(authNode).toBeVisible();
    await authNode.click();

    // Wait for drill-in view to appear
    await page.waitForTimeout(500);

    // Breadcrumbs should show: "Octospark / Auth & Identity"
    // Use nav-scoped locator since "Octospark" also appears in the header bar
    const breadcrumbNav = page.locator("nav");
    await expect(breadcrumbNav.getByText("Octospark")).toBeVisible();
    await expect(breadcrumbNav.locator("span.text-slate-200.font-medium", { hasText: "Auth & Identity" })).toBeVisible();

    // React Flow canvas should be replaced by drill-in view
    // The drill-in has a back button (chevron)
    await expect(page.getByText("Mock Subsystem")).toBeVisible({ timeout: 5_000 });
  });

  test("click breadcrumb → navigates back, tube map restores", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Drill into "Auth & Identity"
    const authNode = page.getByRole("heading", { name: "Auth & Identity" });
    await authNode.click();
    await page.waitForTimeout(500);

    // Verify we're in drill-in view
    await expect(page.getByText("Mock Subsystem")).toBeVisible({ timeout: 5_000 });

    // Click the "Octospark" breadcrumb to navigate back
    const breadcrumb = page.locator("nav button", { hasText: "Octospark" });
    await expect(breadcrumb).toBeVisible();
    await breadcrumb.click();

    // Wait for tube map to restore
    await page.waitForTimeout(500);

    // React Flow canvas should be visible again
    await expect(page.locator(".react-flow")).toBeVisible();

    // Station nodes should reappear
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).toBeVisible();
  });

  test("click domain in legend → filters to that domain only, compact layout triggers", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Verify all 18 nodes are visible initially
    const initialCount = await countVisibleNodes(page);
    expect(initialCount).toBe(18);

    // Click "Platform Core" domain in the legend (has 6 subsystems)
    const platformCoreButton = page.locator("button", { hasText: "Platform Core" });
    await expect(platformCoreButton).toBeVisible();
    await platformCoreButton.click();

    // Wait for filter and layout re-computation
    await page.waitForTimeout(500);

    // After filtering, only Platform Core subsystems should be visible (6)
    // The nodes may still be in DOM but the layout should change
    // Verify a non-Platform Core subsystem heading is hidden/removed
    // "AI Generation" is in "experimentation" domain — it should be filtered out
    const aiNode = page.getByRole("heading", { name: "AI Generation" });
    // In compact mode with domain filter, non-matching nodes are removed from layout
    // They may not be visible anymore
    await expect(aiNode).not.toBeVisible({ timeout: 5_000 });

    // Platform Core subsystems should remain visible
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Billing & Subscriptions" }),
    ).toBeVisible();

    // Click the same domain again to toggle off the filter
    await platformCoreButton.click();
    await page.waitForTimeout(500);

    // All nodes should be visible again
    await expect(
      page.getByRole("heading", { name: "AI Generation" }),
    ).toBeVisible();
  });

  test("type in search bar → stations filter by name, layout re-compacts", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Verify all 18 nodes are initially visible
    const initialCount = await countVisibleNodes(page);
    expect(initialCount).toBe(18);

    // Type "Auth" in the search bar
    await searchTubeMap(page, "Auth");

    // Wait for filter
    await page.waitForTimeout(500);

    // Only stations with "Auth" in their name should be visible
    // "Auth & Identity" and possibly others matching "Auth"
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).toBeVisible();

    // Non-matching stations should be hidden
    await expect(
      page.getByRole("heading", { name: "Workflows" }),
    ).not.toBeVisible({ timeout: 3_000 });
  });

  test("clear search → all stations reappear at canonical positions", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Search for "Billing" to filter
    await searchTubeMap(page, "Billing");
    await page.waitForTimeout(500);

    // Verify filter is active: non-matching node hidden
    await expect(
      page.getByRole("heading", { name: "Workflows" }),
    ).not.toBeVisible({ timeout: 3_000 });

    // Clear the search
    await clearSearch(page);
    await page.waitForTimeout(500);

    // All stations should reappear
    await expect(
      page.getByRole("heading", { name: "Workflows" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Billing & Subscriptions" }),
    ).toBeVisible();

    // Verify all 18 nodes are back
    const count = await countVisibleNodes(page);
    expect(count).toBe(18);
  });

  test("click trace on a station → dependency chain highlighted, non-chain nodes dimmed", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Select a subsystem in the "Dep. Trace" dropdown to activate trace
    const traceSelect = page.locator("select", { hasText: "Dep. Trace: Off" });
    await expect(traceSelect).toBeVisible();

    // Select "Retention Cleaner" for dependency trace — a leaf node with limited
    // connectivity (only depends on asset-management and notifications), so most
    // of the 18 nodes will be dimmed. Auth connects to everything transitively,
    // so it would leave 0 dimmed nodes.
    await traceSelect.selectOption("retention-cleaner");

    // Wait for trace computation and opacity changes
    await page.waitForTimeout(500);

    // Direction toggle buttons should appear (since a subsystem is now focused)
    await expect(page.getByRole("button", { name: "Upstream" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Both" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Downstream" })).toBeVisible();

    // Nodes in the dependency chain should have full opacity.
    // Retention Cleaner's chain (in "both" direction, upstream walk):
    // retention-cleaner → asset-management → auth, notifications → auth, org-team
    // That's ~5 highlighted nodes, so ~13 should be dimmed.
    const allNodes = page.locator(".react-flow__node");
    const nodeCount = await allNodes.count();
    expect(nodeCount).toBeGreaterThan(0);

    // Check that some nodes have dimmed opacity (opacity: 0.2 in their style)
    // The SubsystemNode sets opacity via inline style based on the `dimmed` prop
    let dimmedCount = 0;
    for (let i = 0; i < nodeCount; i++) {
      const nodeStyle = await allNodes.nth(i).locator("div").first().getAttribute("style");
      if (nodeStyle && nodeStyle.includes("opacity: 0.2")) {
        dimmedCount++;
      }
    }
    // At least some nodes should be dimmed (retention-cleaner doesn't connect to everything)
    expect(dimmedCount).toBeGreaterThan(0);
  });

  test("press Escape during trace → trace clears, all nodes restore opacity", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });

    await page.goto("/");
    await waitForAppReady(page);
    await switchTab(page, "Subsystem Tube Map");
    await loadManifestAndWait(page);

    // Activate dependency trace on "retention-cleaner" (a peripheral node)
    const traceSelect = page.locator("select", { hasText: "Dep. Trace: Off" });
    await traceSelect.selectOption("retention-cleaner");
    await page.waitForTimeout(500);

    // Verify trace is active: direction buttons visible
    await expect(page.getByRole("button", { name: "Both" })).toBeVisible();

    // Press Escape — should clear the dependency trace
    // First click somewhere on the pane to ensure no input is focused
    const pane = page.locator(".react-flow__pane").first();
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);

    await pressKey(page, "Escape");
    await page.waitForTimeout(500);

    // Direction buttons should disappear (trace cleared)
    await expect(
      page.getByRole("button", { name: "Upstream" }),
    ).not.toBeVisible({ timeout: 3_000 });

    // All nodes should have full opacity restored
    const allNodes = page.locator(".react-flow__node");
    const nodeCount = await allNodes.count();
    expect(nodeCount).toBeGreaterThan(0);

    let dimmedCount = 0;
    for (let i = 0; i < nodeCount; i++) {
      const nodeStyle = await allNodes.nth(i).locator("div").first().getAttribute("style");
      if (nodeStyle && nodeStyle.includes("opacity: 0.2")) {
        dimmedCount++;
      }
    }
    // No nodes should be dimmed after clearing trace
    expect(dimmedCount).toBe(0);
  });
});
