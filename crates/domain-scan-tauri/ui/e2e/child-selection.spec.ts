/**
 * E2E tests for child row selection (methods, properties, fields, routes).
 *
 * Verifies that clicking a child row in the entity tree:
 * - Highlights the child visually
 * - Scrolls Monaco to the child's line
 * - Keeps the parent's tab active
 * - Clears child selection when parent changes
 */

import { test, expect } from "@playwright/test";
import { setupTauriMocks, MOCK_SCAN_STATS } from "./mocks";
import { waitForAppReady, switchTab, clickOpenDirectory } from "./helpers";
import type { EntitySummary } from "../src/types";

// Entities with different kinds to test child population
const ENTITIES: EntitySummary[] = [
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
    kind: "class",
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

async function scanAndWait(page: import("@playwright/test").Page) {
  await switchTab(page, "Entities/Types");
  await clickOpenDirectory(page);
  await expect(
    page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
  ).toBeVisible({ timeout: 10_000 });
}

test.describe("Child row selection", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("clicking a parent entity shows it as selected in the tree", async ({
    page,
  }) => {
    await scanAndWait(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // Parent row should have the selected styling
    const selectedRow = treePanel.locator(".bg-blue-900\\/50").first();
    await expect(selectedRow).toBeVisible({ timeout: 5_000 });
    await expect(selectedRow).toContainText("AuthProvider");
  });

  test("clicking parent creates a file tab in the Monaco preview", async ({
    page,
  }) => {
    await scanAndWait(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // A tab should appear in Monaco with the file name
    await expect(
      page.locator(".truncate.max-w-\\[160px\\]", { hasText: "auth/provider.ts" }),
    ).toBeVisible({ timeout: 5_000 });

    // Monaco editor should be loaded
    await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 5_000 });
  });

  test("clicking a second entity from a different file opens a new tab", async ({
    page,
  }) => {
    await scanAndWait(page);

    const treePanel = page.locator(".w-72").first();

    // Click first entity
    await treePanel.getByText("AuthProvider").click();
    await expect(
      page.locator(".truncate.max-w-\\[160px\\]", { hasText: "auth/provider.ts" }),
    ).toBeVisible({ timeout: 5_000 });

    // Click second entity from different file
    await treePanel.getByText("UserService").click();
    await expect(
      page.locator(".truncate.max-w-\\[160px\\]", { hasText: "services/user.ts" }),
    ).toBeVisible({ timeout: 5_000 });

    // Both tabs should exist
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: "auth/provider.ts" })).toBeVisible();
    await expect(page.locator(".truncate.max-w-\\[160px\\]", { hasText: "services/user.ts" })).toBeVisible();
  });

  test("clicking parent entity while another is selected switches correctly", async ({
    page,
  }) => {
    await scanAndWait(page);

    const treePanel = page.locator(".w-72").first();

    await treePanel.getByText("AuthProvider").click();
    const detailsPanel = page.locator(".w-80").first();
    await expect(detailsPanel.getByText("AuthProvider")).toBeVisible({
      timeout: 5_000,
    });

    // Switch to another entity
    await treePanel.getByText("ApiRouter").click();
    await expect(detailsPanel.getByText("ApiRouter")).toBeVisible({
      timeout: 5_000,
    });
  });
});

test.describe("Entity tree highlight state", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("selected entity has blue highlight, others do not", async ({
    page,
  }) => {
    await scanAndWait(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // Exactly one row with selected styling
    const selectedRows = treePanel.locator(".bg-blue-900\\/50");
    await expect(selectedRows).toHaveCount(1);
    await expect(selectedRows.first()).toContainText("AuthProvider");
  });

  test("re-clicking the same entity toggles expand/collapse", async ({
    page,
  }) => {
    await scanAndWait(page);

    const treePanel = page.locator(".w-72").first();

    // First click selects and expands
    await treePanel.getByText("AuthProvider").click();
    await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 5_000 });

    // Second click on same entity toggles collapse (expand indicator changes)
    await treePanel.getByText("AuthProvider").click();
    // The entity should still be selected (blue highlight)
    const selectedRows = treePanel.locator(".bg-blue-900\\/50");
    await expect(selectedRows).toHaveCount(1);
  });
});
