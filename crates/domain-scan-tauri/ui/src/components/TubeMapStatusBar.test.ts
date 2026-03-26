import { describe, it, expect } from "vitest";
import { formatZoomPercent, buildFilterText } from "./TubeMapStatusBar";
import type { DomainDef } from "../types";

// ---------------------------------------------------------------------------
// formatZoomPercent
// ---------------------------------------------------------------------------

describe("formatZoomPercent", () => {
  it("converts 1.0 to 100", () => {
    expect(formatZoomPercent(1.0)).toBe(100);
  });

  it("converts 0.5 to 50", () => {
    expect(formatZoomPercent(0.5)).toBe(50);
  });

  it("converts 1.5 to 150", () => {
    expect(formatZoomPercent(1.5)).toBe(150);
  });

  it("converts 2.0 to 200", () => {
    expect(formatZoomPercent(2.0)).toBe(200);
  });

  it("converts 0.0 to 0", () => {
    expect(formatZoomPercent(0.0)).toBe(0);
  });

  it("rounds 0.333 to 33", () => {
    expect(formatZoomPercent(0.333)).toBe(33);
  });

  it("rounds 0.667 to 67", () => {
    expect(formatZoomPercent(0.667)).toBe(67);
  });

  it("rounds 0.755 to 76 (Math.round rounds 75.5 to 76)", () => {
    expect(formatZoomPercent(0.755)).toBe(76);
  });

  it("handles very small zoom values", () => {
    expect(formatZoomPercent(0.01)).toBe(1);
  });

  it("handles very large zoom values", () => {
    expect(formatZoomPercent(10.0)).toBe(1000);
  });

  it("returns NaN for NaN input", () => {
    expect(formatZoomPercent(NaN)).toBeNaN();
  });
});

// ---------------------------------------------------------------------------
// buildFilterText
// ---------------------------------------------------------------------------

describe("buildFilterText", () => {
  const domains: Record<string, DomainDef> = {
    "platform-core": { label: "Platform Core", color: "#3b82f6" },
    "media-storage": { label: "Media & Storage", color: "#22c55e" },
    services: { label: "Services", color: "#f97316" },
  };

  it("returns 'No filters' when both filters are 'all'", () => {
    const result = buildFilterText("all", "all", domains);
    expect(result.text).toBe("No filters");
    expect(result.hasActiveFilter).toBe(false);
  });

  it("returns domain label when domain filter is set", () => {
    const result = buildFilterText("platform-core", "all", domains);
    expect(result.text).toBe("Platform Core");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("returns capitalized status when status filter is set", () => {
    const result = buildFilterText("all", "built", domains);
    expect(result.text).toBe("Built");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("joins domain and status with ' + ' when both are set", () => {
    const result = buildFilterText("platform-core", "built", domains);
    expect(result.text).toBe("Platform Core + Built");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("falls back to domain ID when domain not found in domains map", () => {
    const result = buildFilterText("unknown-domain", "all", domains);
    expect(result.text).toBe("unknown-domain");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("capitalizes status filter correctly for multi-word status", () => {
    const result = buildFilterText("all", "needs-review", domains);
    expect(result.text).toBe("Needs-review");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("capitalizes single character status", () => {
    const result = buildFilterText("all", "a", domains);
    expect(result.text).toBe("A");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("handles empty domains map with domain filter", () => {
    const result = buildFilterText("some-domain", "all", {});
    expect(result.text).toBe("some-domain");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("uses domain label from domains map (media-storage)", () => {
    const result = buildFilterText("media-storage", "all", domains);
    expect(result.text).toBe("Media & Storage");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("handles status filter 'unbuilt'", () => {
    const result = buildFilterText("all", "unbuilt", domains);
    expect(result.text).toBe("Unbuilt");
    expect(result.hasActiveFilter).toBe(true);
  });

  it("joins multiple parts: domain + status for services + error", () => {
    const result = buildFilterText("services", "error", domains);
    expect(result.text).toBe("Services + Error");
    expect(result.hasActiveFilter).toBe(true);
  });
});
