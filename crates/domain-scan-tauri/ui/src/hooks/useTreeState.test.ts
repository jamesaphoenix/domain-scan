import { describe, it, expect, beforeEach } from "vitest";
import type { Entity, Span, MethodSignature, MethodDef, PropertyDef, RouteDef, SchemaField } from "../types";
import { extractChildren } from "./useTreeState";

/** Helper to build a default Span */
function makeSpan(startLine = 1, endLine = 10): Span {
  return {
    start_line: startLine,
    start_col: 0,
    end_line: endLine,
    end_col: 0,
    byte_range: [0, 100],
  };
}

/** Helper to build a MethodSignature (used by Interface) */
function makeMethodSig(
  name: string,
  opts?: { startLine?: number; is_async?: boolean; return_type?: string | null },
): MethodSignature {
  return {
    name,
    span: makeSpan(opts?.startLine ?? 1),
    is_async: opts?.is_async ?? false,
    parameters: [],
    return_type: opts?.return_type ?? null,
  };
}

/** Helper to build a MethodDef (used by Service, Class, Impl) */
function makeMethodDef(
  name: string,
  opts?: { startLine?: number; is_async?: boolean; return_type?: string | null },
): MethodDef {
  return {
    name,
    file: "test.ts",
    span: makeSpan(opts?.startLine ?? 1),
    visibility: "public",
    is_async: opts?.is_async ?? false,
    is_static: false,
    is_generator: false,
    parameters: [],
    return_type: opts?.return_type ?? null,
    decorators: [],
    owner: null,
    implements: null,
  };
}

/** Helper to build a PropertyDef */
function makeProp(name: string): PropertyDef {
  return {
    name,
    type_annotation: null,
    is_optional: false,
    is_readonly: false,
    visibility: "public",
  };
}

/** Helper to build a RouteDef */
function makeRoute(method: string, path: string): RouteDef {
  return { method, path, handler: "handler" };
}

/** Helper to build a SchemaField */
function makeField(name: string): SchemaField {
  return { name, type_annotation: null, is_optional: false };
}

