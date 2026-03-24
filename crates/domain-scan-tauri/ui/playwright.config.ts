import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright config for domain-scan Tauri E2E tests.
 *
 * Tests run against the Vite dev server at http://localhost:5173.
 * The Tauri app's webview loads from this same URL during `cargo tauri dev`,
 * so UI behavior is identical between browser and Tauri webview.
 *
 * For full Tauri integration (native dialogs, IPC), run with:
 *   TAURI_TEST=1 cargo tauri dev
 * and then:
 *   npm run test:e2e
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? "github" : "html",
  timeout: 30_000,

  use: {
    baseURL: "http://localhost:5173",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  webServer: {
    command: "npm run dev",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
