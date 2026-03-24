import { useEffect, useCallback, useRef } from "react";

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
 * Keyboard navigation hook with input-focus guards.
 * When an input/textarea is focused, navigation keys are ignored
 * so the user can type freely. Escape always blurs inputs.
 */
export function useKeyboard(actions: KeyboardActions): void {
  const actionsRef = useRef(actions);
  actionsRef.current = actions;

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
