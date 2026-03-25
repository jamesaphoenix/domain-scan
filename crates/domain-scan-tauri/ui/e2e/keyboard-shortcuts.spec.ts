/**
 * Phase F.6: E2E tests for Keyboard Shortcuts (Tube Map Tab).
 *
 * Tests cover fitView, search focus, domain filter toggles,
 * clear all filters, shortcut help overlay, Escape cascading,
 * and input-focus suppression.
 *
 * Tests run against the Vite dev server with Tauri IPC mocked
 * via `setupTauriMocks()` (injected before page load).
 */

import { test, expect } from "@playwright/test";
import {
  setupTauriMocks,
  MOCK_OCTOSPARK_TUBE_MAP,
} from "./mocks";
import {
  waitForAppReady,
  switchTab,
  clickLoadManifest,
  waitForTubeMap,
  countVisibleNodes,
  searchTubeMap,
  pressKey,
  isShortcutHelpVisible,
} from "./helpers";

/**
 * Helper: load a manifest and wait for the tube map canvas.
 */
async function loadManifestAndWait(page: import("@playwright/test").Page) {
  await clickLoadManifest(page);
  await waitForTubeMap(page);
}

/**
 * Helper: set up octospark mocks, navigate to tube map, and load manifest.
 */
async function setupTubeMapWithManifest(page: import("@playwright/test").Page) {
  await setupTauriMocks(page, {
    tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
  });
  await page.goto("/");
  await waitForAppReady(page);
  await switchTab(page, "Subsystem Tube Map");
  await loadManifestAndWait(page);
  // Wait for fitView animation to settle
  await page.waitForTimeout(600);
}

