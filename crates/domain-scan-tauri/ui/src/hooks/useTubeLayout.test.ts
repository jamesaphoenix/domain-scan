import { describe, it, expect } from "vitest";
import { buildIndividualEdge, BUNDLE_THRESHOLD } from "./useTubeLayout";
import type { TubeMapSubsystem, TubeMapConnection } from "../types";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeSub(
  id: string,
  domain: string,
  overrides?: Partial<TubeMapSubsystem>,
): TubeMapSubsystem {
  return {
    id,
    name: id,
    domain,
    status: "built",
    description: `${id} description`,
    file_path: `/project/src/${id}/`,
    matched_entity_count: 0,
    interface_count: 0,
    operation_count: 0,
    table_count: 0,
    event_count: 0,
    has_children: false,
    child_count: 0,
    dependency_count: 0,
    ...overrides,
  };
}

function makeConn(
  from: string,
  to: string,
  overrides?: Partial<TubeMapConnection>,
): TubeMapConnection {
  return {
    from,
    to,
    label: `${from}->${to}`,
    type: "depends_on",
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// buildIndividualEdge
// ---------------------------------------------------------------------------

describe("buildIndividualEdge", () => {
  it("builds an edge with correct id, source, and target", () => {
    const conn = makeConn("auth", "billing");
    const sourceSub = makeSub("auth", "security");
    const targetSub = makeSub("billing", "payments");
    const domainColors = new Map([
      ["security", "#ff0000"],
      ["payments", "#00ff00"],
    ]);

    const edge = buildIndividualEdge(conn, sourceSub, targetSub, domainColors);

    expect(edge.id).toBe("auth->billing");
    expect(edge.source).toBe("auth");
    expect(edge.target).toBe("billing");
    expect(edge.type).toBe("dependency");
  });

  it("uses domain colors from the map", () => {
    const conn = makeConn("a", "b");
    const sourceSub = makeSub("a", "domain-x");
    const targetSub = makeSub("b", "domain-y");
    const domainColors = new Map([
      ["domain-x", "#aaa"],
      ["domain-y", "#bbb"],
    ]);

    const edge = buildIndividualEdge(conn, sourceSub, targetSub, domainColors);
    const data = edge.data as Record<string, unknown>;

    expect(data.sourceDomainColor).toBe("#aaa");
    expect(data.targetDomainColor).toBe("#bbb");
  });

  it("falls back to gray when domain color is missing", () => {
    const conn = makeConn("a", "b");
    const sourceSub = makeSub("a", "unknown-domain");
    const targetSub = makeSub("b", "another-unknown");
    const domainColors = new Map<string, string>();

    const edge = buildIndividualEdge(conn, sourceSub, targetSub, domainColors);
    const data = edge.data as Record<string, unknown>;

    expect(data.sourceDomainColor).toBe("#6b7280");
    expect(data.targetDomainColor).toBe("#6b7280");
  });

  it("handles undefined source subsystem gracefully", () => {
    const conn = makeConn("unknown-source", "b");
    const targetSub = makeSub("b", "payments");
    const domainColors = new Map([["payments", "#00ff00"]]);

    const edge = buildIndividualEdge(conn, undefined, targetSub, domainColors);
    const data = edge.data as Record<string, unknown>;

    expect(data.sourceName).toBe("unknown-source");
    expect(data.targetName).toBe("b");
    // Source domain is empty string, not in map, so fallback
    expect(data.sourceDomainColor).toBe("#6b7280");
  });

  it("handles undefined target subsystem gracefully", () => {
    const conn = makeConn("a", "unknown-target");
    const sourceSub = makeSub("a", "security");
    const domainColors = new Map([["security", "#ff0000"]]);

    const edge = buildIndividualEdge(conn, sourceSub, undefined, domainColors);
    const data = edge.data as Record<string, unknown>;

    expect(data.sourceName).toBe("a");
    expect(data.targetName).toBe("unknown-target");
    expect(data.targetDomainColor).toBe("#6b7280");
  });

  it("handles both subsystems undefined", () => {
    const conn = makeConn("x", "y");
    const domainColors = new Map<string, string>();

    const edge = buildIndividualEdge(conn, undefined, undefined, domainColors);
    const data = edge.data as Record<string, unknown>;

    expect(edge.id).toBe("x->y");
    expect(data.sourceName).toBe("x");
    expect(data.targetName).toBe("y");
  });

  it("preserves connection type in edge data", () => {
    const conn = makeConn("a", "b", { type: "api_call" as TubeMapConnection["type"] });
    const sourceSub = makeSub("a", "d1");
    const targetSub = makeSub("b", "d2");
    const domainColors = new Map<string, string>();

    const edge = buildIndividualEdge(conn, sourceSub, targetSub, domainColors);
    const data = edge.data as Record<string, unknown>;

    expect(data.connectionType).toBe("api_call");
  });

  it("preserves connection label in edge data", () => {
    const conn = makeConn("a", "b", { label: "authenticates-via" });
    const sourceSub = makeSub("a", "d1");
    const targetSub = makeSub("b", "d2");
    const domainColors = new Map<string, string>();

    const edge = buildIndividualEdge(conn, sourceSub, targetSub, domainColors);
    const data = edge.data as Record<string, unknown>;

    expect(data.label).toBe("authenticates-via");
  });
});

// ---------------------------------------------------------------------------
// BUNDLE_THRESHOLD constant
// ---------------------------------------------------------------------------

describe("BUNDLE_THRESHOLD", () => {
  it("is set to 3", () => {
    expect(BUNDLE_THRESHOLD).toBe(3);
  });
});
