/**
 * E2E tests for the Tube Map scan gate and agent prompt.
 *
 * Verifies:
 * - Tube Map tab shows "Open a project first" when no scan loaded
 * - After opening a directory and scanning, the ManifestLoader appears
 * - The agent prompt contains the scanned project path
 * - The agent prompt contains the release version and platform info
 * - Copy prompt button works
 */

import { test, expect } from "@playwright/test";
import { setupTauriMocks, MOCK_SCAN_STATS } from "./mocks";
import { waitForAppReady, switchTab } from "./helpers";
import type { EntitySummary } from "../src/types";

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
];

test.describe("Tube Map scan gate", () => {
  test("shows both Open Directory and Open Manifest when no scan is loaded", async ({
    page,
  }) => {
    // Set scanStats to null — no scan loaded
    await setupTauriMocks(page, {
      scanStats: null,
      scanError: "No scan loaded",
    });
    await page.goto("/");
    await waitForAppReady(page);

    await expect(page.getByText("Recommended")).toBeVisible({ timeout: 5_000 });
    await expect(
      page.getByRole("button", { name: "Open Directory" }).first(),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Open Manifest" }),
    ).toBeVisible();
  });

  test("shows ManifestLoader after scanning a directory", async ({ page }) => {
    // Scan stats are available — scan was performed
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);

    // The scan gate should detect the loaded scan and show ManifestLoader
    await expect(
      page.getByText("Recommended"),
    ).toBeVisible({ timeout: 5_000 });
  });

  test("Open Directory button is present on the empty-state loader", async ({
    page,
  }) => {
    await setupTauriMocks(page, {
      scanStats: null,
      scanError: "No scan loaded",
    });
    await page.goto("/");
    await waitForAppReady(page);

    const openBtn = page.getByRole("button", { name: "Open Directory" }).first();
    await expect(openBtn).toBeVisible({ timeout: 5_000 });
    await expect(openBtn).toBeEnabled();
  });
});

test.describe("Agent prompt content", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("agent prompt contains release version badge", async ({ page }) => {
    // The release badge should show the version
    await expect(
      page.getByText("v0.4.0"),
    ).toBeVisible({ timeout: 5_000 });
  });

  test("agent prompt contains platform info in badge", async ({ page }) => {
    await expect(
      page.getByText("darwin/aarch64"),
    ).toBeVisible({ timeout: 5_000 });
  });

  test("agent prompt shows 'binary available' indicator", async ({ page }) => {
    await expect(
      page.getByText("binary available"),
    ).toBeVisible({ timeout: 5_000 });
  });

  test("agent prompt uses doctor when the CLI is already installed", async ({ page }) => {
    const expandBtn = page.getByRole("button", { name: "Expand" });
    await expandBtn.click();

    await expect(
      page.getByText("domain-scan doctor --output json"),
    ).toBeVisible({ timeout: 3_000 });
  });

  test("expand button shows full prompt", async ({ page }) => {
    const expandBtn = page.getByRole("button", { name: "Expand" });
    await expandBtn.click();

    // Full prompt should be visible — check for content from later steps
    await expect(
      page.getByText("Step 2"),
    ).toBeVisible({ timeout: 3_000 });
    await expect(
      page.getByText("Install agent skills"),
    ).toBeVisible();
  });

  test("copy prompt button works", async ({ page }) => {
    await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);

    const copyBtn = page.getByRole("button", { name: "Copy prompt" });
    await copyBtn.click();

    await expect(
      page.getByRole("button", { name: "Copied!" }),
    ).toBeVisible({ timeout: 3_000 });

    // Should revert back
    await expect(
      page.getByRole("button", { name: "Copy prompt" }),
    ).toBeVisible({ timeout: 5_000 });
  });

  test("agent prompt contains project path when scanned", async ({ page }) => {
    const expandBtn = page.getByRole("button", { name: "Expand" });
    await expandBtn.click();

    // The prompt should include the scanned root path
    await expect(
      page.getByText("/mock/test-project"),
    ).toBeVisible({ timeout: 3_000 });
  });
});

test.describe("ManifestLoader layout", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page, {
      dialogResult: "/mock/test-project",
      scanStats: MOCK_SCAN_STATS,
      entities: ENTITIES,
    });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("agent prompt section appears before Open Manifest button", async ({
    page,
  }) => {
    // "Recommended" label should be visible (appears above the prompt)
    const recommended = page.getByText("Recommended");
    await expect(recommended).toBeVisible({ timeout: 5_000 });

    const loadBtn = page.getByRole("button", { name: "Open Manifest" });
    await expect(loadBtn).toBeVisible();

    // Check vertical order: Recommended is above Open Manifest
    const recBox = await recommended.boundingBox();
    const loadBox = await loadBtn.boundingBox();
    expect(recBox).not.toBeNull();
    expect(loadBox).not.toBeNull();
    if (recBox && loadBox) {
      expect(recBox.y).toBeLessThan(loadBox.y);
    }
  });

  test("wizard link is at the bottom and is a text link, not a button", async ({
    page,
  }) => {
    const wizardLink = page.getByText("or create manually with the wizard");
    await expect(wizardLink).toBeVisible({ timeout: 5_000 });

    // Should be below Open Manifest
    const loadBtn = page.getByRole("button", { name: "Open Manifest" });
    const loadBox = await loadBtn.boundingBox();
    const wizardBox = await wizardLink.boundingBox();
    if (loadBox && wizardBox) {
      expect(wizardBox.y).toBeGreaterThan(loadBox.y);
    }
  });

  test("What is a manifest? expandable section works", async ({ page }) => {
    const toggle = page.getByText("What is a manifest?");
    await expect(toggle).toBeVisible();

    // Content should be hidden initially
    await expect(page.getByText("system manifest")).not.toBeVisible();

    // Click to expand
    await toggle.click();
    await expect(page.getByText("system manifest")).toBeVisible({ timeout: 3_000 });

    // Click to collapse
    await toggle.click();
    await expect(page.getByText("system manifest")).not.toBeVisible({ timeout: 3_000 });
  });
});