test.describe("F.6: Keyboard Shortcuts (Tube Map Tab)", () => {
  test("press `f` → fitView fires, all nodes visible", async ({ page }) => {
    await setupTubeMapWithManifest(page);

    // First, zoom in so not all nodes are visible by scrolling
    const pane = page.locator(".react-flow__pane").first();
    const paneBounds = await pane.boundingBox();
    expect(paneBounds).not.toBeNull();
    if (!paneBounds) return;

    const centerX = paneBounds.x + paneBounds.width / 2;
    const centerY = paneBounds.y + paneBounds.height / 2;

    // Scroll to zoom in heavily
    await page.mouse.move(centerX, centerY);
    await page.mouse.wheel(0, -500);
    await page.waitForTimeout(400);

    // Record viewport transform after zoom-in
    const zoomedTransform = await page
      .locator(".react-flow__viewport")
      .first()
      .getAttribute("style");

    // Press 'f' to fit view — click on pane first to ensure no input is focused
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);
    await pressKey(page, "f");

    // Wait for fitView animation (duration: 300ms + buffer)
    await page.waitForTimeout(500);

    // Viewport transform should change (fitView recalculates zoom/pan)
    const fitViewTransform = await page
      .locator(".react-flow__viewport")
      .first()
      .getAttribute("style");
    expect(fitViewTransform).not.toBe(zoomedTransform);

    // All 18 nodes should still be in the DOM
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(18);
  });

  test("press `/` → search input focused", async ({ page }) => {
    await setupTubeMapWithManifest(page);

    // Click on the pane to ensure focus is on the canvas, not an input
    const pane = page.locator(".react-flow__pane").first();
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);

    // Verify search input is NOT focused initially
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    await expect(searchInput).not.toBeFocused();

    // Press '/' to focus the search input
    await pressKey(page, "/");
    await page.waitForTimeout(200);

    // Search input should now be focused
    await expect(searchInput).toBeFocused();
  });

  test("press `1`-`7` → corresponding domain filter toggles", async ({
    page,
  }) => {
    await setupTubeMapWithManifest(page);

    // The octospark manifest has 7 domains in this order:
    // 1: platform-core, 2: media-storage, 3: services,
    // 4: experimentation, 5: workflows, 6: frontends, 7: post-launch

    // Click pane to ensure no input focused
    const pane = page.locator(".react-flow__pane").first();
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);

    // Verify all 18 nodes initially visible
    let nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(18);

    // Press '1' to filter to domain 1 (platform-core: 6 subsystems)
    await pressKey(page, "1");
    await page.waitForTimeout(500);

    // Non-platform-core subsystems should be hidden
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "AI Generation" }),
    ).not.toBeVisible({ timeout: 3_000 });

    // Press '1' again to toggle off the filter
    await pressKey(page, "1");
    await page.waitForTimeout(500);

    // All nodes should reappear
    nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(18);
    await expect(
      page.getByRole("heading", { name: "AI Generation" }),
    ).toBeVisible();

    // Press '4' to filter to domain 4 (experimentation: 2 subsystems)
    await pressKey(page, "4");
    await page.waitForTimeout(500);

    // Experimentation domain subsystems should be visible
    await expect(
      page.getByRole("heading", { name: "AI Generation" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Contextual Prompt Bandit" }),
    ).toBeVisible();

    // Non-experimentation subsystems should be hidden
    await expect(
      page.getByRole("heading", { name: "Auth & Identity" }),
    ).not.toBeVisible({ timeout: 3_000 });

    // Press '4' again to toggle off
    await pressKey(page, "4");
    await page.waitForTimeout(500);

    nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(18);
  });

  test("press `0` → all filters cleared", async ({ page }) => {
    await setupTubeMapWithManifest(page);

    // Click pane to ensure no input focused
    const pane = page.locator(".react-flow__pane").first();
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);

    // Apply a domain filter (press '1' for platform-core)
    await pressKey(page, "1");
    await page.waitForTimeout(500);

    // Verify filter is active: non-matching nodes hidden
    await expect(
      page.getByRole("heading", { name: "AI Generation" }),
    ).not.toBeVisible({ timeout: 3_000 });

    // Also set a search query
    await searchTubeMap(page, "Auth");
    await page.waitForTimeout(300);

    // Also activate dependency trace
    const traceSelect = page.locator("select", {
      hasText: "Dep. Trace: Off",
    });
    // The trace dropdown might not be visible after filtering, so skip if not present
    if (await traceSelect.isVisible()) {
      await traceSelect.selectOption("auth");
      await page.waitForTimeout(300);
    }

    // Click pane to unfocus search input before pressing '0'
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);

    // Press '0' to clear all filters
    await pressKey(page, "0");
    await page.waitForTimeout(500);

    // All 18 nodes should be visible again
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(18);

    // Search input should be cleared
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    await expect(searchInput).toHaveValue("");

    // Dependency trace direction buttons should be gone
    await expect(
      page.getByRole("button", { name: "Upstream" }),
    ).not.toBeVisible({ timeout: 3_000 });
  });

  test("press `?` → shortcut help overlay appears", async ({ page }) => {
    await setupTubeMapWithManifest(page);

    // Click pane to ensure no input focused
    const pane = page.locator(".react-flow__pane").first();
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);

    // Verify shortcut help is NOT visible initially
    expect(await isShortcutHelpVisible(page)).toBe(false);

    // Press '?' to show shortcut help
    await pressKey(page, "?");
    await page.waitForTimeout(300);

    // Shortcut help overlay should be visible
    expect(await isShortcutHelpVisible(page)).toBe(true);
    await expect(page.getByText("Keyboard Shortcuts")).toBeVisible();

    // Verify some shortcut entries are shown
    await expect(page.getByText("Fit view")).toBeVisible();
    await expect(page.getByText("Focus search input")).toBeVisible();
    await expect(page.getByText("Clear all filters")).toBeVisible();

    // Press '?' again to toggle it off
    await pressKey(page, "?");
    await page.waitForTimeout(300);

    expect(await isShortcutHelpVisible(page)).toBe(false);
  });

  test("press `Escape` → overlay/search/trace/filter cleared in priority order", async ({
    page,
  }) => {
    await setupTubeMapWithManifest(page);

    // Click pane to ensure no input focused
    const pane = page.locator(".react-flow__pane").first();
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);

    // --- Step 1: Open shortcut help, then Escape closes it ---
    await pressKey(page, "?");
    await page.waitForTimeout(300);
    expect(await isShortcutHelpVisible(page)).toBe(true);

    await pressKey(page, "Escape");
    await page.waitForTimeout(300);
    expect(await isShortcutHelpVisible(page)).toBe(false);

    // --- Step 2: Set search query, then Escape clears it ---
    await searchTubeMap(page, "Auth");
    await page.waitForTimeout(300);

    // Blur the search input first (Escape on focused input just blurs it)
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    await pane.click({ position: { x: 10, y: 10 } });
    await page.waitForTimeout(100);

    // Verify search is still set
    await expect(searchInput).toHaveValue("Auth");

    // Press Escape — should clear search query
    await pressKey(page, "Escape");
    await page.waitForTimeout(300);
    await expect(searchInput).toHaveValue("");

    // --- Step 3: Activate dependency trace, then Escape clears it ---
    const traceSelect = page.locator("select", {
      hasText: "Dep. Trace: Off",
    });
    await expect(traceSelect).toBeVisible();
    await traceSelect.selectOption("retention-cleaner");
    await page.waitForTimeout(500);

    // Verify trace is active (direction buttons visible)
    await expect(
      page.getByRole("button", { name: "Both" }),
    ).toBeVisible();

    // Press Escape — should clear the trace
    await pressKey(page, "Escape");
    await page.waitForTimeout(500);

    await expect(
      page.getByRole("button", { name: "Upstream" }),
    ).not.toBeVisible({ timeout: 3_000 });

    // --- Step 4: Set domain filter, then Escape clears it ---
    await pressKey(page, "1");
    await page.waitForTimeout(500);

    // Verify filter active
    await expect(
      page.getByRole("heading", { name: "AI Generation" }),
    ).not.toBeVisible({ timeout: 3_000 });

    // Press Escape — should clear domain filter
    await pressKey(page, "Escape");
    await page.waitForTimeout(500);

    // All nodes should be visible again
    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(18);
  });

  test("keyboard shortcuts do NOT fire when typing in search input", async ({
    page,
  }) => {
    await setupTubeMapWithManifest(page);

    // Focus the search input
    const searchInput = page.locator('input[placeholder*="Search"]').first();
    await searchInput.click();
    await page.waitForTimeout(100);

    // Type 'f' — should go into search input, NOT trigger fitView
    await page.keyboard.type("f");
    await page.waitForTimeout(200);

    // Search input should contain 'f'
    await expect(searchInput).toHaveValue("f");

    // Type '?' — should go into search input, NOT toggle shortcut help
    await page.keyboard.type("?");
    await page.waitForTimeout(200);
    await expect(searchInput).toHaveValue("f?");
    expect(await isShortcutHelpVisible(page)).toBe(false);

    // Type '0' — should go into search input, NOT clear filters
    await page.keyboard.type("0");
    await page.waitForTimeout(200);
    await expect(searchInput).toHaveValue("f?0");

    // Type '1' — should go into search input, NOT toggle domain filter
    await page.keyboard.type("1");
    await page.waitForTimeout(200);
    await expect(searchInput).toHaveValue("f?01");

    // All 18 nodes should still be visible (no domain filter was applied)
    // Note: search filtering may hide some nodes, but that's search behavior, not shortcut behavior.
    // Clear the search to verify no shortcut-based filter was applied
    await searchInput.fill("");
    await page.waitForTimeout(300);

    const nodeCount = await countVisibleNodes(page);
    expect(nodeCount).toBe(18);

    // Shortcut help should NOT have been opened
    expect(await isShortcutHelpVisible(page)).toBe(false);
  });
});