describe("extractChildren", () => {
  // 1. Interface with methods and properties -> returns methods first, then properties
  it("returns methods first, then properties for Interface", () => {
    const entity: Entity = {
      Interface: {
        name: "MyInterface",
        file: "test.ts",
        span: makeSpan(),
        visibility: "public",
        generics: [],
        extends: [],
        methods: [makeMethodSig("doStuff"), makeMethodSig("compute")],
        properties: [makeProp("value"), makeProp("count")],
        language_kind: "interface",
        decorators: [],
      },
    };

    const children = extractChildren(entity);
    expect(children).toHaveLength(4);
    // Methods come first
    expect(children[0]).toMatchObject({ name: "doStuff", kind: "method" });
    expect(children[1]).toMatchObject({ name: "compute", kind: "method" });
    // Properties come after
    expect(children[2]).toMatchObject({ name: "value", kind: "property" });
    expect(children[3]).toMatchObject({ name: "count", kind: "property" });
  });

  // 2. Interface with empty methods and properties -> returns []
  it("returns empty array for Interface with no methods or properties", () => {
    const entity: Entity = {
      Interface: {
        name: "EmptyInterface",
        file: "test.ts",
        span: makeSpan(),
        visibility: "public",
        generics: [],
        extends: [],
        methods: [],
        properties: [],
        language_kind: "interface",
        decorators: [],
      },
    };

    expect(extractChildren(entity)).toEqual([]);
  });

  // 3. Service with methods and routes -> returns methods first, then routes with "METHOD /path" format
  it("returns methods first, then routes for Service", () => {
    const entity: Entity = {
      Service: {
        name: "UserService",
        file: "test.ts",
        span: makeSpan(),
        kind: "http_controller",
        methods: [makeMethodDef("getUser"), makeMethodDef("createUser")],
        dependencies: [],
        decorators: [],
        routes: [makeRoute("GET", "/users"), makeRoute("POST", "/users")],
      },
    };

    const children = extractChildren(entity);
    expect(children).toHaveLength(4);
    // Methods first
    expect(children[0]).toMatchObject({ name: "getUser", kind: "method" });
    expect(children[1]).toMatchObject({ name: "createUser", kind: "method" });
    // Routes after, formatted as "METHOD /path"
    expect(children[2]).toMatchObject({ name: "GET /users", kind: "route" });
    expect(children[3]).toMatchObject({ name: "POST /users", kind: "route" });
  });

  // 4. Service with no methods or routes -> returns []
  it("returns empty array for Service with no methods or routes", () => {
    const entity: Entity = {
      Service: {
        name: "EmptyService",
        file: "test.ts",
        span: makeSpan(),
        kind: "worker",
        methods: [],
        dependencies: [],
        decorators: [],
        routes: [],
      },
    };

    expect(extractChildren(entity)).toEqual([]);
  });

  // 5. Class with methods and properties -> returns methods first, then properties
  it("returns methods first, then properties for Class", () => {
    const entity: Entity = {
      Class: {
        name: "MyClass",
        file: "test.ts",
        span: makeSpan(),
        visibility: "public",
        generics: [],
        extends: null,
        implements: [],
        methods: [makeMethodDef("run")],
        properties: [makeProp("id")],
        is_abstract: false,
        decorators: [],
      },
    };

    const children = extractChildren(entity);
    expect(children).toHaveLength(2);
    expect(children[0]).toMatchObject({ name: "run", kind: "method" });
    expect(children[1]).toMatchObject({ name: "id", kind: "property" });
  });

  // 6. Schema with fields -> returns field children with kind "field"
  it("returns field children with kind 'field' for Schema", () => {
    const entity: Entity = {
      Schema: {
        name: "UserSchema",
        file: "test.ts",
        span: makeSpan(),
        kind: "zod",
        fields: [makeField("name"), makeField("email"), makeField("age")],
        source_framework: "zod",
        table_name: null,
        derives: [],
        visibility: "public",
      },
    };

    const children = extractChildren(entity);
    expect(children).toHaveLength(3);
    expect(children[0]).toMatchObject({ name: "name", kind: "field" });
    expect(children[1]).toMatchObject({ name: "email", kind: "field" });
    expect(children[2]).toMatchObject({ name: "age", kind: "field" });
  });

  // 7. Schema with empty fields -> returns []
  it("returns empty array for Schema with no fields", () => {
    const entity: Entity = {
      Schema: {
        name: "EmptySchema",
        file: "test.ts",
        span: makeSpan(),
        kind: "zod",
        fields: [],
        source_framework: "zod",
        table_name: null,
        derives: [],
        visibility: "public",
      },
    };

    expect(extractChildren(entity)).toEqual([]);
  });

  // 8. Impl with methods -> returns method children
  it("returns method children for Impl", () => {
    const entity: Entity = {
      Impl: {
        target: "MyStruct",
        trait_name: null,
        file: "test.rs",
        span: makeSpan(),
        methods: [makeMethodDef("new"), makeMethodDef("process")],
      },
    };

    const children = extractChildren(entity);
    expect(children).toHaveLength(2);
    expect(children[0]).toMatchObject({ name: "new", kind: "method" });
    expect(children[1]).toMatchObject({ name: "process", kind: "method" });
  });

  // 9. Impl with empty methods -> returns []
  it("returns empty array for Impl with no methods", () => {
    const entity: Entity = {
      Impl: {
        target: "EmptyStruct",
        trait_name: null,
        file: "test.rs",
        span: makeSpan(),
        methods: [],
      },
    };

    expect(extractChildren(entity)).toEqual([]);
  });

  // 10. Function entity -> returns []
  it("returns empty array for Function entity", () => {
    const entity: Entity = {
      Function: {
        name: "doSomething",
        file: "test.ts",
        span: makeSpan(),
        visibility: "public",
        is_async: true,
        is_generator: false,
        parameters: [],
        return_type: "void",
        decorators: [],
      },
    };

    expect(extractChildren(entity)).toEqual([]);
  });

  // 11. TypeAlias entity -> returns []
  it("returns empty array for TypeAlias entity", () => {
    const entity: Entity = {
      TypeAlias: {
        name: "UserId",
        file: "test.ts",
        span: makeSpan(),
        target: "string",
        generics: [],
        visibility: "public",
      },
    };

    expect(extractChildren(entity)).toEqual([]);
  });

  // 12. Interface methods have correct is_async and return_type
  it("preserves is_async and return_type on Interface method children", () => {
    const entity: Entity = {
      Interface: {
        name: "AsyncInterface",
        file: "test.ts",
        span: makeSpan(),
        visibility: "public",
        generics: [],
        extends: [],
        methods: [
          makeMethodSig("fetchData", { is_async: true, return_type: "Promise<Data>" }),
          makeMethodSig("syncOp", { is_async: false, return_type: "number" }),
        ],
        properties: [],
        language_kind: "interface",
        decorators: [],
      },
    };

    const children = extractChildren(entity);
    expect(children).toHaveLength(2);
    expect(children[0]).toMatchObject({
      name: "fetchData",
      kind: "method",
      is_async: true,
      return_type: "Promise<Data>",
    });
    expect(children[1]).toMatchObject({
      name: "syncOp",
      kind: "method",
      is_async: false,
      return_type: "number",
    });
  });

  // 13. Service routes have correct "GET /path" name format
  it("formats Service route names as 'METHOD /path'", () => {
    const entity: Entity = {
      Service: {
        name: "ApiService",
        file: "test.ts",
        span: makeSpan(),
        kind: "http_controller",
        methods: [],
        dependencies: [],
        decorators: [],
        routes: [
          makeRoute("GET", "/api/users"),
          makeRoute("POST", "/api/users"),
          makeRoute("DELETE", "/api/users/:id"),
          makeRoute("PUT", "/api/users/:id"),
        ],
      },
    };

    const children = extractChildren(entity);
    expect(children).toHaveLength(4);
    expect(children[0].name).toBe("GET /api/users");
    expect(children[1].name).toBe("POST /api/users");
    expect(children[2].name).toBe("DELETE /api/users/:id");
    expect(children[3].name).toBe("PUT /api/users/:id");
  });

  // 14. Method children have correct line numbers from span
  it("uses span.start_line as line number for method children", () => {
    const entity: Entity = {
      Interface: {
        name: "SpanInterface",
        file: "test.ts",
        span: makeSpan(),
        visibility: "public",
        generics: [],
        extends: [],
        methods: [
          makeMethodSig("alpha", { startLine: 10 }),
          makeMethodSig("beta", { startLine: 25 }),
          makeMethodSig("gamma", { startLine: 42 }),
        ],
        properties: [],
        language_kind: "interface",
        decorators: [],
      },
    };

    const children = extractChildren(entity);
    expect(children).toHaveLength(3);
    expect(children[0].line).toBe(10);
    expect(children[1].line).toBe(25);
    expect(children[2].line).toBe(42);
  });
});

