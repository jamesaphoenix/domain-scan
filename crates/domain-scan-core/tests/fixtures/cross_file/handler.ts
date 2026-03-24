// handler.ts - Implements EventHandler from types.ts

import { EventHandler } from "./types";

export class LogEventHandler implements EventHandler {
  handle(event: Event): void {
    // log the event
  }

  onError(error: Error): void {
    // log the error
  }

  cleanup(): void {
    // cleanup resources
  }
}

export class MetricsHandler implements EventHandler {
  handle(event: Event): void {
    // record metrics
  }

  onError(error: Error): void {
    // record error metric
  }

  // Missing: cleanup() - intentionally incomplete
}
