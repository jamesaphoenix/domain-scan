/**
 * E2E tests for tab management edge cases.
 *
 * Verifies:
 * - Closing the active tab makes the next tab active and loads its source
 * - Closing a tab to the left of active keeps the same file active
 * - Closing the only remaining tab shows empty state
 * - Middle-click on a tab closes it
 * - Opening more than 10 entities drops the oldest tab
 */

import { test, expect } from "@playwright/test";
import { setupTauriMocks, MOCK_SCAN_STATS } from "./mocks";
import { waitForAppReady, switchTab, clickOpenDirectory } from "./helpers";
import type { EntitySummary } from "../src/types";

// 12 entities across different files — enough to test the 10-tab limit
const TAB_MGMT_ENTITIES: EntitySummary[] = [
  { name: "Entity01", kind: "interface", file: "src/entity01.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity02", kind: "service", file: "src/entity02.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity03", kind: "class", file: "src/entity03.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity04", kind: "function", file: "src/entity04.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity05", kind: "interface", file: "src/entity05.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity06", kind: "service", file: "src/entity06.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity07", kind: "class", file: "src/entity07.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity08", kind: "function", file: "src/entity08.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity09", kind: "interface", file: "src/entity09.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity10", kind: "service", file: "src/entity10.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity11", kind: "class", file: "src/entity11.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
  { name: "Entity12", kind: "function", file: "src/entity12.ts", line: 1, language: "TypeScript", build_status: "built", confidence: "high" },
];

/** Helper: scan and open N entities as tabs */
async function scanAndOpenEntities(
  page: import("@playwright/test").Page,
  count: number,
) {
  await switchTab(page, "Entities/Types");
  await clickOpenDirectory(page);
  await expect(
    page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
  ).toBeVisible({ timeout: 10_000 });

  const treePanel = page.locator(".w-72").first();
  for (let i = 0; i < Math.min(count, TAB_MGMT_ENTITIES.length); i++) {
    await treePanel.getByText(TAB_MGMT_ENTITIES[i].name).click();
    // Wait for tab to appear
    await page.waitForTimeout(200);
  }
}

/** Helper: get the short filename from entity file path */
function shortFile(index: number): string {
  return TAB_MGMT_ENTITIES[index].file.replace("src/", "");
}

test.describe("Tab management edge cases", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: TAB_MGMT_ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("closing the active tab makes the next tab active", async ({ page }) => {
    await scanAndOpenEntities(page, 3);

    // Click on the first tab to make it active
    const firstTab = page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(0) }).first();
    await firstTab.click();
    await page.waitForTimeout(200);

    // Close the first tab via the X button
    const closeBtn = firstTab.locator("..").locator("button").first();
    await closeBtn.click();
    await page.waitForTimeout(300);

    // First tab should be gone
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(0) })).not.toBeVisible({ timeout: 3_000 });

    // Second tab should still exist
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(1) })).toBeVisible();

    // Monaco editor should still be visible (source loaded for the new active tab)
    await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 5_000 });
  });

  test("closing a tab to the left of the active tab keeps the active file", async ({ page }) => {
    await scanAndOpenEntities(page, 3);

    // Click on the third tab to make it active
    const thirdTab = page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(2) }).first();
    await thirdTab.click();
    await page.waitForTimeout(200);

    // Close the first tab (to the left of active)
    const firstTab = page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(0) }).first();
    // Right-click and close via context menu
    await firstTab.click({ button: "right" });
    await page.getByRole("button", { name: "Close", exact: true }).click();
    await page.waitForTimeout(300);

    // First tab should be gone
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(0) })).not.toBeVisible({ timeout: 3_000 });

    // Third tab should still be visible and active (same file stays active)
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(2) })).toBeVisible();

    // Monaco editor should be visible
    await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 5_000 });
  });

  test("closing the only remaining tab shows empty state", async ({ page }) => {
    await scanAndOpenEntities(page, 1);

    // Close the only tab via right-click context menu
    const onlyTab = page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(0) }).first();
    await onlyTab.click({ button: "right" });
    await page.getByRole("button", { name: "Close All" }).click();
    await page.waitForTimeout(300);

    // Empty state should be visible
    await expect(page.getByText("Select an entity to view source")).toBeVisible({
      timeout: 5_000,
    });
  });

  test("middle-click on a tab closes it", async ({ page }) => {
    await scanAndOpenEntities(page, 2);

    // Middle-click the first tab
    const firstTab = page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(0) }).first();
    await firstTab.click({ button: "middle" });

    // First tab should be gone
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(0) })).not.toBeVisible({
      timeout: 3_000,
    });

    // Second tab should remain
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(1) })).toBeVisible();
  });

  test("opening more than 10 entities drops the oldest tab", async ({ page }) => {
    // Open 12 entities — the tab bar should cap at some limit
    await scanAndOpenEntities(page, 12);

    // Count the number of tab labels visible
    const tabLabels = page.locator(".truncate.max-w-\\[160px\\]");
    const tabCount = await tabLabels.count();

    // The app should have dropped old tabs — count should be <= 10
    expect(tabCount).toBeLessThanOrEqual(10);

    // The most recently opened entities should still be visible
    // Entity12 (the last opened) should be present
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(11) })).toBeVisible();

    // Entity01 (the first opened) should have been evicted
    // (only check if the tab count is actually capped)
    if (tabCount <= 10) {
      await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: shortFile(0) })).not.toBeVisible();
    }
  });
});
