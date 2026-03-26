/**
 * E2E tests for click-to-copy functionality on system/subsystem names.
 *
 * Verifies that:
 * - Clicking a subsystem name on the tube map copies it to clipboard
 * - Clicking a child subsystem name in drill-in view copies it to clipboard
 */

import { test, expect } from "@playwright/test";
import { setupTauriMocks, MOCK_OCTOSPARK_TUBE_MAP } from "./mocks";
import { waitForAppReady, clickLoadManifest, waitForTubeMap } from "./helpers";

test.describe("Copy system/subsystem names", () => {
  test.beforeEach(async ({ page, context }) => {
    // Grant clipboard permissions
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);

    await setupTauriMocks(page, {
      tubeMapData: MOCK_OCTOSPARK_TUBE_MAP,
    });
    await page.goto("/");
    await waitForAppReady(page);
  });

  test("subsystem node name has copy cursor and title hint", async ({
    page,
  }) => {
    // Load the tube map
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Find a subsystem node name — look for an h3 with cursor-copy class
    const nodeName = page.locator("h3.cursor-copy").first();
    await expect(nodeName).toBeVisible({ timeout: 10_000 });
    // Verify the title attribute contains "click to copy"
    const title = await nodeName.getAttribute("title");
    expect(title).toContain("click to copy");
  });

  test("clicking subsystem node name copies text to clipboard", async ({
    page,
  }) => {
    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Find a subsystem node name
    const nodeName = page.locator("h3.cursor-copy").first();
    await expect(nodeName).toBeVisible({ timeout: 10_000 });

    const nameText = await nodeName.textContent();

    // Click to copy
    await nodeName.click();

    // Verify clipboard content
    const clipboardText = await page.evaluate(() =>
      navigator.clipboard.readText(),
    );
    expect(clipboardText).toBe(nameText);
  });

  test("drill-in child subsystem name has copy cursor and copies on click", async ({
    page,
  }) => {
    // Update mock to return children for subsystem detail
    await page.evaluate(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const w = window as any;
      // Override get_subsystem_detail to return children
      const origInvoke = w.__TAURI_INTERNALS__.invoke as (cmd: string, args?: Record<string, unknown>) => unknown;
      w.__TAURI_INTERNALS__.invoke = async (
        cmd: string,
        args?: Record<string, unknown>,
      ) => {
        if (cmd === "get_subsystem_detail") {
          return {
            id: args?.subsystemId,
            name: "Auth & Identity",
            domain: "platform-core",
            status: "built",
            file_path: "src/auth/",
            interfaces: ["AuthProvider"],
            operations: ["login"],
            tables: [],
            events: [],
            dependencies: [],
            children: [
              {
                id: "auth-session",
                name: "Session Manager",
                domain: "platform-core",
                status: "built",
                interfaces: ["SessionStore"],
                operations: ["createSession"],
                tables: ["sessions"],
                events: [],
                children: [],
                dependencies: [],
              },
              {
                id: "auth-oauth",
                name: "OAuth Provider",
                domain: "platform-core",
                status: "new",
                interfaces: ["OAuthClient"],
                operations: [],
                tables: [],
                events: [],
                children: [],
                dependencies: [],
              },
            ],
            matched_entities: [],
          };
        }
        return origInvoke(cmd, args);
      };
    });

    await clickLoadManifest(page);
    await waitForTubeMap(page);

    // Find a parent subsystem node (has_children=true) and click "Drill in"
    const drillInButton = page.locator("text=Drill in").first();
    await expect(drillInButton).toBeVisible({ timeout: 10_000 });
    await drillInButton.click();

    // Wait for the drill-in view to load
    await expect(page.getByText("Child Subsystems")).toBeVisible({
      timeout: 10_000,
    });

    // Find a child subsystem name with cursor-copy
    const childName = page.locator(".cursor-copy").filter({ hasText: "Session Manager" });
    await expect(childName).toBeVisible();

    // Click to copy
    await childName.click();

    // Verify clipboard content
    const clipboardText = await page.evaluate(() =>
      navigator.clipboard.readText(),
    );
    expect(clipboardText).toBe("Session Manager");
  });
});
