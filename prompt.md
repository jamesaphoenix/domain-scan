## Step 1: Read these files first (in this order)

1. CLAUDE.md - project summary, architecture, conventions
2. specs/readme.md - spec index
3. specs/tube-map.md - the Subsystem Tube Map spec (sections 1-11). Find the first phase (section 9) with unchecked `- [ ]` tasks

## Step 2: Read the code you'll need for the current phase

Phase A (Foundation — Bug Fix + Tab Shell + IPC Commands):
- crates/domain-scan-tauri/tauri.conf.json - Tauri config, CSP, window settings
- crates/domain-scan-tauri/src/commands.rs - existing IPC commands, AppState
- crates/domain-scan-tauri/src/lib.rs - Tauri builder, plugin registration
- crates/domain-scan-tauri/ui/src/App.tsx - current layout, Open Directory button
- crates/domain-scan-tauri/ui/src/hooks/useScan.ts - existing Tauri IPC hook
- crates/domain-scan-tauri/ui/src/types.ts - TypeScript type definitions
- crates/domain-scan-core/src/manifest.rs - manifest parsing, matching
- crates/domain-scan-core/src/ir.rs - IR types (MatchResult, ManifestSubsystem, etc.)

Phase B (Layout Engine):
- ui/src/layout/types.ts - (create) ComputedLine, LayoutGrid, DomainLayer
- ui/src/layout/colors.ts - (create) domain color assignment
- ui/src/layout/tubeMap.ts - (create) dynamic layout algorithm
- Reference: /Users/jamesaphoenix/Desktop/projects/just-understanding-data/octospark-visualizer/src/layout.ts

Phase C (React Flow Canvas):
- crates/domain-scan-tauri/ui/package.json - add @xyflow/react dependency
- ui/src/components/TubeMapView.tsx - (create) ReactFlow container
- ui/src/components/SubsystemNode.tsx - (create, port from octospark)
- ui/src/components/DependencyEdge.tsx - (create, port from octospark)
- ui/src/hooks/useTubeMapState.ts - (create) manifest + match state
- ui/src/hooks/useTubeLayout.ts - (create) memoized layout
- Reference: /Users/jamesaphoenix/Desktop/projects/just-understanding-data/octospark-visualizer/src/App.tsx
- Reference: /Users/jamesaphoenix/Desktop/projects/just-understanding-data/octospark-visualizer/src/components/SubsystemNode.tsx
- Reference: /Users/jamesaphoenix/Desktop/projects/just-understanding-data/octospark-visualizer/src/components/DependencyEdge.tsx

Phase D (Interaction — Search, Filter, Trace, Drill-In):
- ui/src/components/TubeMapSearchBar.tsx - (create, port from octospark SearchBar)
- ui/src/components/Legend.tsx - (create, port from octospark)
- ui/src/components/Breadcrumbs.tsx - (create, port from octospark)
- ui/src/components/SubsystemDrillIn.tsx - (create) drill-in entity cards
- ui/src/components/CoverageOverlay.tsx - (create) match coverage display
- Reference: /Users/jamesaphoenix/Desktop/projects/just-understanding-data/octospark-visualizer/src/components/

Phase E (Polish):
- Edge bundling, tube line stripes, animations, performance
- Snapshot tests for layout algorithm

Phase F (Hardening — E2E Tests, Bug Hunting, Edge Cases):
- crates/domain-scan-tauri/ui/e2e/ - Playwright E2E test suite
- crates/domain-scan-tauri/ui/e2e/fixtures/ - test manifests (minimal, large, circular, malformed, etc.)
- Test all flows: Open Directory, tab switching, manifest loading, tube map interactions, keyboard shortcuts
- Stress tests: 200 subsystems, circular deps, orphan domains, rapid tab switching
- Data integrity: entity counts match, coverage % consistent, no cross-tab state corruption

Phase G.1 (Core Prompt Generation):
- crates/domain-scan-core/src/manifest_builder.rs - prompt generation + response parsing

Phase G.2 (Smart Defaults):
- crates/domain-scan-core/src/manifest_builder.rs - heuristic domain/subsystem/connection inference

Phase G.3 (CLI `domain-scan init`):
- crates/domain-scan-cli/src/main.rs - init subcommand with --bootstrap, --apply-manifest, --dry-run

Phase G.4 (Tauri Wizard UI):
- crates/domain-scan-tauri/ui/src/components/ManifestWizard.tsx + wizard step components

Phase G.5 (Agent Skill Files + Bootstrapping):
- skills/domain-scan-init.md - manifest building/refining skill with patch guidelines
- skills/domain-scan-tube-map.md - tube map interaction skill
- domain-scan skills install --claude-code → .claude/skills/ in project root

## Step 3: Pick the most important unchecked task and implement it

CRITICAL: Complete phases sequentially (Phase A -> B -> C -> D -> E -> F -> G.1 -> G.2 -> G.3 -> G.4 -> G.5). Do NOT skip ahead to a later phase while earlier phases have unchecked tasks.

## Housekeeping

- Before starting work, check if `target/` is over 5GB: `du -sh target/ 2>/dev/null`. If over 5GB, run `cargo clean` to prune build artifacts before proceeding.

## Workflow (MUST follow this order for every task)

1. **Write the code** for the task
2. **Compile/Build**:
   - Rust: `cargo build -p domain-scan-core` (or `domain-scan-tauri`). Fix all compiler errors.
   - Frontend: `cd crates/domain-scan-tauri/ui && npx tsc --noEmit`. Fix all type errors.
3. **Lint**:
   - Rust: `cargo clippy -p domain-scan-core -- -D warnings`. Fix all warnings.
4. **Write the tests** for the code you just wrote
5. **Run the tests**:
   - Rust: `cargo test -p domain-scan-core`. Fix any failures.
   - Frontend: TypeScript type-check is sufficient (no Jest/Vitest setup yet).
6. **All tests pass**: verify zero failures before committing.
7. **Update the spec**: check off `- [x]` for the completed task in `specs/tube-map.md`
8. **Commit and push**: `git add -A && git commit -m "<descriptive message>" && git push`

Do NOT skip steps. Do NOT commit code that doesn't compile. Do NOT commit tests that don't pass.

## Rules

- Use shared test fixtures: real code snippets in `tests/fixtures/<lang>/` with expected JSON output in `tests/fixtures/<lang>/expected/`
- Write integration tests that exercise real tree-sitter parsing (not mocks)
- Use rstest for parameterized tests, proptest for IR roundtrip invariants (NOT for generating source code)
- Use insta for snapshot tests on CLI output and prompt generation
- For tree-sitter query development: write the .scm file, then immediately write the integration test against a fixture. Do not write queries without tests.
- Every .scm query file must have at least one integration test that parses a real code fixture through tree-sitter and asserts the IR output
- No business logic in Tauri IPC or MCP layers. Both are thin wrappers over domain-scan-core.
