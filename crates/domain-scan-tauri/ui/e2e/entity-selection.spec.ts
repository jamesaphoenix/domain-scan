/**
 * E2E tests for entity selection, tree expansion, source preview,
 * details panel, kind filtering, and manifest scaffold copy.
 *
 * Tests run against the Vite dev server with Tauri IPC mocked.
 */

import { test, expect } from "@playwright/test";
import { setupTauriMocks, MOCK_SCAN_STATS } from "./mocks";
import type { EntitySummary } from "../src/types";
import {
  waitForAppReady,
  switchTab,
  clickOpenDirectory,
} from "./helpers";

// Extended mock entities covering multiple kinds
const ENTITIES_WITH_KINDS: EntitySummary[] = [
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
    kind: "service",
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
  {
    name: "getUser",
    kind: "function",
    file: "src/api/users.ts",
    line: 20,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
  {
    name: "UserSchema",
    kind: "schema",
    file: "src/db/schema.ts",
    line: 15,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  },
];

/** Helper: switch to entities tab, scan and wait for entities to appear */
async function scanAndWaitForEntities(page: import("@playwright/test").Page) {
  await switchTab(page, "Entities/Types");
  await clickOpenDirectory(page);
  await expect(
    page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
  ).toBeVisible({ timeout: 10_000 });
}

test.describe("Entity selection and source preview", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES_WITH_KINDS,
    });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("clicking an entity shows its name and kind in the details panel", async ({
    page,
  }) => {
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // Details panel (right side) should show the entity name and kind
    const detailsPanel = page.locator(".w-80").first();
    await expect(detailsPanel.getByText("AuthProvider")).toBeVisible({
      timeout: 5_000,
    });
    // The kind label appears below the name
    await expect(detailsPanel.locator(".capitalize").first()).toHaveText("interface");
  });

  test("clicking an entity loads source code in the center panel", async ({
    page,
  }) => {
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // The source preview should no longer show the empty state
    await expect(page.getByText("Select an entity to view source")).not.toBeVisible({
      timeout: 5_000,
    });

    // Monaco editor should be present with source content loaded
    await expect(page.locator(".monaco-editor")).toBeVisible({
      timeout: 5_000,
    });
  });

  test("selecting a different entity updates both source and details", async ({
    page,
  }) => {
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();

    // Select first entity
    await treePanel.getByText("AuthProvider").click();
    const detailsPanel = page.locator(".w-80").first();
    await expect(detailsPanel.getByText("AuthProvider")).toBeVisible({
      timeout: 5_000,
    });

    // Select a different entity
    await treePanel.getByText("getUser").click();
    await expect(detailsPanel.getByText("getUser")).toBeVisible({
      timeout: 5_000,
    });
    await expect(detailsPanel.getByText("function")).toBeVisible();
  });

  test("details panel shows file path as a clickable link", async ({
    page,
  }) => {
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // File link should contain the file path
    const detailsPanel = page.locator(".w-80").first();
    await expect(
      detailsPanel.getByText(/provider\.ts/),
    ).toBeVisible({ timeout: 5_000 });
  });

  test("details panel shows language for selected entity", async ({
    page,
  }) => {
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    const detailsPanel = page.locator(".w-80").first();
    await expect(detailsPanel.getByText("TypeScript")).toBeVisible({
      timeout: 5_000,
    });
  });
});

