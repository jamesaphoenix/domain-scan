/**
 * E2E test helpers for the Domain Scan Tauri app.
 *
 * Tests run against the Vite dev server (http://localhost:5173).
 * Tauri IPC calls are intercepted via `page.exposeFunction` or
 * by running tests against the full Tauri app via `cargo tauri dev`.
 *
 * When TAURI_TEST=1 is set, the app reads manifests from test
 * fixture paths instead of showing native file dialogs.
 */

import { type Page, expect } from "@playwright/test";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/** Absolute path to the e2e/fixtures directory */
export const FIXTURES_DIR = path.resolve(__dirname, "fixtures");

/** Get the absolute path to a fixture file */
export function fixturePath(name: string): string {
  return path.join(FIXTURES_DIR, name);
}

// ---------------------------------------------------------------------------
// App lifecycle
// ---------------------------------------------------------------------------

/**
 * Wait for the app to be fully loaded (React has mounted).
 * Checks that the status bar and tab bar are present.
 */
export async function waitForAppReady(page: Page): Promise<void> {
  // Wait for the Domain Scan label in the status bar
  await page.waitForSelector("text=Domain Scan", { timeout: 10_000 });
  // Wait for tab bar to render
  await page.waitForSelector("text=Entities/Types", { timeout: 5_000 });
  await page.waitForSelector("text=Subsystem Tube Map", { timeout: 5_000 });
}

/**
 * Switch to a specific tab by clicking it.
 */
export async function switchTab(
  page: Page,
  tab: "Entities/Types" | "Subsystem Tube Map",
): Promise<void> {
  await page.getByRole("button", { name: tab }).click();
}

/**
 * Wait for the tube map canvas to render after a manifest is loaded.
 * Looks for the React Flow container and subsystem nodes.
 */
export async function waitForTubeMap(page: Page): Promise<void> {
  // React Flow renders a div with class "react-flow"
  await page.waitForSelector(".react-flow", { timeout: 10_000 });
}

/**
 * Wait for scanning to complete by checking the stats bar updates.
 */
export async function waitForScan(page: Page): Promise<void> {
  // Wait for the scanning indicator to disappear and stats to appear
  await page.waitForFunction(
    () => {
      const text = document.body.innerText;
      return text.includes("files") && text.includes("interfaces") && !text.includes("Scanning...");
    },
    { timeout: 30_000 },
  );
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/**
 * Click "Open Directory" in the status bar.
 * Note: In E2E mode, the native dialog may need to be mocked or the
 * path injected via the TAURI_TEST_SCAN_PATH environment variable.
 */
export async function clickOpenDirectory(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Open Directory" }).first().click();
}

/**
 * Click the "Open Manifest" button to trigger the file picker.
 * In test mode, the manifest path should be injected.
 */
export async function clickLoadManifest(page: Page): Promise<void> {
  const button = page.getByRole("button", { name: /open manifest/i });
  await button.click();
}

/**
 * Get the currently active tab label.
 */
export async function getActiveTab(page: Page): Promise<string> {
  const activeButton = page.locator("button.bg-gray-700.text-white");
  return (await activeButton.textContent()) ?? "";
}

/**
 * Search in the tube map search bar.
 */
export async function searchTubeMap(
  page: Page,
  query: string,
): Promise<void> {
  const searchInput = page.locator(
    'input[placeholder*="Search"]',
  ).first();
  await searchInput.fill(query);
}

/**
 * Clear the tube map search.
 */
export async function clearSearch(page: Page): Promise<void> {
  const searchInput = page.locator(
    'input[placeholder*="Search"]',
  ).first();
  await searchInput.fill("");
}

/**
 * Count the number of visible subsystem nodes on the React Flow canvas.
 */
export async function countVisibleNodes(page: Page): Promise<number> {
  return page.locator('[data-testid="rf__node-default"], .react-flow__node').count();
}

/**
 * Get the text content of the status bar.
 */
export async function getStatusBarText(page: Page): Promise<string> {
  const statusBar = page.locator(".flex.items-center.justify-between").first();
  return (await statusBar.textContent()) ?? "";
}

/**
 * Check if the shortcut help overlay is visible.
 */
export async function isShortcutHelpVisible(page: Page): Promise<boolean> {
  const overlay = page.locator("text=Keyboard Shortcuts");
  return overlay.isVisible();
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/**
 * Assert that the app is on the specified tab.
 */
export async function assertActiveTab(
  page: Page,
  tab: "Entities/Types" | "Subsystem Tube Map",
): Promise<void> {
  const activeTab = page.locator("button.bg-gray-700.text-white");
  await expect(activeTab).toHaveText(tab);
}

/**
 * Assert that the tube map shows either the scan gate or the manifest loader.
 */
export async function assertManifestLoaderVisible(page: Page): Promise<void> {
  // Could be the scan gate ("Open a project first") or the manifest loader ("Recommended")
  const scanGate = page.getByText("Open a project first");
  const manifestLoader = page.getByText("Recommended");
  await expect(scanGate.or(manifestLoader)).toBeVisible({ timeout: 5_000 });
}

/**
 * Assert that an error toast appears with the given text.
 */
export async function assertErrorToast(
  page: Page,
  text: string | RegExp,
): Promise<void> {
  const toast = page.locator('[role="alert"], .text-red-400');
  await expect(toast.filter({ hasText: text })).toBeVisible({ timeout: 5_000 });
}

/**
 * Assert that the coverage overlay shows the expected values.
 */
export async function assertCoverage(
  page: Page,
  expectedPercent: number,
): Promise<void> {
  const coverageText = page.getByText(`${expectedPercent}%`);
  await expect(coverageText).toBeVisible();
}

/**
 * Assert that breadcrumbs contain the expected items.
 */
export async function assertBreadcrumbs(
  page: Page,
  expected: string[],
): Promise<void> {
  for (const item of expected) {
    await expect(page.getByText(item)).toBeVisible();
  }
}

// ---------------------------------------------------------------------------
// Keyboard
// ---------------------------------------------------------------------------

/**
 * Press a key on the page (for keyboard shortcut testing).
 */
export async function pressKey(page: Page, key: string): Promise<void> {
  await page.keyboard.press(key);
}

/**
 * Type text into the currently focused element.
 */
export async function typeText(page: Page, text: string): Promise<void> {
  await page.keyboard.type(text);
}
