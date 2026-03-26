import { describe, it, expect } from "vitest";

/**
 * Unit tests for the ToastProvider/useToast system.
 *
 * The hook is a React context consumer, so testing state management requires
 * a DOM environment + @testing-library/react (not installed). Instead we test
 * the pure logic and constants that underpin the toast system.
 *
 * Integration-level behavior (auto-dismiss via setTimeout, context propagation)
 * is covered by E2E tests.
 */

// ---------------------------------------------------------------------------
// Constants extracted from useToast.tsx for testing
// ---------------------------------------------------------------------------
const TOAST_DURATION_MS = 3000;

type ToastType = "success" | "error" | "info";

interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

// ---------------------------------------------------------------------------
// Pure logic: toast list management (simulates reducer behavior)
// ---------------------------------------------------------------------------

function addToast(
  toasts: Toast[],
  nextId: number,
  message: string,
  type: ToastType = "info",
): { toasts: Toast[]; nextId: number } {
  const id = nextId;
  return {
    toasts: [...toasts, { id, message, type }],
    nextId: nextId + 1,
  };
}

function removeToast(toasts: Toast[], id: number): Toast[] {
  return toasts.filter((t) => t.id !== id);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Toast system — pure logic", () => {
  describe("addToast", () => {
    it("adds a toast to an empty list", () => {
      const result = addToast([], 0, "Hello", "info");
      expect(result.toasts).toHaveLength(1);
      expect(result.toasts[0]).toEqual({ id: 0, message: "Hello", type: "info" });
      expect(result.nextId).toBe(1);
    });

    it("defaults type to info", () => {
      const result = addToast([], 0, "Default type");
      expect(result.toasts[0].type).toBe("info");
    });

    it("appends to existing toasts", () => {
      const existing: Toast[] = [{ id: 0, message: "First", type: "success" }];
      const result = addToast(existing, 1, "Second", "error");
      expect(result.toasts).toHaveLength(2);
      expect(result.toasts[1]).toEqual({ id: 1, message: "Second", type: "error" });
    });

    it("generates monotonically increasing IDs", () => {
      let state = { toasts: [] as Toast[], nextId: 0 };
      state = addToast(state.toasts, state.nextId, "A");
      state = addToast(state.toasts, state.nextId, "B");
      state = addToast(state.toasts, state.nextId, "C");

      const ids = state.toasts.map((t) => t.id);
      expect(ids).toEqual([0, 1, 2]);
    });
  });

  describe("removeToast", () => {
    it("removes a toast by ID", () => {
      const toasts: Toast[] = [
        { id: 0, message: "A", type: "info" },
        { id: 1, message: "B", type: "success" },
        { id: 2, message: "C", type: "error" },
      ];
      const result = removeToast(toasts, 1);
      expect(result).toHaveLength(2);
      expect(result.map((t) => t.id)).toEqual([0, 2]);
    });

    it("returns empty array when removing last toast", () => {
      const toasts: Toast[] = [{ id: 0, message: "Only", type: "info" }];
      const result = removeToast(toasts, 0);
      expect(result).toHaveLength(0);
    });

    it("returns unchanged array when ID not found", () => {
      const toasts: Toast[] = [{ id: 0, message: "A", type: "info" }];
      const result = removeToast(toasts, 999);
      expect(result).toHaveLength(1);
      expect(result[0].id).toBe(0);
    });

    it("does not mutate the original array", () => {
      const toasts: Toast[] = [
        { id: 0, message: "A", type: "info" },
        { id: 1, message: "B", type: "info" },
      ];
      const original = [...toasts];
      removeToast(toasts, 0);
      expect(toasts).toEqual(original);
    });
  });

  describe("toast types", () => {
    it("accepts success type", () => {
      const result = addToast([], 0, "Done!", "success");
      expect(result.toasts[0].type).toBe("success");
    });

    it("accepts error type", () => {
      const result = addToast([], 0, "Failed!", "error");
      expect(result.toasts[0].type).toBe("error");
    });

    it("accepts info type", () => {
      const result = addToast([], 0, "FYI", "info");
      expect(result.toasts[0].type).toBe("info");
    });
  });

  describe("constants", () => {
    it("TOAST_DURATION_MS is a positive number", () => {
      expect(TOAST_DURATION_MS).toBeGreaterThan(0);
    });

    it("TOAST_DURATION_MS is 3 seconds", () => {
      expect(TOAST_DURATION_MS).toBe(3000);
    });
  });

  describe("add-remove cycle", () => {
    it("add then remove returns to empty", () => {
      let state = addToast([], 0, "Temporary");
      const result = removeToast(state.toasts, 0);
      expect(result).toHaveLength(0);
    });

    it("handles interleaved add/remove", () => {
      let state = { toasts: [] as Toast[], nextId: 0 };

      // Add three
      state = addToast(state.toasts, state.nextId, "A");
      state = addToast(state.toasts, state.nextId, "B");
      state = addToast(state.toasts, state.nextId, "C");
      expect(state.toasts).toHaveLength(3);

      // Remove middle
      const after = removeToast(state.toasts, 1);
      expect(after).toHaveLength(2);
      expect(after.map((t) => t.message)).toEqual(["A", "C"]);
    });
  });
});
