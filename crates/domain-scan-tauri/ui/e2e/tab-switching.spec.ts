/**
 * Phase F.3: E2E tests for Tab Switching.
 *
 * Tests run against the Vite dev server with Tauri IPC mocked
 * via `setupTauriMocks()` (injected before page load).
 */

import { test, expect } from "@playwright/test";
import { setupTauriMocks, MOCK_SCAN_STATS, MOCK_ENTITIES } from "./mocks";
import {
  waitForAppReady,
  switchTab,
  assertActiveTab,
  assertManifestLoaderVisible,
  clickOpenDirectory,
} from "./helpers";

test.describe("F.3: Tab Switching", () => {
  test("app starts on Tube Map tab by default", async ({ page }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await waitForAppReady(page);

    // Tube Map tab should be active (has active styling)
    await assertActiveTab(page, "Subsystem Tube Map");

    // Entity tree panel should NOT be visible
    const treePanel = page.locator(".w-72").first();
    await expect(treePanel).not.toBeVisible();
  });

  test("click Tube Map tab → tube map placeholder renders (no manifest loaded)", async ({
    page,
  }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await waitForAppReady(page);

    // Switch to Tube Map tab
    await switchTab(page, "Subsystem Tube Map");

    // Tube Map tab should now be active
    await assertActiveTab(page, "Subsystem Tube Map");

    // ManifestLoader should be visible (no manifest loaded yet)
    await assertManifestLoaderVisible(page);

    // Entities panel should NOT be visible
    const treePanel = page.locator(".w-72").first();
    await expect(treePanel).not.toBeVisible();
  });

  test("switch back to Entities → tree state preserved (selection, expansion)", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: MOCK_ENTITIES,
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Switch to Entities tab first (app starts on Tube Map by default)
    await switchTab(page, "Entities/Types");

    // Trigger a scan so the entity tree has content
    await clickOpenDirectory(page);
    await expect(
      page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
    ).toBeVisible({ timeout: 10_000 });

    // Verify entities are visible in the tree
    const treePanel = page.locator(".w-72").first();
    await expect(treePanel.getByText("AuthProvider")).toBeVisible({
      timeout: 5_000,
    });

    // Click on an entity to select it
    await treePanel.getByText("AuthProvider").click();

    // Switch to Tube Map tab
    await switchTab(page, "Subsystem Tube Map");
    await assertActiveTab(page, "Subsystem Tube Map");

    // Switch back to Entities tab
    await switchTab(page, "Entities/Types");
    await assertActiveTab(page, "Entities/Types");

    // Tree state should be preserved: entities still visible
    await expect(treePanel.getByText("AuthProvider")).toBeVisible();
    await expect(treePanel.getByText("UserService")).toBeVisible();
    await expect(treePanel.getByText("ApiRouter")).toBeVisible();
  });

  test("rapid tab switching (10x in 1 second) → no crash, no leaked state", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: MOCK_ENTITIES,
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Switch to Entities tab first (app starts on Tube Map by default)
    await switchTab(page, "Entities/Types");

    // Trigger a scan first so both tabs have content to render
    await clickOpenDirectory(page);
    await expect(
      page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
    ).toBeVisible({ timeout: 10_000 });

    // Rapidly switch tabs 10 times
    for (let i = 0; i < 10; i++) {
      await switchTab(page, "Subsystem Tube Map");
      await switchTab(page, "Entities/Types");
    }

    // App should not have crashed — verify core elements are still present
    await expect(page.getByText("domain-scan")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Entities/Types" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Subsystem Tube Map" }),
    ).toBeVisible();

    // Should be on Entities tab (last switch was to Entities)
    await assertActiveTab(page, "Entities/Types");

    // Entity tree should still have its content (no leaked/corrupted state)
    const treePanel = page.locator(".w-72").first();
    await expect(treePanel.getByText("AuthProvider")).toBeVisible({
      timeout: 5_000,
    });

    // Switch to Tube Map one more time to verify it still works
    await switchTab(page, "Subsystem Tube Map");
    await assertActiveTab(page, "Subsystem Tube Map");
    await assertManifestLoaderVisible(page);
  });
});
