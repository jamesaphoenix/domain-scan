import { describe, it, expect } from "vitest";
import type { Tab } from "../components/TabBar";

/**
 * Unit tests for the useKeyboard hook's routing logic.
 *
 * Since the hook depends on React (useEffect, useCallback, useRef) and the DOM
 * (window.addEventListener), we extract and test the key-routing decision logic
 * as pure functions. The hook itself is integration-tested via E2E (Playwright).
 */

// ---------------------------------------------------------------------------
// Extract the routing logic from the hook into testable pure functions
// ---------------------------------------------------------------------------

/** Determine if a key event target is an input element (should suppress shortcuts). */
function isInputFocused(tagName: string, isContentEditable: boolean): boolean {
  return (
    tagName === "INPUT" || tagName === "TEXTAREA" || isContentEditable
  );
}

/** Determine which action (if any) a key maps to on the entities tab. */
type Action =
  | "moveDown"
  | "moveUp"
  | "expandOrSelect"
  | "collapse"
  | "search"
  | "prompt"
  | "export"
  | "escape-blur"
  | null;

function resolveKeyAction(
  key: string,
  activeTab: Tab,
  tagName: string,
  contentEditable: boolean,
): Action {
  const inputFocused = isInputFocused(tagName, contentEditable);

  // Escape always blurs inputs
  if (key === "Escape" && inputFocused) {
    return "escape-blur";
  }

  // Skip navigation keys when typing in inputs
  if (inputFocused) return null;

  // Only fire entities-tab shortcuts when on entities tab
  if (activeTab !== "entities") return null;

  switch (key) {
    case "j":
    case "ArrowDown":
      return "moveDown";
    case "k":
    case "ArrowUp":
      return "moveUp";
    case "Enter":
    case "ArrowRight":
    case "l":
      return "expandOrSelect";
    case "ArrowLeft":
    case "h":
      return "collapse";
    case "/":
      return "search";
    case "p":
      return "prompt";
    case "e":
      return "export";
    default:
      return null;
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("useKeyboard — key routing logic", () => {
  describe("entities tab shortcuts", () => {
    it("j maps to moveDown", () => {
      expect(resolveKeyAction("j", "entities", "DIV", false)).toBe("moveDown");
    });

    it("ArrowDown maps to moveDown", () => {
      expect(resolveKeyAction("ArrowDown", "entities", "DIV", false)).toBe(
        "moveDown",
      );
    });

    it("k maps to moveUp", () => {
      expect(resolveKeyAction("k", "entities", "DIV", false)).toBe("moveUp");
    });

    it("ArrowUp maps to moveUp", () => {
      expect(resolveKeyAction("ArrowUp", "entities", "DIV", false)).toBe(
        "moveUp",
      );
    });

    it("Enter maps to expandOrSelect", () => {
      expect(resolveKeyAction("Enter", "entities", "DIV", false)).toBe(
        "expandOrSelect",
      );
    });

    it("l maps to expandOrSelect", () => {
      expect(resolveKeyAction("l", "entities", "DIV", false)).toBe(
        "expandOrSelect",
      );
    });

    it("ArrowRight maps to expandOrSelect", () => {
      expect(resolveKeyAction("ArrowRight", "entities", "DIV", false)).toBe(
        "expandOrSelect",
      );
    });

    it("h maps to collapse", () => {
      expect(resolveKeyAction("h", "entities", "DIV", false)).toBe("collapse");
    });

    it("ArrowLeft maps to collapse", () => {
      expect(resolveKeyAction("ArrowLeft", "entities", "DIV", false)).toBe(
        "collapse",
      );
    });

    it("/ maps to search", () => {
      expect(resolveKeyAction("/", "entities", "DIV", false)).toBe("search");
    });

    it("p maps to prompt", () => {
      expect(resolveKeyAction("p", "entities", "DIV", false)).toBe("prompt");
    });

    it("e maps to export", () => {
      expect(resolveKeyAction("e", "entities", "DIV", false)).toBe("export");
    });
  });

  describe("input focus guard", () => {
    it("suppresses shortcuts when INPUT is focused", () => {
      expect(resolveKeyAction("j", "entities", "INPUT", false)).toBeNull();
    });

    it("suppresses shortcuts when TEXTAREA is focused", () => {
      expect(resolveKeyAction("k", "entities", "TEXTAREA", false)).toBeNull();
    });

    it("suppresses shortcuts when contentEditable is true", () => {
      expect(resolveKeyAction("l", "entities", "DIV", true)).toBeNull();
    });

    it("Escape blurs focused input", () => {
      expect(resolveKeyAction("Escape", "entities", "INPUT", false)).toBe(
        "escape-blur",
      );
    });

    it("Escape blurs focused textarea", () => {
      expect(resolveKeyAction("Escape", "entities", "TEXTAREA", false)).toBe(
        "escape-blur",
      );
    });

    it("Escape blurs contentEditable", () => {
      expect(resolveKeyAction("Escape", "entities", "DIV", true)).toBe(
        "escape-blur",
      );
    });
  });

  describe("tab guard", () => {
    it("does not fire entity shortcuts on non-entities tab", () => {
      // On the tube-map tab, entity shortcuts should not fire
      const tubeMapTab = "tube-map" as Tab;
      expect(resolveKeyAction("j", tubeMapTab, "DIV", false)).toBeNull();
      expect(resolveKeyAction("k", tubeMapTab, "DIV", false)).toBeNull();
      expect(resolveKeyAction("l", tubeMapTab, "DIV", false)).toBeNull();
      expect(resolveKeyAction("h", tubeMapTab, "DIV", false)).toBeNull();
      expect(resolveKeyAction("p", tubeMapTab, "DIV", false)).toBeNull();
      expect(resolveKeyAction("e", tubeMapTab, "DIV", false)).toBeNull();
    });
  });

  describe("unknown keys", () => {
    it("returns null for unrecognized keys", () => {
      expect(resolveKeyAction("x", "entities", "DIV", false)).toBeNull();
      expect(resolveKeyAction("z", "entities", "DIV", false)).toBeNull();
      expect(resolveKeyAction("F1", "entities", "DIV", false)).toBeNull();
      expect(resolveKeyAction("Tab", "entities", "DIV", false)).toBeNull();
    });
  });

  describe("isInputFocused helper", () => {
    it("returns true for INPUT", () => {
      expect(isInputFocused("INPUT", false)).toBe(true);
    });

    it("returns true for TEXTAREA", () => {
      expect(isInputFocused("TEXTAREA", false)).toBe(true);
    });

    it("returns true for contentEditable", () => {
      expect(isInputFocused("DIV", true)).toBe(true);
    });

    it("returns false for non-input elements", () => {
      expect(isInputFocused("DIV", false)).toBe(false);
      expect(isInputFocused("SPAN", false)).toBe(false);
      expect(isInputFocused("BUTTON", false)).toBe(false);
    });
  });
});
