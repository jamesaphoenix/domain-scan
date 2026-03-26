/**
 * Phase F.2: E2E tests for Open Directory & Scan Flow.
 *
 * Tests run against the Vite dev server with Tauri IPC mocked
 * via `setupTauriMocks()` (injected before page load).
 */

import { test, expect } from "@playwright/test";
import {
  setupTauriMocks,
  MOCK_SCAN_STATS,
  MOCK_ENTITIES,
} from "./mocks";
import { waitForAppReady, clickOpenDirectory, getStatusBarText, switchTab } from "./helpers";

test.describe("F.2: Open Directory & Scan Flow", () => {
  test("click Open Directory → dialog opens, scan runs, stats appear", async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: MOCK_ENTITIES,
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Click Open Directory
    await clickOpenDirectory(page);

    // Wait for scan to complete — stats should appear in the status bar
    await expect(
      page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
    ).toBeVisible({ timeout: 10_000 });

    // Verify specific counts in the stats bar
    const statusText = await getStatusBarText(page);
    expect(statusText).toContain(`${MOCK_SCAN_STATS.total_interfaces} interfaces`);
    expect(statusText).toContain(`${MOCK_SCAN_STATS.total_services} services`);
    expect(statusText).toContain(`${MOCK_SCAN_STATS.total_schemas} schemas`);
  });

  test("scan fixture directory → stats bar shows correct file/entity counts", async ({ page }) => {
    const customStats = {
      ...MOCK_SCAN_STATS,
      total_files: 100,
      total_interfaces: 25,
      total_services: 12,
      total_schemas: 8,
      parse_duration_ms: 250,
    };

    await setupTauriMocks(page, {
      dialogResult: "/mock/large-project",
      scanStats: customStats,
      entities: MOCK_ENTITIES,
    });

    await page.goto("/");
    await waitForAppReady(page);

    await clickOpenDirectory(page);

    // Verify exact counts from our custom stats
    await expect(page.getByText("100 files")).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText("25 interfaces")).toBeVisible();
    await expect(page.getByText("12 services")).toBeVisible();
    await expect(page.getByText("8 schemas")).toBeVisible();
    await expect(page.getByText("250ms")).toBeVisible();
  });

  test("scan completes → entities tab shows tree with nodes", async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: MOCK_ENTITIES,
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Switch to the Entities tab (app starts on Tube Map by default)
    await switchTab(page, "Entities/Types");
    const entitiesTab = page.getByRole("button", { name: "Entities/Types" });
    await expect(entitiesTab).toBeVisible();

    // Trigger scan
    await clickOpenDirectory(page);

    // Wait for scan to complete
    await expect(
      page.getByText(`${MOCK_SCAN_STATS.total_files} files`),
    ).toBeVisible({ timeout: 10_000 });

    // The entity tree (left panel) should show our mock entities
    const treePanel = page.locator(".w-72").first();
    await expect(treePanel.getByText("AuthProvider")).toBeVisible({ timeout: 5_000 });
    await expect(treePanel.getByText("UserService")).toBeVisible();
    await expect(treePanel.getByText("ApiRouter")).toBeVisible();
  });

  test("scan empty directory → structured error shown (not a crash)", async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/empty-directory",
      scanStats: null,
      scanError: "No supported source files found in directory",
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Switch to Entities tab to avoid duplicate "Open Directory" buttons
    // (scan gate on Tube Map also shows "Open Directory" when no scan is loaded)
    await switchTab(page, "Entities/Types");

    // Trigger scan
    await clickOpenDirectory(page);

    // The error should appear in the status bar (useScan sets error state)
    // and/or a toast should appear
    await expect(
      page.getByText(/no supported source files|scan failed/i),
    ).toBeVisible({ timeout: 10_000 });

    // App should still be functional — tab bar should be present
    await expect(page.getByText("Entities/Types")).toBeVisible();
    await expect(page.getByText("Subsystem Tube Map")).toBeVisible();
  });

  test("scan non-existent path → structured error shown", async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/nonexistent/path/that/does/not/exist",
      scanStats: null,
      scanError: "Directory not found: /nonexistent/path/that/does/not/exist",
    });

    await page.goto("/");
    await waitForAppReady(page);

    // Switch to Entities tab to avoid duplicate "Open Directory" buttons
    await switchTab(page, "Entities/Types");

    // Trigger scan
    await clickOpenDirectory(page);

    // Error should be shown — either in status bar or toast
    await expect(
      page.getByText(/directory not found|scan failed/i),
    ).toBeVisible({ timeout: 10_000 });

    // App should remain stable — tabs still present and clickable
    await expect(page.getByText("Entities/Types")).toBeVisible();
    await expect(page.getByText("Subsystem Tube Map")).toBeVisible();

    // Status bar still shows the app name (not a blank crash page)
    await expect(page.getByText("Domain Scan")).toBeVisible();
  });
});
