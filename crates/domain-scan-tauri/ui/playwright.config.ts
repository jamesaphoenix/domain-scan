import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright config for domain-scan Tauri E2E tests.
 *
 * Uses port 5179 to avoid conflicts with other Vite dev servers.
 * Tauri IPC calls are mocked via `e2e/mocks.ts` (injected before page load).
 *
 * For full Tauri integration (native dialogs, IPC), run with:
 *   TAURI_TEST=1 cargo tauri dev
 * and then:
 *   npm run test:e2e
 */
const E2E_PORT = 5179;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? "github" : "html",
  timeout: 30_000,

  use: {
    baseURL: `http://localhost:${E2E_PORT}`,
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
    command: `npx vite --port ${E2E_PORT}`,
    url: `http://localhost:${E2E_PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
