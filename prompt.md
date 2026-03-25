## Step 1: Read these files first (in this order)

1. CLAUDE.md - project summary, architecture, conventions
2. specs/readme.md - spec index
3. specs/manifest-hardening.md - the manifest hardening spec (sections 1-8). Find the first phase (section 6) with unchecked `- [ ]` tasks

## Step 2: Read the code you'll need for the current phase

Phase A (P0 Fixes — Data Loss Prevention):
- crates/domain-scan-core/src/manifest.rs - manifest parsing, matching, write-back
- crates/domain-scan-cli/src/main.rs - cmd_match() write-back path
- crates/domain-scan-core/src/ir.rs - IR types (MatchResult, ManifestSubsystem, SystemManifest, etc.)

Phase B (P1 Fixes — Correct Behavior):
- crates/domain-scan-core/src/manifest.rs - write_back_to_subsystem() status logic
- crates/domain-scan-core/src/manifest_builder.rs - bootstrap, group_files_by_directory(), infer_subsystems()
- crates/domain-scan-cli/src/main.rs - Cli struct, positional args
- crates/domain-scan-tauri/ui/src/components/ManifestLoader.tsx - agent prompt template
- skills/domain-scan-init.md - bootstrap guidance

Phase C (P2 Polish):
- crates/domain-scan-core/src/schema.rs - schema_for_command(), all_command_names()
- Cargo.toml - workspace version
- skills/domain-scan-match.md - write-back documentation

Phase D (P3 Features):
- crates/domain-scan-core/src/manifest.rs - find_match(), glob matching
- crates/domain-scan-tauri/ui/src/components/ManifestLoader.tsx - prompt schema children example
- skills/domain-scan-init.md - nested subsystem guidance

## Step 3: Pick the most important unchecked task and implement it

CRITICAL: Complete phases sequentially (Phase A -> B -> C -> D). Do NOT skip ahead to a later phase while earlier phases have unchecked tasks.

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
7. **Update the spec**: check off `- [x]` for the completed task in `specs/manifest-hardening.md`
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