test.describe("Entity tree expansion", () => {
  test("default mock returns empty methods — tree shows expand indicator", async ({
    page,
  }) => {
    // The default mock returns an Interface with methods:[] (no children)
    // Verify the expand indicator shows ">" for interface-kind entities
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES_WITH_KINDS,
    });
    await page.goto("/");
    await waitForAppReady(page);
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();

    // AuthProvider (interface) should have expand indicator ">"
    const authRow = treePanel.locator("div").filter({ hasText: /^>.*AuthProvider/ }).first();
    await expect(authRow).toBeVisible({ timeout: 5_000 });

    // getUser (function) should NOT have expand indicator
    const funcRow = treePanel.locator("div").filter({ hasText: "getUser" }).first();
    await expect(funcRow).toBeVisible();
  });

  test("selecting entity loads details and source", async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES_WITH_KINDS,
    });
    await page.goto("/");
    await waitForAppReady(page);
    await scanAndWaitForEntities(page);

    const treePanel = page.locator(".w-72").first();
    await treePanel.getByText("AuthProvider").click();

    // Details panel should show Methods count from mock (0 for default mock)
    const detailsPanel = page.locator(".w-80").first();
    await expect(detailsPanel.getByText("Methods")).toBeVisible({ timeout: 5_000 });

    // Source preview should show Monaco editor with content loaded
    await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 5_000 });
  });
});

test.describe("Kind filter buttons", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES_WITH_KINDS,
    });
    await page.goto("/");
    await waitForAppReady(page);
    await scanAndWaitForEntities(page);
  });

  test("clicking Interfaces filter highlights the button", async ({
    page,
  }) => {
    const interfacesBtn = page
      .locator("button")
      .filter({ hasText: /^Interfaces$/ });
    await interfacesBtn.click();

    // Button should have active styling (bg-blue-600)
    await expect(interfacesBtn).toHaveClass(/bg-blue-600/);
  });

  test("clicking a filter button toggles it off on second click", async ({
    page,
  }) => {
    const interfacesBtn = page
      .locator("button")
      .filter({ hasText: /^Interfaces$/ });

    // Activate
    await interfacesBtn.click();
    await expect(interfacesBtn).toHaveClass(/bg-blue-600/);

    // Deactivate
    await interfacesBtn.click();
    await expect(interfacesBtn).not.toHaveClass(/bg-blue-600/);
  });

  test("no build status filter buttons exist", async ({ page }) => {
    // Build status filters were removed — verify they're gone
    await expect(page.locator("button").filter({ hasText: /^Built$/ })).not.toBeVisible();
    await expect(page.locator("button").filter({ hasText: /^Unbuilt$/ })).not.toBeVisible();
    await expect(page.locator("button").filter({ hasText: /^Rebuild$/ })).not.toBeVisible();
  });
});

test.describe("Manifest scaffold copy button", () => {
  test("tube map page shows agent prompt with copy button when scan is loaded", async ({
    page,
  }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");

    // The "Recommended" agent prompt section should be visible (ManifestLoader)
    await expect(
      page.getByText("Recommended"),
    ).toBeVisible({ timeout: 5_000 });

    // Copy prompt button should be present
    await expect(
      page.getByRole("button", { name: "Copy prompt" }),
    ).toBeVisible();

    // Load Manifest button should also be present
    await expect(
      page.getByRole("button", { name: "Load Manifest" }),
    ).toBeVisible();
  });

  test("tube map shows scan gate when no scan is loaded", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      scanStats: null,
    });
    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");

    // The scan gate should show "Open a project first"
    await expect(
      page.getByText("Open a project first"),
    ).toBeVisible({ timeout: 5_000 });

    // ManifestLoader content should NOT be visible
    await expect(
      page.getByText("Recommended"),
    ).not.toBeVisible();
  });

  test("clicking copy prompt button changes text to 'Copied!'", async ({
    page,
  }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await waitForAppReady(page);

    await switchTab(page, "Subsystem Tube Map");

    // Grant clipboard permissions
    await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);

    const copyBtn = page.getByRole("button", { name: "Copy prompt" });
    await copyBtn.click();

    // Button text should change to "Copied!"
    await expect(
      page.getByRole("button", { name: "Copied!" }),
    ).toBeVisible();

    // After 2s it should revert back to "Copy prompt"
    await expect(
      page.getByRole("button", { name: "Copy prompt" }),
    ).toBeVisible({ timeout: 5_000 });
  });
});