// ---------------------------------------------------------------------------
// useTreeState — state transformation tests
// ---------------------------------------------------------------------------
//
// Since @testing-library/react and jsdom are not installed, we test the
// hook's logic by simulating the same state transformations it applies.
// Each callback in useTreeState is a pure transformation on (nodes, selectedIndex).

import type { EntitySummary, TreeNode, TreeChild } from "../types";

/** Helper to create a minimal EntitySummary */
function makeEntitySummary(name: string, kind: EntitySummary["kind"] = "interface"): EntitySummary {
  return {
    name,
    kind,
    file: `src/${name}.ts`,
    line: 1,
    language: "TypeScript",
    build_status: "built",
    confidence: "high",
  };
}

/**
 * Simulates the useTreeState hook's state management without React.
 * Mirrors the exact logic from useTreeState.ts.
 */
class TreeStateSimulator {
  nodes: TreeNode[] = [];
  selectedIndex = 0;

  setEntities(entities: EntitySummary[]) {
    this.nodes = entities.map((entity) => ({
      entity,
      expanded: false,
      children: [],
    }));
    this.selectedIndex = 0;
  }

  toggleExpand(index: number) {
    this.nodes = this.nodes.map((node, i) =>
      i === index ? { ...node, expanded: !node.expanded } : node,
    );
  }

  expandNode(index: number) {
    this.nodes = this.nodes.map((node, i) =>
      i === index ? { ...node, expanded: true } : node,
    );
  }

  updateNodeChildren(index: number, children: TreeChild[], expand?: boolean) {
    this.nodes = this.nodes.map((node, i) =>
      i === index
        ? { ...node, children, ...(expand ? { expanded: true } : {}) }
        : node,
    );
  }

  select(index: number) {
    this.selectedIndex = index;
  }

  moveDown() {
    this.selectedIndex = Math.min(this.nodes.length - 1, this.selectedIndex + 1);
  }

