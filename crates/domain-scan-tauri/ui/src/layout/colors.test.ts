import { describe, it, expect } from "vitest";
import { assignDomainColors } from "./colors";

describe("assignDomainColors", () => {
  it("uses manifest-specified colors when available", () => {
    const manifest = {
      core: { label: "Core", color: "#ff0000" },
      api: { label: "API", color: "#00ff00" },
    };
    const result = assignDomainColors(manifest, ["core", "api"]);
    expect(result.get("core")).toBe("#ff0000");
    expect(result.get("api")).toBe("#00ff00");
  });

  it("falls back to static palette for domains without manifest color", () => {
    const result = assignDomainColors({}, ["a", "b", "c"]);
    expect(result.get("a")).toBe("#3b82f6"); // blue (first palette color)
    expect(result.get("b")).toBe("#22c55e"); // green (second)
    expect(result.get("c")).toBe("#f97316"); // orange (third)
  });

  it("mixes manifest colors with palette fallbacks", () => {
    const manifest = {
      api: { label: "API", color: "#custom1" },
    };
    const result = assignDomainColors(manifest, ["core", "api", "data"]);
    // core has no manifest color → gets first palette slot
    expect(result.get("core")).toBe("#3b82f6");
    // api has manifest color → uses it
    expect(result.get("api")).toBe("#custom1");
    // data has no manifest color → gets second palette slot
    expect(result.get("data")).toBe("#22c55e");
  });

  it("returns a color for every domain ID", () => {
    const ids = Array.from({ length: 20 }, (_, i) => `domain-${i}`);
    const result = assignDomainColors({}, ids);
    expect(result.size).toBe(20);
    for (const id of ids) {
      expect(result.has(id)).toBe(true);
      expect(result.get(id)).toMatch(/^#[0-9a-f]{6}$/);
    }
  });

  it("generates HSL colors when palette is exhausted (>12 domains)", () => {
    const ids = Array.from({ length: 15 }, (_, i) => `d-${i}`);
    const result = assignDomainColors({}, ids);
    // First 12 use static palette
    expect(result.get("d-0")).toBe("#3b82f6");
    expect(result.get("d-11")).toBe("#84cc16");
    // 13th+ use HSL-generated colors
    const color12 = result.get("d-12")!;
    expect(color12).toMatch(/^#[0-9a-f]{6}$/);
    expect(color12).not.toBe("#3b82f6"); // not recycling palette
  });

  it("handles empty domain list", () => {
    const result = assignDomainColors({}, []);
    expect(result.size).toBe(0);
  });

  it("handles single domain", () => {
    const result = assignDomainColors({}, ["only"]);
    expect(result.size).toBe(1);
    expect(result.get("only")).toBe("#3b82f6");
  });

  it("skips manifest entries with empty color string", () => {
    const manifest = {
      core: { label: "Core", color: "" },
    };
    const result = assignDomainColors(manifest, ["core"]);
    // Empty color is falsy → falls back to palette
    expect(result.get("core")).toBe("#3b82f6");
  });

  it("preserves domain order in map", () => {
    const ids = ["zebra", "alpha", "middle"];
    const result = assignDomainColors({}, ids);
    const keys = [...result.keys()];
    expect(keys).toEqual(["zebra", "alpha", "middle"]);
  });
});
