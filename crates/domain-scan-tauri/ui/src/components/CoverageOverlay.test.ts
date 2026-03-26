import { describe, it, expect } from "vitest";
import { clampCoverage, getBarColorClass, shouldShowOverlay } from "./CoverageOverlay";

// ---------------------------------------------------------------------------
// clampCoverage
// ---------------------------------------------------------------------------

describe("clampCoverage", () => {
  it("returns the value unchanged for 0%", () => {
    expect(clampCoverage(0)).toBe(0);
  });

  it("returns the value unchanged for 50%", () => {
    expect(clampCoverage(50)).toBe(50);
  });

  it("returns the value unchanged for 100%", () => {
    expect(clampCoverage(100)).toBe(100);
  });

  it("clamps values above 100 to 100", () => {
    expect(clampCoverage(150)).toBe(100);
    expect(clampCoverage(200)).toBe(100);
    expect(clampCoverage(100.1)).toBe(100);
  });

  it("does not clamp negative values (Math.min only has upper bound)", () => {
    expect(clampCoverage(-10)).toBe(-10);
  });

  it("returns NaN for NaN input", () => {
    expect(clampCoverage(NaN)).toBeNaN();
  });

  it("clamps Infinity to 100", () => {
    expect(clampCoverage(Infinity)).toBe(100);
  });

  it("does not clamp -Infinity", () => {
    expect(clampCoverage(-Infinity)).toBe(-Infinity);
  });

  it("handles fractional percentages", () => {
    expect(clampCoverage(79.9)).toBe(79.9);
    expect(clampCoverage(99.99)).toBe(99.99);
  });
});

// ---------------------------------------------------------------------------
// getBarColorClass
// ---------------------------------------------------------------------------

describe("getBarColorClass", () => {
  it("returns emerald for 100%", () => {
    expect(getBarColorClass(100)).toBe("bg-emerald-500");
  });

  it("returns emerald for exactly 80%", () => {
    expect(getBarColorClass(80)).toBe("bg-emerald-500");
  });

  it("returns emerald for 95%", () => {
    expect(getBarColorClass(95)).toBe("bg-emerald-500");
  });

  it("returns amber for 79.9%", () => {
    expect(getBarColorClass(79.9)).toBe("bg-amber-500");
  });

  it("returns amber for exactly 50%", () => {
    expect(getBarColorClass(50)).toBe("bg-amber-500");
  });

  it("returns amber for 65%", () => {
    expect(getBarColorClass(65)).toBe("bg-amber-500");
  });

  it("returns red for 49.9%", () => {
    expect(getBarColorClass(49.9)).toBe("bg-red-500");
  });

  it("returns red for 0%", () => {
    expect(getBarColorClass(0)).toBe("bg-red-500");
  });

  it("returns red for 25%", () => {
    expect(getBarColorClass(25)).toBe("bg-red-500");
  });

  it("returns red for negative values", () => {
    expect(getBarColorClass(-10)).toBe("bg-red-500");
  });

  it("returns red for NaN (NaN < 50 is false, NaN >= 80 is false)", () => {
    // NaN >= 80 is false, NaN >= 50 is false, so it falls through to red
    expect(getBarColorClass(NaN)).toBe("bg-red-500");
  });
});

// ---------------------------------------------------------------------------
// shouldShowOverlay
// ---------------------------------------------------------------------------

describe("shouldShowOverlay", () => {
  it("returns false when both totalEntities and unmatchedCount are 0", () => {
    expect(shouldShowOverlay(0, 0)).toBe(false);
  });

  it("returns true when totalEntities > 0", () => {
    expect(shouldShowOverlay(10, 0)).toBe(true);
  });

  it("returns true when unmatchedCount > 0", () => {
    expect(shouldShowOverlay(0, 5)).toBe(true);
  });

  it("returns true when both are > 0", () => {
    expect(shouldShowOverlay(10, 5)).toBe(true);
  });

  it("returns true when totalEntities is 1 and unmatchedCount is 0", () => {
    expect(shouldShowOverlay(1, 0)).toBe(true);
  });

  it("returns true when totalEntities is 0 and unmatchedCount is 1", () => {
    expect(shouldShowOverlay(0, 1)).toBe(true);
  });
});