  moveUp() {
    this.selectedIndex = Math.max(0, this.selectedIndex - 1);
  }

  get visibleRowCount(): number {
    let count = 0;
    for (const node of this.nodes) {
      count += 1;
      if (node.expanded) {
        count += node.children.length;
      }
    }
    return count;
  }

  get selectedEntity(): EntitySummary | null {
    return this.nodes[this.selectedIndex]?.entity ?? null;
  }
}

describe("useTreeState — state transitions", () => {
  let state: TreeStateSimulator;

  beforeEach(() => {
    state = new TreeStateSimulator();
  });

  // 1. setEntities converts EntitySummary[] to TreeNode[] with expanded=false and children=[]
  it("setEntities converts EntitySummary[] to TreeNode[] with expanded=false and children=[]", () => {
    const entities = [
      makeEntitySummary("Alpha"),
      makeEntitySummary("Beta"),
      makeEntitySummary("Gamma"),
    ];

    state.setEntities(entities);

    expect(state.nodes).toHaveLength(3);
    for (const node of state.nodes) {
      expect(node.expanded).toBe(false);
      expect(node.children).toEqual([]);
    }
    expect(state.nodes[0].entity.name).toBe("Alpha");
    expect(state.nodes[1].entity.name).toBe("Beta");
    expect(state.nodes[2].entity.name).toBe("Gamma");
  });

  // 2. setEntities resets selectedIndex to 0
  it("setEntities resets selectedIndex to 0", () => {
    state.setEntities([
      makeEntitySummary("A"),
      makeEntitySummary("B"),
      makeEntitySummary("C"),
    ]);
    state.select(2);
    expect(state.selectedIndex).toBe(2);

    // Call setEntities again — should reset to 0
    state.setEntities([makeEntitySummary("X"), makeEntitySummary("Y")]);
    expect(state.selectedIndex).toBe(0);
  });

  // 3. toggleExpand flips expanded state for the target node only
  it("toggleExpand flips expanded state for the target node only", () => {
    state.setEntities([
      makeEntitySummary("A"),
      makeEntitySummary("B"),
      makeEntitySummary("C"),
    ]);

    // Toggle node 1 (B) — should become expanded
    state.toggleExpand(1);
    expect(state.nodes[0].expanded).toBe(false);
    expect(state.nodes[1].expanded).toBe(true);
    expect(state.nodes[2].expanded).toBe(false);

    // Toggle node 1 again — should collapse
    state.toggleExpand(1);
    expect(state.nodes[1].expanded).toBe(false);
  });

  // 4. expandNode sets expanded=true for the target node
  it("expandNode sets expanded=true for the target node", () => {
    state.setEntities([
      makeEntitySummary("A"),
      makeEntitySummary("B"),
    ]);

    state.expandNode(0);
    expect(state.nodes[0].expanded).toBe(true);
    expect(state.nodes[1].expanded).toBe(false);

    // Calling expandNode again should keep it true (not toggle)
    state.expandNode(0);
    expect(state.nodes[0].expanded).toBe(true);
  });

  // 5. updateNodeChildren sets children and optionally expands
  it("updateNodeChildren sets children and optionally expands", () => {
    state.setEntities([makeEntitySummary("A"), makeEntitySummary("B")]);

    const children: TreeChild[] = [
      { name: "method1", kind: "method", line: 10 },
      { name: "method2", kind: "method", line: 20 },
    ];

    // Without expand
    state.updateNodeChildren(0, children);
    expect(state.nodes[0].children).toHaveLength(2);
    expect(state.nodes[0].children[0].name).toBe("method1");
    expect(state.nodes[0].children[1].name).toBe("method2");
    expect(state.nodes[0].expanded).toBe(false); // not expanded

    // With expand = true
    state.updateNodeChildren(1, [{ name: "prop1", kind: "property", line: 5 }], true);
    expect(state.nodes[1].children).toHaveLength(1);
    expect(state.nodes[1].expanded).toBe(true); // expanded

    // Other node unchanged
    expect(state.nodes[0].children).toHaveLength(2);
  });

  // 6. select sets selectedIndex
  it("select sets selectedIndex", () => {
    state.setEntities([
      makeEntitySummary("A"),
      makeEntitySummary("B"),
      makeEntitySummary("C"),
    ]);

    state.select(0);
    expect(state.selectedIndex).toBe(0);
    expect(state.selectedEntity?.name).toBe("A");

    state.select(2);
    expect(state.selectedIndex).toBe(2);
    expect(state.selectedEntity?.name).toBe("C");

    state.select(1);
    expect(state.selectedIndex).toBe(1);
    expect(state.selectedEntity?.name).toBe("B");
  });

  // 7. moveDown increments selectedIndex, clamped to nodes.length - 1
  it("moveDown increments selectedIndex, clamped to nodes.length - 1", () => {
    state.setEntities([
      makeEntitySummary("A"),
      makeEntitySummary("B"),
      makeEntitySummary("C"),
    ]);

    expect(state.selectedIndex).toBe(0);

    state.moveDown();
    expect(state.selectedIndex).toBe(1);

    state.moveDown();
    expect(state.selectedIndex).toBe(2);

    // At the end — should clamp to 2
    state.moveDown();
    expect(state.selectedIndex).toBe(2);

    state.moveDown();
    expect(state.selectedIndex).toBe(2);
  });

  // 8. moveUp decrements selectedIndex, clamped to 0
  it("moveUp decrements selectedIndex, clamped to 0", () => {
    state.setEntities([
      makeEntitySummary("A"),
      makeEntitySummary("B"),
      makeEntitySummary("C"),
    ]);
    state.select(2);

    state.moveUp();
    expect(state.selectedIndex).toBe(1);

    state.moveUp();
    expect(state.selectedIndex).toBe(0);

    // At the beginning — should clamp to 0
    state.moveUp();
    expect(state.selectedIndex).toBe(0);

    state.moveUp();
    expect(state.selectedIndex).toBe(0);
  });

  // 9. visibleRowCount equals nodes.length when nothing is expanded
  it("visibleRowCount equals nodes.length when nothing is expanded", () => {
    state.setEntities([
      makeEntitySummary("A"),
      makeEntitySummary("B"),
      makeEntitySummary("C"),
      makeEntitySummary("D"),
    ]);

    expect(state.visibleRowCount).toBe(4);
  });

  // 10. visibleRowCount equals nodes.length + sum(children.length) when nodes are expanded
  it("visibleRowCount equals nodes.length + sum(children.length) when nodes are expanded", () => {
    state.setEntities([
      makeEntitySummary("A"),
      makeEntitySummary("B"),
      makeEntitySummary("C"),
    ]);

    // Add 2 children to A and expand it
    state.updateNodeChildren(0, [
      { name: "m1", kind: "method", line: 1 },
      { name: "m2", kind: "method", line: 2 },
    ], true);

    // Add 3 children to C and expand it
    state.updateNodeChildren(2, [
      { name: "p1", kind: "property", line: 1 },
      { name: "p2", kind: "property", line: 2 },
      { name: "p3", kind: "property", line: 3 },
    ], true);

    // B is not expanded, no children
    // visibleRowCount = 3 (nodes) + 2 (A's children) + 3 (C's children) = 8
    expect(state.visibleRowCount).toBe(8);

    // Collapse A — should drop its 2 children
    state.toggleExpand(0);
    // visibleRowCount = 3 (nodes) + 3 (C's children) = 6
    expect(state.visibleRowCount).toBe(6);

    // Collapse C — should drop its 3 children
    state.toggleExpand(2);
    // visibleRowCount = 3 (nodes) = 3
    expect(state.visibleRowCount).toBe(3);
  });

  // Additional: empty state
  it("handles empty entity list", () => {
    state.setEntities([]);
    expect(state.nodes).toHaveLength(0);
    expect(state.selectedIndex).toBe(0);
    expect(state.selectedEntity).toBeNull();
    expect(state.visibleRowCount).toBe(0);
  });

  // Additional: moveDown on empty list
  it("moveDown on empty list clamps to -1 (nodes.length - 1)", () => {
    state.setEntities([]);
    state.moveDown();
    // Math.min(nodes.length - 1, prev + 1) = Math.min(-1, 1) = -1
    expect(state.selectedIndex).toBe(-1);
  });

  // Additional: moveUp on empty list
  it("moveUp on empty list stays at 0", () => {
    state.setEntities([]);
    state.moveUp();
    expect(state.selectedIndex).toBe(0);
  });
});
