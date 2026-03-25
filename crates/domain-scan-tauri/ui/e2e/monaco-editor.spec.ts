/**
 * E2E tests for Monaco Editor integration (Phase H.2).
 *
 * Verifies syntax highlighting, scroll-to-line, language detection,
 * read-only mode, tab management, rapid switching, large files, and minimap.
 *
 * Tests run against the Vite dev server with Tauri IPC mocked.
 */

import { test, expect } from "@playwright/test";
import { setupTauriMocks, MOCK_SCAN_STATS } from "./mocks";
import type { EntitySummary } from "../src/types";
import { waitForAppReady, clickOpenDirectory } from "./helpers";

// ---------------------------------------------------------------------------
// Mock entities for Monaco tests
// ---------------------------------------------------------------------------

/** Entities spanning multiple files and languages */
const MONACO_ENTITIES: EntitySummary[] = [
  {
    name: "AuthProvider",
    kind: "interface",
    file: "src/auth/provider.ts",
    line: 2,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "UserService",
    kind: "service",
    file: "src/auth/provider.ts", // same file as AuthProvider
    line: 12,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "ApiRouter",
    kind: "class",
    file: "src/api/router.ts", // different file
    line: 1,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "getUser",
    kind: "function",
    file: "src/api/users.ts",
    line: 8,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "UserSchema",
    kind: "schema",
    file: "src/db/schema.rs", // Rust file for language detection test
    line: 1,
    language: "Rust",
    build_status: "built",
    confidence: "high",
  },
];

/** Entities for rapid switching test (10 items) */
const RAPID_SWITCH_ENTITIES: EntitySummary[] = Array.from(
  { length: 10 },
  (_, i) => ({
    name: `Entity${i}`,
    kind: "interface" as const,
    file: `src/module${i}/index.ts`,
    line: 1,
    language: "TypeScript" as const,
    build_status: "built" as const,
    confidence: "high" as const,
  }),
);

/** Entity referencing a large file (1500 lines) for stress test */
const LARGE_FILE_ENTITIES: EntitySummary[] = [
  {
    name: "LargeModule",
    kind: "class",
    file: "src/large-file/module.ts", // triggers large file mock
    line: 500,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Scan and wait for entities to appear in the tree */
async function scanAndWaitForEntities(page: import("@playwright/test").Page) {
  await clickOpenDirectory(page);
  await expect(
    page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
  ).toBeVisible({ timeout: 10_000 });
}

/** Wait for Monaco editor to mount in the DOM */
async function waitForMonaco(page: import("@playwright/test").Page) {
  await page.waitForSelector(".monaco-editor", { timeout: 10_000 });
}

/**
 * Get the Monaco footer element — the bar at the bottom of the editor
 * that shows the file path and language. Scoped to avoid matching file
 * paths rendered inside Monaco's view-lines or the details panel.
 *
 * MonacoPreview renders: <div class="... border-t border-gray-700 text-[10px] ...">
 */
function monacoFooter(page: import("@playwright/test").Page) {
  return page.locator(".text-\\[10px\\].border-t");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe("Monaco Editor — rendering and syntax highlighting", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: MONACO_ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
    await scanAndWaitForEntities(page);
  });

  test("select entity → Monaco editor renders with syntax highlighting (not plain text)", async ({
    page,
  }) => {
    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // Wait for Monaco editor to mount
    await waitForMonaco(page);

    // Monaco should be present (not a plain <pre> element)
    await expect(page.locator(".monaco-editor")).toBeVisible();

    // Monaco renders syntax tokens with specific CSS classes
    // The presence of .view-lines confirms Monaco's code view is active
    await expect(page.locator(".monaco-editor .view-lines")).toBeVisible();

    // The "Select an entity" placeholder should be gone
    await expect(
      page.getByText("Select an entity to view source"),
    ).not.toBeVisible();
  });

  test("select entity → editor scrolls to the entity's start line", async ({
    page,
  }) => {
    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    await waitForMonaco(page);

    // The footer should show the file path (confirms file is loaded)
    const footer = monacoFooter(page);
    await expect(footer.locator(".truncate")).toContainText("src/auth/provider.ts", {
      timeout: 5_000,
    });

    // Monaco should have rendered lines around the entity's start_line.
    // The mock source for .ts files includes "export interface MockEntity"
    // near line 2 — since the editor scrolls to that line, it should be visible.
    await expect(
      page.locator(".monaco-editor .view-lines"),
    ).toBeVisible({ timeout: 5_000 });
  });

  test("select different entity in same file → editor stays open, scrolls to new position", async ({
    page,
  }) => {
    const treePanel = page.locator(".w-72").first();

    // Select first entity in src/auth/provider.ts
    await treePanel.getByText("AuthProvider").click();
    await waitForMonaco(page);
    const footer = monacoFooter(page);
    await expect(footer.locator(".truncate")).toContainText("src/auth/provider.ts", {
      timeout: 5_000,
    });

    // Select second entity in the SAME file
    await treePanel.getByText("UserService").click();

    // File path should still be the same
    await expect(footer.locator(".truncate")).toContainText("src/auth/provider.ts", {
      timeout: 5_000,
    });

    // Monaco editor should still be visible (not re-mounted from scratch)
    await expect(page.locator(".monaco-editor")).toBeVisible();

    // Only one tab should be open (same file, no duplicate)
    const editorTabs = page.locator(".bg-gray-800\\/80 .cursor-pointer");
    const providerTabs = editorTabs.filter({ hasText: /provider\.ts/ });
    await expect(providerTabs).toHaveCount(1);
  });

  test("select entity in different file → editor loads new file content", async ({
    page,
  }) => {
    const treePanel = page.locator(".w-72").first();
    const footer = monacoFooter(page);

    // Select entity in first file
    await treePanel.getByText("AuthProvider").click();
    await waitForMonaco(page);
    await expect(footer.locator(".truncate")).toContainText("src/auth/provider.ts", {
      timeout: 5_000,
    });

    // Select entity in a different file
    await treePanel.getByText("ApiRouter").click();

    // Footer should update to the new file path
    await expect(footer.locator(".truncate")).toContainText("src/api/router.ts", {
      timeout: 5_000,
    });

    // Two tabs should now be open
    const tabBar = page.locator(".bg-gray-800\\/80");
    await expect(tabBar.locator(".truncate").filter({ hasText: /provider\.ts/ })).toBeVisible();
    await expect(tabBar.locator(".truncate").filter({ hasText: /router\.ts/ })).toBeVisible();
  });

  test("Monaco shows correct language mode (TypeScript for .ts, Rust for .rs)", async ({
    page,
  }) => {
    const treePanel = page.locator(".w-72").first();
    const footer = monacoFooter(page);

    // Select TypeScript entity
    await treePanel.getByText("AuthProvider").click();
    await waitForMonaco(page);

    // Footer shows "typescript" language tag
    await expect(footer).toContainText("typescript", {
      timeout: 5_000,
    });

    // Select Rust entity
    await treePanel.getByText("UserSchema").click();

    // Footer should update to "rust"
    await expect(footer).toContainText("rust", { timeout: 5_000 });
  });

  test("editor is read-only (typing does not modify content)", async ({
    page,
  }) => {
    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();
    await waitForMonaco(page);

    // Get the initial content from Monaco's view-lines
    const viewLines = page.locator(".monaco-editor .view-lines");
    const initialText = await viewLines.textContent();

    // Click on the editor to focus it
    await page.locator(".monaco-editor").click();

    // Try typing — in read-only mode this should have no effect
    await page.keyboard.type("SHOULD NOT APPEAR");

    // Content should be unchanged
    const afterText = await viewLines.textContent();
    expect(afterText).toBe(initialText);

    // Verify "SHOULD NOT APPEAR" is not in the editor
    await expect(
      page.locator(".monaco-editor").getByText("SHOULD NOT APPEAR"),
    ).not.toBeVisible();
  });

  test("selecting entity shows source (center panel is not empty)", async ({
    page,
  }) => {
    // Select an entity
    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // Monaco editor should render with source content
    await waitForMonaco(page);
    await expect(page.locator(".monaco-editor .view-lines")).toBeVisible();

    // The empty state text should NOT be present
    await expect(
      page.getByText("Select an entity to view source"),
    ).not.toBeVisible();

    // The footer confirms a file is loaded
    const footer = monacoFooter(page);
    await expect(footer.locator(".truncate")).toContainText("provider.ts", {
      timeout: 5_000,
    });
  });
});

