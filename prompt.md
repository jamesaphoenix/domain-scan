## Step 1: Read these files first (in this order)

1. CLAUDE.md - project summary, architecture, conventions
2. specs/readme.md - spec index
3. specs/domain-scan.md - the full spec (sections 1-16). Find the first phase (section 11) with unchecked `- [ ]` tasks

## Step 2: Read the code you'll need for the current phase

Phase 1 (Foundation):
- Cargo.toml - workspace setup
- crates/domain-scan-core/src/lib.rs - public API + clippy deny wall
- crates/domain-scan-core/src/walker.rs - filesystem traversal
- crates/domain-scan-core/src/lang.rs - language detection
- crates/domain-scan-core/src/parser.rs - tree-sitter parsing
- crates/domain-scan-core/src/ir.rs - intermediate representation types
- crates/domain-scan-core/src/build_status.rs - build status detection
- crates/domain-scan-core/src/output.rs - serialization skeleton
- crates/domain-scan-core/src/types.rs - public types

Phase 2 (Query Engine + TypeScript):
- crates/domain-scan-core/src/query_engine.rs - .scm loading/dispatch
- crates/domain-scan-core/queries/typescript/ - all .scm files
- crates/domain-scan-core/tests/fixtures/typescript/ - test fixtures
- crates/domain-scan-core/tests/integration/ - integration tests

Phase 3 (Rust + Go + Python):
- crates/domain-scan-core/queries/rust/ - all .scm files
- crates/domain-scan-core/queries/go/ - all .scm files (uses method_elem not method_spec)
- crates/domain-scan-core/queries/python/ - all .scm files
- crates/domain-scan-core/tests/fixtures/rust/, go/, python/

Phase 4a (JVM: Java, Kotlin, Scala):
- crates/domain-scan-core/queries/java/ - including schemas.scm (@Entity, records)
- crates/domain-scan-core/queries/kotlin/ - uses (identifier) not (type_identifier) for names
- crates/domain-scan-core/queries/scala/
- crates/domain-scan-core/tests/fixtures/java/, kotlin/, scala/

Phase 4b (Systems/Scripting: C#, Swift, C++, PHP, Ruby):
- crates/domain-scan-core/queries/<lang>/ - all remaining language dirs
- crates/domain-scan-core/tests/fixtures/<lang>/

Phase 5 (Cross-File Resolution + Index + Config + Cache):
- crates/domain-scan-core/src/config.rs - .domain-scan.toml parsing
- crates/domain-scan-core/src/cache.rs - content-addressed cache
- crates/domain-scan-core/src/resolver.rs - import/export tracking
- crates/domain-scan-core/src/index.rs - ScanIndex construction
- crates/domain-scan-core/src/manifest.rs - manifest parsing, match, validate, write-back
- crates/domain-scan-core/src/validate.rs - validation rules

Phase 6a (CLI Core):
- crates/domain-scan-cli/src/main.rs - clap subcommands
- crates/domain-scan-core/src/output.rs - JSON + table + compact formatting

Phase 6b (TUI Interactive Mode):
- crates/domain-scan-cli/src/tui.rs - ratatui TUI app
- TuiApp struct with handle_event + render (testable via TestBackend)

Phase 7 (LLM Prompt Generation):
- crates/domain-scan-core/src/output.rs - prompt template
- Partition strategy, build-status-aware instructions

Phase 8 (MCP Server):
- crates/domain-scan-mcp/src/main.rs - MCP stdio server using rmcp
- ServerState with tokio::sync::RwLock
- All 14+ tools delegating to domain-scan-core

Phase 9 (Tauri Backend):
- crates/domain-scan-tauri/src/main.rs - Tauri setup
- crates/domain-scan-tauri/src/commands.rs - IPC commands with AppState + CommandError
- crates/domain-scan-tauri/tauri.conf.json

Phase 10 (Tauri Frontend):
- crates/domain-scan-tauri/ui/src/App.tsx - three-panel layout
- crates/domain-scan-tauri/ui/src/components/ - EntityTree, SourcePreview, DetailsPanel
- crates/domain-scan-tauri/ui/src/hooks/ - useScan, useTreeState, useKeyboard

Phase 11 (Polish + Performance):
- Benchmarks, --verbose, README, self-test

## Step 3: Pick the most important unchecked task and implement it

CRITICAL: Complete phases sequentially (Phase 1 -> 2 -> 3 -> 4a -> 4b -> 5 -> 6a -> 6b -> 7 -> 8 -> 9 -> 10 -> 11). Do NOT skip ahead to a later phase while earlier phases have unchecked tasks.

## Housekeeping

- Before starting work, check if `target/` is over 5GB: `du -sh target/ 2>/dev/null`. If over 5GB, run `cargo clean` to prune build artifacts before proceeding.

## Workflow (MUST follow this order for every task)

1. **Write the code** for the task
2. **Compile**: run `cargo build -p domain-scan-core` (or the relevant crate). Fix all compiler errors before proceeding.
3. **Run clippy**: `cargo clippy -p domain-scan-core -- -D warnings`. Fix all warnings.
4. **Write the tests** for the code you just wrote
5. **Run the tests**: `cargo test -p domain-scan-core`. Fix any failures.
6. **All tests pass**: verify zero failures before committing.
7. **Update the spec**: check off `- [x]` for the completed task in `specs/domain-scan.md`
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
