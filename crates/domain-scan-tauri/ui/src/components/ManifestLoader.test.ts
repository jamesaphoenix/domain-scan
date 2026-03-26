import { describe, it, expect } from "vitest";
import { buildAgentPrompt } from "./ManifestLoader";
import type { PlatformReleaseInfo } from "./ManifestLoader";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeReleaseInfo(
  overrides?: Partial<PlatformReleaseInfo>,
): PlatformReleaseInfo {
  return {
    os: "macos",
    arch: "aarch64",
    latest_tag: "v0.5.0",
    assets: [],
    matching_asset: null,
    cargo_install_cmd:
      "cargo install --force domain-scan-cli --git https://github.com/jamesaphoenix/domain-scan.git",
    recommended_install_cmd: "curl -L ... | tar xz",
    recommended_update_cmd: "cargo install --force domain-scan-cli",
    scanned_root: null,
    installed_path: null,
    installed_version: null,
    doctor_supported: false,
    update_available: null,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// buildAgentPrompt — dynamic prompt generation
// ---------------------------------------------------------------------------

describe("buildAgentPrompt", () => {
  it("returns a valid prompt when info is null (generic fallback)", () => {
    const prompt = buildAgentPrompt(null);

    expect(prompt).toContain("# Build a system.json manifest");
    expect(prompt).toContain("Step 1");
    expect(prompt).toContain("Step 2");
    expect(prompt).toContain("cargo install");
    // Should not contain platform-specific info
    expect(prompt).not.toContain("macos");
    expect(prompt).not.toContain("aarch64");
  });

  it("includes platform info when info is provided", () => {
    const info = makeReleaseInfo({ os: "macos", arch: "aarch64" });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("macos");
    expect(prompt).toContain("aarch64");
  });

  it("shows matching asset download when available", () => {
    const info = makeReleaseInfo({
      matching_asset: {
        name: "domain-scan-macos-aarch64.tar.gz",
        download_url: "https://example.com/download",
        size: 10 * 1024 * 1024,
      },
      latest_tag: "v0.5.0",
    });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("domain-scan-macos-aarch64.tar.gz");
    expect(prompt).toContain("v0.5.0");
    expect(prompt).toContain("10.0 MB");
    expect(prompt).toContain("pre-built binary is available");
  });

  it("falls back to cargo install when no matching asset", () => {
    const info = makeReleaseInfo({
      matching_asset: null,
      assets: [
        {
          name: "domain-scan-linux-x86_64.tar.gz",
          download_url: "https://example.com",
          size: 5000000,
        },
      ],
    });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("No pre-built binary is available");
    expect(prompt).toContain("cargo install");
    // Should list available assets
    expect(prompt).toContain("domain-scan-linux-x86_64.tar.gz");
  });

  it("shows doctor workflow when CLI is installed with doctor support", () => {
    const info = makeReleaseInfo({
      installed_path: "/usr/local/bin/domain-scan",
      installed_version: "0.4.0",
      doctor_supported: true,
      update_available: false,
    });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("already installed at");
    expect(prompt).toContain("/usr/local/bin/domain-scan");
    expect(prompt).toContain("version 0.4.0");
    expect(prompt).toContain("domain-scan doctor --output json");
    expect(prompt).toContain("update_available");
  });

  it("shows upgrade workflow when CLI is installed without doctor", () => {
    const info = makeReleaseInfo({
      installed_path: "/usr/local/bin/domain-scan",
      installed_version: "0.2.0",
      doctor_supported: false,
    });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("predates the `doctor` command");
    expect(prompt).toContain("Upgrade");
  });

  it("shows update available message when update is needed", () => {
    const info = makeReleaseInfo({
      installed_path: "/usr/local/bin/domain-scan",
      doctor_supported: true,
      update_available: true,
    });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("`update_available` is `true`");
  });

  it("shows keep existing message when no update needed", () => {
    const info = makeReleaseInfo({
      installed_path: "/usr/local/bin/domain-scan",
      doctor_supported: true,
      update_available: false,
    });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("`update_available` is `false`");
    expect(prompt).toContain("keep the existing install");
  });

  it("includes scanned_root in prompt when provided", () => {
    const info = makeReleaseInfo({
      scanned_root: "/Users/james/myproject",
    });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("/Users/james/myproject");
    expect(prompt).toContain("--root /Users/james/myproject");
  });

  it("does not include scanned_root section when null", () => {
    const info = makeReleaseInfo({ scanned_root: null });
    const prompt = buildAgentPrompt(info);

    expect(prompt).not.toContain("--root");
    expect(prompt).not.toContain("Project directory");
  });

  it("includes all required steps", () => {
    const prompt = buildAgentPrompt(null);

    expect(prompt).toContain("Step 1");
    expect(prompt).toContain("Step 2");
    expect(prompt).toContain("Step 3");
    expect(prompt).toContain("Step 4");
    expect(prompt).toContain("Step 5");
    expect(prompt).toContain("Step 6");
    expect(prompt).toContain("Step 7");
    expect(prompt).toContain("Step 8");
  });

  it("includes hard rules", () => {
    const prompt = buildAgentPrompt(null);

    expect(prompt).toContain("Hard rules");
    expect(prompt).toContain("filePath must be a real directory");
    expect(prompt).toContain("Entity names must come from scan output");
    expect(prompt).toContain("kebab-case IDs only");
  });

  it("includes skills installation instructions", () => {
    const prompt = buildAgentPrompt(null);

    expect(prompt).toContain("domain-scan skills install --claude-code");
    expect(prompt).toContain("domain-scan skills install --codex");
  });

  it("handles empty assets array with no matching asset", () => {
    const info = makeReleaseInfo({
      matching_asset: null,
      assets: [],
    });
    const prompt = buildAgentPrompt(info);

    expect(prompt).toContain("No pre-built binary is available");
    // Should not contain "Available release assets" when empty
    expect(prompt).not.toContain("Available release assets");
  });

  it("handles info with null latest_tag", () => {
    const info = makeReleaseInfo({
      latest_tag: null,
      matching_asset: {
        name: "domain-scan-macos.tar.gz",
        download_url: "https://example.com",
        size: 5000000,
      },
    });
    const prompt = buildAgentPrompt(info);

    // Should still produce a valid prompt (uses "latest" as fallback tag)
    expect(prompt).toContain("latest");
  });

  it("handles null installed_version gracefully", () => {
    const info = makeReleaseInfo({
      installed_path: "/usr/local/bin/domain-scan",
      installed_version: null,
      doctor_supported: true,
      update_available: null,
    });
    const prompt = buildAgentPrompt(info);

    // Should not include "version null" in output
    expect(prompt).not.toContain("version null");
    expect(prompt).toContain("already installed at");
  });
});
