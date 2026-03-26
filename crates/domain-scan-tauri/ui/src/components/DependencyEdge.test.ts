import { describe, it, expect } from "vitest";
import { getTubeStyle } from "./DependencyEdge";
import type { ConnectionType } from "../types";

// ---------------------------------------------------------------------------
// getTubeStyle — pure function for edge styling based on connection type
// ---------------------------------------------------------------------------

describe("getTubeStyle", () => {
  // --- depends_on ---

  it("depends_on: uses sourceDomainColor as stroke", () => {
    const style = getTubeStyle("depends_on", "#3b82f6", false);
    expect(style.stroke).toBe("#3b82f6");
  });

  it("depends_on: has width 5 when not hovered", () => {
    const style = getTubeStyle("depends_on", "#3b82f6", false);
    expect(style.strokeWidth).toBe(5);
  });

  it("depends_on: has width 7 when hovered", () => {
    const style = getTubeStyle("depends_on", "#3b82f6", true);
    expect(style.strokeWidth).toBe(7);
  });

  it("depends_on: has no dasharray", () => {
    const style = getTubeStyle("depends_on", "#3b82f6", false);
    expect(style.strokeDasharray).toBeUndefined();
  });

  it("depends_on: has opacity 1 when not hovered", () => {
    const style = getTubeStyle("depends_on", "#3b82f6", false);
    expect(style.opacity).toBe(1);
  });

  it("depends_on: has opacity 1 when hovered", () => {
    const style = getTubeStyle("depends_on", "#3b82f6", true);
    expect(style.opacity).toBe(1);
  });

  // --- uses ---

  it("uses: uses sourceDomainColor as stroke", () => {
    const style = getTubeStyle("uses", "#22c55e", false);
    expect(style.stroke).toBe("#22c55e");
  });

  it("uses: has width 2 when not hovered", () => {
    const style = getTubeStyle("uses", "#22c55e", false);
    expect(style.strokeWidth).toBe(2);
  });

  it("uses: has width 3.5 when hovered", () => {
    const style = getTubeStyle("uses", "#22c55e", true);
    expect(style.strokeWidth).toBe(3.5);
  });

  it("uses: has no dasharray", () => {
    const style = getTubeStyle("uses", "#22c55e", false);
    expect(style.strokeDasharray).toBeUndefined();
  });

  it("uses: has opacity 0.6 when not hovered", () => {
    const style = getTubeStyle("uses", "#22c55e", false);
    expect(style.opacity).toBe(0.6);
  });

  it("uses: has opacity 1 when hovered", () => {
    const style = getTubeStyle("uses", "#22c55e", true);
    expect(style.opacity).toBe(1);
  });

  // --- triggers ---

  it("triggers: uses fixed amber color #f59e0b as stroke (ignores sourceDomainColor)", () => {
    const style = getTubeStyle("triggers", "#3b82f6", false);
    expect(style.stroke).toBe("#f59e0b");
  });

  it("triggers: has width 5 when not hovered", () => {
    const style = getTubeStyle("triggers", "#3b82f6", false);
    expect(style.strokeWidth).toBe(5);
  });

  it("triggers: has width 7 when hovered", () => {
    const style = getTubeStyle("triggers", "#3b82f6", true);
    expect(style.strokeWidth).toBe(7);
  });

  it("triggers: has dashed stroke pattern '12 6'", () => {
    const style = getTubeStyle("triggers", "#3b82f6", false);
    expect(style.strokeDasharray).toBe("12 6");
  });

  it("triggers: has opacity 1 when not hovered", () => {
    const style = getTubeStyle("triggers", "#3b82f6", false);
    expect(style.opacity).toBe(1);
  });

  // --- cross-connection-type consistency ---

  it("all connection types return an object with stroke, strokeWidth, strokeDasharray, opacity", () => {
    const types: ConnectionType[] = ["depends_on", "uses", "triggers"];
    for (const connType of types) {
      const style = getTubeStyle(connType, "#000", false);
      expect(style).toHaveProperty("stroke");
      expect(style).toHaveProperty("strokeWidth");
      expect(style).toHaveProperty("strokeDasharray");
      expect(style).toHaveProperty("opacity");
    }
  });

  it("hovered always sets opacity to 1 for all connection types", () => {
    const types: ConnectionType[] = ["depends_on", "uses", "triggers"];
    for (const connType of types) {
      const style = getTubeStyle(connType, "#000", true);
      expect(style.opacity).toBe(1);
    }
  });

  it("hovered strokeWidth is always greater than non-hovered strokeWidth", () => {
    const types: ConnectionType[] = ["depends_on", "uses", "triggers"];
    for (const connType of types) {
      const normal = getTubeStyle(connType, "#000", false);
      const hovered = getTubeStyle(connType, "#000", true);
      expect(hovered.strokeWidth).toBeGreaterThan(normal.strokeWidth);
    }
  });

  // --- edge cases ---

  it("uses empty string as sourceDomainColor for depends_on", () => {
    const style = getTubeStyle("depends_on", "", false);
    expect(style.stroke).toBe("");
  });

  it("triggers ignores any sourceDomainColor value", () => {
    const style1 = getTubeStyle("triggers", "#ff0000", false);
    const style2 = getTubeStyle("triggers", "#00ff00", false);
    expect(style1.stroke).toBe(style2.stroke);
    expect(style1.stroke).toBe("#f59e0b");
  });
});
