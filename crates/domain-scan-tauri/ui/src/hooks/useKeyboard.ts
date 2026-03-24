import { useEffect, useCallback, useRef } from "react";
import type { Tab } from "../components/TabBar";

export interface KeyboardActions {
  onMoveUp: () => void;
  onMoveDown: () => void;
  onExpandOrSelect: () => void;
  onCollapse: () => void;
  onSearch: () => void;
  onPrompt: () => void;
  onExport: () => void;
}

/**
 * Keyboard navigation hook with input-focus guards and tab awareness.
 * Entity-tab shortcuts (j/k/h/l/p/e) only fire when activeTab === "entities".
 * `/` (search) fires on both tabs (tube map handles its own via TubeMapView).
 * Escape always blurs inputs.
 */
export function useKeyboard(
  actions: KeyboardActions,
  activeTab: Tab = "entities",
): void {
  const actionsRef = useRef(actions);
  actionsRef.current = actions;

  const activeTabRef = useRef(activeTab);
  activeTabRef.current = activeTab;

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    const target = e.target as HTMLElement;
    const isInputFocused =
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable;

    // Escape always blurs inputs
    if (e.key === "Escape" && isInputFocused) {
      target.blur();
      e.preventDefault();
      return;
    }

    // Skip navigation keys when typing in inputs
    if (isInputFocused) return;

    // Only fire entities-tab shortcuts when on entities tab
    if (activeTabRef.current !== "entities") return;

    switch (e.key) {
      case "j":
      case "ArrowDown":
        e.preventDefault();
        actionsRef.current.onMoveDown();
        break;
      case "k":
      case "ArrowUp":
        e.preventDefault();
        actionsRef.current.onMoveUp();
        break;
      case "Enter":
      case "ArrowRight":
      case "l":
        e.preventDefault();
        actionsRef.current.onExpandOrSelect();
        break;
      case "ArrowLeft":
      case "h":
        e.preventDefault();
        actionsRef.current.onCollapse();
        break;
      case "/":
        e.preventDefault();
        actionsRef.current.onSearch();
        break;
      case "p":
        e.preventDefault();
        actionsRef.current.onPrompt();
        break;
      case "e":
        e.preventDefault();
        actionsRef.current.onExport();
        break;
    }
  }, []);

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);
}