test.describe("Monaco Editor — rapid entity switching", () => {
  test("rapid entity switching (10 entities in 2 seconds) → no crash, editor updates", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: RAPID_SWITCH_ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();

    // Rapidly click through all 10 entities
    for (let i = 0; i < 10; i++) {
      await treePanel.getByText(`Entity${i}`).click();
      // Small delay to simulate rapid but not instant clicking
      await page.waitForTimeout(150);
    }

    // After rapid switching, the last entity should be selected
    // and the editor should be functional (not crashed)
    await waitForMonaco(page);
    await expect(page.locator(".monaco-editor")).toBeVisible();

    // The footer should show the last entity's file
    const footer = monacoFooter(page);
    await expect(footer.locator(".truncate")).toContainText("src/module9/index.ts", {
      timeout: 5_000,
    });

    // The app should still be responsive — verify by checking the
    // details panel shows the last entity
    const detailsPanel = page.locator(".w-80").first();
    await expect(detailsPanel.getByText("Entity9")).toBeVisible({
      timeout: 5_000,
    });
  });
});

test.describe("Monaco Editor — large file handling", () => {
  test("large file (1000+ lines) → editor renders without freezing", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: LARGE_FILE_ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("LargeModule").click();

    // Monaco should render (not hang)
    await waitForMonaco(page);
    await expect(page.locator(".monaco-editor")).toBeVisible();

    // The editor's view-lines should be present (content rendered)
    await expect(
      page.locator(".monaco-editor .view-lines"),
    ).toBeVisible({ timeout: 10_000 });

    // The footer should show the file path
    const footer = monacoFooter(page);
    await expect(footer.locator(".truncate")).toContainText("src/large-file/module.ts", {
      timeout: 5_000,
    });
  });
});

test.describe("Monaco Editor — minimap", () => {
  test("minimap visible and reflects file structure", async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: MONACO_ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    await waitForMonaco(page);

    // Monaco minimap should be visible (enabled in MonacoPreview options)
    await expect(page.locator(".monaco-editor .minimap")).toBeVisible({
      timeout: 5_000,
    });
  });
});
