# manifest-hardening -- fix 13 bugs found during real-world agent dogfood run

> Harden the manifest matching, write-back, and bootstrap pipelines so that AI agents can generate accurate `system.json` manifests without data loss, path mismatches, or silent failures. Every fix has a regression test.

---

## 1. Context

### 1.1 The Dogfood Run

An AI agent (Claude Code) was given `domain-scan` and asked to produce a `system.json` manifest for a real 2,790-file TypeScript/React monorepo (`apps/*`, `packages/*` layout). The agent followed the skill files and prompt template, ran `domain-scan init --bootstrap`, iterated with `domain-scan match --manifest system.json --write-back`, and attempted to reach 90%+ coverage.

The run ultimately achieved 99.7% coverage — but only after the agent worked around 13 bugs using Python scripts, manual manifest reconstruction, and absolute-path rewrites. These workarounds consumed ~40% of the agent's context window and would be invisible to a less capable agent.

### 1.2 Issue Summary

| Priority | Count | Impact |
|----------|-------|--------|
| P0 | 3 | Data loss, 0% coverage, wrong match results |
| P1 | 4 | Status corruption, empty bootstrap, syntax errors, contradictory docs |
| P2 | 4 | Missing schema, version mismatch, sudo required, no warnings |
| P3 | 2 | No glob matching, no children subsystem guidance |

### 1.3 Design Principles

1. **Write-back must be lossless.** Any field present in the input manifest must survive a round-trip through `--write-back`.
2. **Path matching must be normalized.** Relative and absolute `filePath` values must produce identical results when resolved against the scan root.
3. **Specificity is path depth, not tree depth.** `packages/platform/test-utils/` (4 components) is more specific than `packages/platform/` (2 components), regardless of manifest hierarchy.
4. **Status is human-controlled.** Only the user upgrades a subsystem to `built`. The tool never auto-promotes.
5. **Bootstrap must work for monorepos.** A 2,790-file repo with `apps/*` and `packages/*` must produce a non-empty manifest.

### 1.4 Files Modified

| File | Bugs |
|------|------|
| `crates/domain-scan-core/src/manifest.rs` | 1, 2, 3, 4, F1 |
| `crates/domain-scan-cli/src/main.rs` | 1, 6 |
| `crates/domain-scan-core/src/manifest_builder.rs` | 5, F2 |
| `crates/domain-scan-core/src/schema.rs` | 8 |
| `crates/domain-scan-tauri/ui/src/components/ManifestLoader.tsx` | 7, 10, F2 |
| `skills/domain-scan-init.md` | 7, F2 |
| `skills/domain-scan-match.md` | 11 |
| `Cargo.toml` | 9 |

---

## 2. P0 — Data Loss / Broken Core

### 2.1 Bug 1: `--write-back` drops meta, domains, and connections

**Symptom:** After `domain-scan match --manifest system.json --write-back`, the output file contains only `{"subsystems": [...]}`. The `meta`, `domains`, and `connections` sections are silently deleted.

**Root cause:** `cmd_match()` (`main.rs:2223`) calls `parse_manifest_file()` which deserializes into `Manifest` — a struct with only `subsystems` (manifest.rs:28-30). After write-back, `serialize_manifest()` serializes this `Manifest` which lacks the other three fields. The full `SystemManifest` struct (manifest.rs:32-41) that includes `meta`, `domains`, and `connections` is never loaded.

```
SystemManifest { meta, domains, subsystems, connections }
         ↓ parse_manifest_file() → deserializes as:
Manifest { subsystems }                                   ← meta/domains/connections lost
         ↓ serialize_manifest()
{"subsystems": [...]}                                     ← written to disk
```

**Fix:**

1. Add `serialize_system_manifest()` and `write_back_system()` to `manifest.rs`:

```rust
/// Serialize a full SystemManifest back to pretty-printed JSON.
pub fn serialize_system_manifest(manifest: &SystemManifest) -> Result<String, DomainScanError> {
    serde_json::to_string_pretty(manifest).map_err(DomainScanError::Serialization)
}

/// Write-back into a full SystemManifest. Preserves meta, domains, connections.
pub fn write_back_system(
    manifest: &mut SystemManifest,
    match_result: &MatchResult,
    index: &ScanIndex,
) {
    for m in &match_result.matched {
        write_back_to_subsystem(&mut manifest.subsystems, m, index);
    }
}
```

2. Rewrite the write-back path in `cmd_match()` (`main.rs:2227-2265`):

```rust
if write_back {
    // Try SystemManifest first (preserves meta/domains/connections)
    let serialized = if let Ok(mut sys_manifest) = manifest::parse_system_manifest_file(&manifest_path) {
        manifest::write_back_system(&mut sys_manifest, &result, &scan_index);
        manifest::serialize_system_manifest(&sys_manifest)?
    } else {
        // Fallback: plain Manifest (subsystems only)
        let mut updated = manifest_data.clone();
        manifest::write_back(&mut updated, &result, &scan_index);
        manifest::serialize_manifest(&updated)?
    };
    // ... rest of dry-run / write logic unchanged
}
```

**Acceptance criteria:**
- [ ] `--write-back` on a file with `meta`, `domains`, `connections`, `subsystems` preserves all four sections
- [ ] Round-trip test: parse `SystemManifest`, write-back, re-parse — `meta == original.meta`, `domains == original.domains`, `connections == original.connections`
- [ ] Dry-run preview includes the full `SystemManifest`, not just subsystems
- [ ] Fallback: a manifest with only `subsystems` (no `meta`) still works via the `Manifest` path

### 2.2 Bug 2: Relative filePaths produce 0% coverage

**Symptom:** A manifest with `"filePath": "packages/auth/"` produces 0% coverage when scanned entities have absolute paths like `/Users/james/project/packages/auth/handler.ts`.

**Root cause:** `find_match()` (manifest.rs:275) checks `entity.file.starts_with(&sub.file_path)`. An absolute path never starts with a relative prefix.

**Fix:** Pass `scan_root` (from `ScanIndex.root`) into `match_entities()` and `find_match()`. Resolve relative `file_path` values before the `starts_with` check:

```rust
pub fn match_entities(index: &ScanIndex, manifest: &Manifest) -> MatchResult {
    let flat = flatten_manifest(manifest);
    let scan_root = &index.root;  // already stores the absolute scan root
    // ... pass scan_root to find_match()
}

fn find_match(
    entity: &EntitySummary,
    flat: &[FlatSubsystem],
    scan_root: &Path,
) -> Option<(String, String, MatchStrategy)> {
    for sub in flat {
        let resolved = if sub.file_path.is_relative() {
            scan_root.join(&sub.file_path)
        } else {
            sub.file_path.clone()
        };
        if entity.file.starts_with(&resolved) {
            // ... specificity check (see Bug 3)
        }
    }
}
```

Normalization happens at match-time, never modifying the manifest file.

**Acceptance criteria:**
- [ ] Manifest with `filePath: "packages/auth/"` matches entities at `/abs/project/packages/auth/handler.ts` when scan root is `/abs/project/`
- [ ] Absolute `filePath` values still work as before (no regression)
- [ ] Mixed relative and absolute paths in the same manifest both match correctly
- [ ] Coverage > 0% for all-relative-path manifest scanned from the correct root

### 2.3 Bug 3: Deepest-match uses tree depth, not path specificity

**Symptom:** Two sibling subsystems — `platform-core` (`filePath: "packages/platform/"`) and `test-utils` (`filePath: "packages/platform/test-utils/"`) — both have `depth=0` because they're siblings at the top level. All entities under `packages/platform/test-utils/` match `platform-core` (which appears first in the array) instead of `test-utils`.

**Root cause:** `find_match()` (manifest.rs:276-280) uses `sub.depth` to determine the "deepest" match. `depth` comes from `flatten_recursive` and represents hierarchy nesting (parent → child), not path specificity. Two sibling subsystems at the same manifest level both have the same `depth`, so the first in iteration order wins.

Ironically, `manifest_builder.rs:439` already uses the correct approach: `sub.file_path.components().count()` as the specificity metric in `find_subsystem_for_path()`.

**Fix:** Replace `sub.depth` with `resolved_path.components().count()` in `find_match()`:

```rust
fn find_match(
    entity: &EntitySummary,
    flat: &[FlatSubsystem],
    scan_root: &Path,
) -> Option<(String, String, MatchStrategy)> {
    let mut best_match: Option<(&FlatSubsystem, usize)> = None;
    for sub in flat {
        let resolved = if sub.file_path.is_relative() {
            scan_root.join(&sub.file_path)
        } else {
            sub.file_path.clone()
        };
        if entity.file.starts_with(&resolved) {
            let specificity = resolved.components().count();
            let is_more_specific = best_match
                .as_ref()
                .is_none_or(|(_, s)| specificity > *s);
            if is_more_specific {
                best_match = Some((sub, specificity));
            }
        }
    }
    if let Some((sub, _)) = best_match {
        return Some((sub.id.clone(), sub.name.clone(), MatchStrategy::FilePath));
    }
    // ... name matching fallback unchanged
}
```

The `depth` field on `FlatSubsystem` can remain for informational use but is no longer the match discriminator.

**Acceptance criteria:**
- [ ] Two sibling subsystems: `packages/platform/` and `packages/platform/test-utils/` — entities under `test-utils/` match to `test-utils`, not `platform-core`
- [ ] Parent-child hierarchy still works: entity at `src/auth/jwt/token.ts` matches `auth-jwt` (child) over `auth` (parent)
- [ ] Existing `test_match_by_file_path` passes (tests child vs parent via hierarchy — works with both `depth` and component count)

---

## 3. P1 — Incorrect Behavior

### 3.1 Bug 4: `--write-back` auto-upgrades status to "built"

**Symptom:** Subsystems with `status: "new"` become `status: "built"` after `--write-back`, even though the skill file says "only the user can upgrade to built."

**Root cause:** `write_back_to_subsystem()` (manifest.rs:506-517) checks if any entity has `BuildStatus::Built` and if any file under the subsystem is Built, then upgrades `New`/`Boilerplate` to `Built`. This directly contradicts `domain-scan-init.md`: "Never auto-confirm `built` status."

**Fix:** Remove the auto-upgrade block entirely (manifest.rs:506-517):

```rust
// DELETE: lines 506-517 of write_back_to_subsystem()
// The status upgrade block that checks matched.entity.build_status == BuildStatus::Built
// and upgrades sub.status from New/Boilerplate to Built.
```

If a future `--upgrade-status` flag is desired, implement it as a separate opt-in flag, not implicit write-back behavior.

**Acceptance criteria:**
- [ ] Write-back on `status: "new"` subsystem with all-Built entities leaves status as `"new"`
- [ ] Write-back on `status: "boilerplate"` leaves status as `"boilerplate"`
- [ ] Write-back on `status: "built"` leaves status unchanged (no downgrade)
- [ ] No `ManifestStatus` mutation anywhere in the write-back code path

### 3.2 Bug 5: `--bootstrap` produces empty output for monorepos

**Symptom:** `domain-scan init --bootstrap` on a 2,790-file monorepo with `apps/*` and `packages/*/src/*` layout produces `{"subsystems": [], "domains": {}, "connections": []}`.

**Root cause:** `group_files_by_directory()` (manifest_builder.rs:146-183) assigns subsystem names from the third path component. For `packages/platform/src/auth/handler.ts`, it creates `domain="platform"`, `subsystem="src"` (line 171-175). All files across all sub-packages are grouped under subsystem `"src"`, producing either one giant meaningless group or many groups each below `min_entities=3`.

**Fix (three changes):**

**Change 1:** Lower `BootstrapOptions::default().min_entities` from 3 to 1 (manifest_builder.rs:36). A subsystem with 1-2 entities is better than zero subsystems. Agents merge small ones during refinement.

**Change 2:** Improve `group_files_by_directory()` to skip `src`/`lib`/`app` intermediary dirs for workspace packages:

```rust
// At manifest_builder.rs line 165-176, when is_workspace_dir(first):
if is_workspace_dir(first) || first == "src" || first == "lib" || first == "app" {
    let domain = components[1].to_string();
    // Skip "src"/"lib"/"app" if it's the third component
    let subsys = if components.len() > 3 {
        let third = components[2];
        if matches!(third, "src" | "lib" | "app") && components.len() > 4 {
            components[3].to_string()
        } else {
            third.to_string()
        }
    } else {
        components[1].to_string()
    };
    (domain, subsys)
}
```

**Change 3:** Add a fallback in `infer_subsystems()` — if zero subsystems pass the threshold, create one per domain group:

```rust
fn infer_subsystems(groups: &DirGroups, root: &Path, min_entities: usize) -> Vec<ManifestSubsystem> {
    let mut subsystems = Vec::new();
    // ... existing per-subsystem logic ...

    // Fallback: if no subsystems passed threshold, create one per domain
    if subsystems.is_empty() {
        for (domain, subsys_map) in groups {
            let all_files: Vec<PathBuf> = subsys_map.values().flatten().cloned().collect();
            if !all_files.is_empty() {
                let file_path = compute_common_prefix(&all_files, root);
                subsystems.push(ManifestSubsystem {
                    id: domain.clone(),
                    name: humanize_name(domain),
                    domain: domain.clone(),
                    status: ManifestStatus::New,
                    file_path,
                    interfaces: Vec::new(),
                    operations: Vec::new(),
                    tables: Vec::new(),
                    events: Vec::new(),
                    children: Vec::new(),
                    dependencies: Vec::new(),
                });
            }
        }
    }

    subsystems
}
```

**Acceptance criteria:**
- [ ] `--bootstrap` on a repo with `packages/a/src/...`, `packages/b/src/...`, `apps/web/src/...` produces at least one subsystem per package
- [ ] `BootstrapOptions::default().min_entities == 1`
- [ ] Grouping skips `src`/`lib`/`app` intermediary dirs for workspace packages
- [ ] Fallback creates one subsystem per domain when all groups are too small
- [ ] Existing `test_bootstrap_on_own_codebase` (if present) still passes

### 3.3 Bug 6: `scan .` positional syntax not accepted

**Symptom:** `domain-scan scan .` fails with "unexpected argument '.' found". Must use `domain-scan scan --root .`.

**Root cause:** `--root` (main.rs:37-39) is a named flag with `#[arg(long, global = true, default_value = ".")]`. No positional argument is defined on the `Cli` struct for the root path.

**Fix:** Add an optional trailing positional argument to the `Cli` struct:

```rust
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Root directory to scan
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    /// Root directory (positional alternative to --root)
    #[arg(global = true, value_name = "PATH")]
    path: Option<PathBuf>,
}
```

Then merge at resolution time (before `run_scan`):

```rust
let effective_root = cli.path.clone().unwrap_or_else(|| cli.root.clone());
```

**Acceptance criteria:**
- [ ] `domain-scan scan .` works (equivalent to `domain-scan scan --root .`)
- [ ] `domain-scan scan /path/to/project` works
- [ ] `domain-scan scan` still defaults to `.`
- [ ] `domain-scan scan --root /path` still works
- [ ] Conflict between positional and `--root` produces a clear error

### 3.4 Bug 7: Prompt contradicts skills on bootstrap usage

**Symptom:** The agent prompt template (ManifestLoader.tsx) says "Do NOT use `domain-scan init --bootstrap`" while `domain-scan-init.md` says "Always start with `--bootstrap`." Agents receive contradictory instructions.

**Fix:** Once Bug 5 is resolved (bootstrap works for monorepos), align both to recommend bootstrap:
- Change ManifestLoader.tsx to recommend `--bootstrap` as Step 1
- Remove manual-from-scratch instructions from the prompt
- Keep `domain-scan-init.md` as-is (already recommends bootstrap)

**Acceptance criteria:**
- [ ] Prompt template and skill file both recommend `--bootstrap` as the first step
- [ ] No contradictions between prompt and skill files on bootstrap/init guidance

---

## 4. P2 — Ergonomic / Polish

### 4.1 Bug 8: `schema init` not registered

**Symptom:** `domain-scan schema init` returns `"Unknown command: 'init'"`.

**Root cause:** `all_command_names()` (schema.rs:180-194) lists every command except `"init"`. `schema_for_command()` has no `"init"` match arm.

**Fix:** Add `"init"` to `all_command_names()` and implement `init_schema()`:

```rust
"init" => Some(init_schema()),
```

Where `init_schema()` returns `BootstrapOptions` as input and `SystemManifest` as output.

**Acceptance criteria:**
- [x] `domain-scan schema init` returns valid JSON Schema
- [x] `domain-scan schema --all` includes `init`

### 4.2 Bug 9: Version mismatch (binary reports 0.1.0, release is v0.3.0)

**Symptom:** `domain-scan --version` outputs `0.1.0` but the GitHub release is tagged `v0.3.0`.

**Root cause:** Workspace `Cargo.toml` line 11 has `version = "0.1.0"`.

**Fix:** Bump `[workspace.package] version` to `"0.3.0"` (or the current release tag). Add a CI check that validates the Cargo.toml version matches the git tag on release.

**Acceptance criteria:**
- [x] `domain-scan --version` matches the release tag
- [x] All crate `Cargo.toml` files inherit the workspace version

### 4.3 Bug 10: Install path requires sudo

**Symptom:** Prompt suggests `sudo mv /tmp/domain-scan /usr/local/bin/domain-scan`. Fails in sandboxed environments (Claude Code sandbox, CI runners, restrictive macOS).

**Fix:** Change install instructions to `~/.local/bin/`:

```bash
mkdir -p ~/.local/bin
mv /tmp/domain-scan ~/.local/bin/domain-scan
export PATH="$HOME/.local/bin:$PATH"
```

**Acceptance criteria:**
- [x] Install instructions do not use `sudo`
- [x] Default install location is `~/.local/bin/`
- [x] PATH export hint included

### 4.4 Bug 11: No warning about write-back data loss

**Symptom:** Agent prompt and skill files give no warning that `--write-back` could lose data. Becomes moot after Bug 1 is fixed, but defense-in-depth is warranted.

**Fix:** Add a format-detection note to `skills/domain-scan-match.md`:

```markdown
## Write-back format preservation

`--write-back` auto-detects whether the manifest is a full `SystemManifest` (with
meta/domains/connections) or a plain `Manifest` (subsystems only). It preserves all
existing fields. Requires domain-scan >= 0.3.0.
```

**Acceptance criteria:**
- [x] Skill file documents write-back format preservation

---

## 5. P3 — Feature Gaps

### 5.1 Feature 1: Glob/file-level matching in filePath

**Current:** `filePath` is directory-prefix only. Cannot match `src/services/scheduling*` or `packages/*/src/`.

**Design:** Detect glob metacharacters (`*`, `?`, `[`, `{`) in `filePath`. If present, use `globset::GlobMatcher` (already a dependency) instead of `starts_with`:

```rust
fn is_glob_pattern(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}

// In find_match():
let matches = if is_glob_pattern(&resolved) {
    globset::Glob::new(&resolved.to_string_lossy())
        .ok()
        .map(|g| g.compile_matcher())
        .map_or(false, |m| m.is_match(&entity.file))
} else {
    entity.file.starts_with(&resolved)
};
```

**Acceptance criteria:**
- [ ] `filePath: "packages/platform/src/services/scheduling*"` matches `scheduling.ts` and `schedulingUtils.ts`
- [ ] `filePath: "packages/*/src/"` matches all packages
- [ ] Non-glob paths work as before (no regression)
- [ ] Invalid glob patterns produce a `DomainScanError`, not a panic

### 5.2 Feature 2: Children/nested subsystem guidance for agents

**Current:** The `children` field exists on `ManifestSubsystem` (manifest.rs:108) and is fully supported by the matching engine (`flatten_recursive` walks children). However, agents never populate it because neither bootstrap nor the agent prompt teaches them to.

**Design (documentation + optional bootstrap enhancement):**

1. Add a "Nested subsystems" section to `domain-scan-init.md`:
   - When to use: a single package has distinct sub-modules (e.g., `auth`, `billing`, `notifications` under `packages/platform/`)
   - Format: same structure as top-level, nested in `children` array
   - Matching: children inherit parent's domain, override filePath with a more specific path

2. Optionally: in `infer_subsystems()`, detect when 3+ subsystems share a common parent directory and auto-nest them.

3. Add `children` example to the ManifestLoader.tsx prompt schema.

**Acceptance criteria:**
- [ ] Skill file documents when and how to use `children`
- [ ] Agent prompt schema example includes a `children` example
- [ ] Existing children matching continues to work

---

## 6. Build Phases

### Phase A: P0 Fixes — Data Loss Prevention

Focus: `manifest.rs` and `main.rs`.

- [x] A.1 Add `serialize_system_manifest()` to `manifest.rs`
- [x] A.2 Add `write_back_system()` to `manifest.rs`
- [x] A.3 Rewrite `cmd_match()` write-back path to try `SystemManifest` first, fall back to `Manifest`
- [x] A.4 Add `scan_root: &Path` parameter to `match_entities()` and `find_match()`
- [x] A.5 Resolve relative `file_path` against `scan_root` in `find_match()`
- [x] A.6 Replace `sub.depth` with `resolved_path.components().count()` in `find_match()`
- [x] A.7 Test: SystemManifest write-back round-trip preserves meta/domains/connections
- [x] A.8 Test: relative filePaths match against absolute entity paths
- [x] A.9 Test: sibling subsystems resolve by path specificity, not array order
- [x] A.10 Integration test: `--write-back --dry-run` on a full system.json shows all four sections

### Phase B: P1 Fixes — Correct Behavior

Focus: `manifest.rs`, `manifest_builder.rs`, `main.rs`, skill files.

- [x] B.1 Remove status auto-upgrade block from `write_back_to_subsystem()` (manifest.rs:506-517)
- [x] B.2 Lower `BootstrapOptions::default().min_entities` from 3 to 1
- [x] B.3 Improve `group_files_by_directory()` to skip `src`/`lib`/`app` intermediary dirs
- [x] B.4 Add fallback in `infer_subsystems()`: one subsystem per domain if all groups too small
- [x] B.5 Add optional positional `[PATH]` argument to `Cli` struct, merge with `--root`
- [x] B.6 Align ManifestLoader.tsx prompt to recommend `--bootstrap`
- [x] B.7 Test: write-back does not change subsystem status
- [x] B.8 Test: bootstrap on monorepo fixture produces non-empty subsystems
- [x] B.9 Test: `scan .` positional argument accepted

### Phase C: P2 Polish

Focus: `schema.rs`, `Cargo.toml`, skill files.

- [x] C.1 Add `init_schema()` to `schema.rs`, register in `schema_for_command()` and `all_command_names()`
- [x] C.2 Bump `[workspace.package] version` to match release tag
- [x] C.3 Change install instructions from `sudo /usr/local/bin` to `~/.local/bin`
- [x] C.4 Add write-back format note to `domain-scan-match.md` skill file
- [x] C.5 Test: `domain-scan schema init` returns valid JSON

### Phase D: P3 Features

Focus: `manifest.rs`, skill files.

- [ ] D.1 Add `is_glob_pattern()` to `manifest.rs`
- [ ] D.2 Add glob matching branch in `find_match()` using `globset::GlobMatcher`
- [ ] D.3 Add nested subsystem section to `domain-scan-init.md`
- [ ] D.4 Update ManifestLoader.tsx prompt schema example to include `children`
- [ ] D.5 Test: glob filePath matches individual files
- [ ] D.6 Test: glob filePath matches wildcard patterns
- [ ] D.7 Test: invalid glob produces structured error

---

## 7. Testing Strategy

### 7.1 Unit Tests — `manifest.rs`

| Test | Bug | Asserts |
|------|-----|---------|
| `test_system_manifest_write_back_roundtrip` | 1 | Parse SystemManifest → write-back → serialize → re-parse. `meta == original.meta`, `domains == original.domains`, `connections == original.connections` |
| `test_relative_filepath_matching` | 2 | Manifest with relative `filePath`, entities with absolute paths. Coverage > 0% |
| `test_mixed_relative_absolute_paths` | 2 | Mix in same manifest. All match correctly |
| `test_path_specificity_sibling_subsystems` | 3 | Siblings: `packages/platform/` and `packages/platform/test-utils/`. Entity under test-utils matches test-utils |
| `test_path_specificity_parent_child` | 3 | Existing `test_match_by_file_path` still passes (parent/child hierarchy) |
| `test_write_back_no_status_upgrade` | 4 | Write-back on `status: "new"` with Built entities. Status stays `"new"` |
| `test_glob_filepath_matching` | F1 | `filePath: "src/services/scheduling*"` matches files |
| `test_glob_wildcard_matching` | F1 | `filePath: "packages/*/src/"` matches multi-package |
| `test_invalid_glob_error` | F1 | Invalid glob → `DomainScanError`, not panic |

### 7.2 Unit Tests — `manifest_builder.rs`

| Test | Bug | Asserts |
|------|-----|---------|
| `test_bootstrap_monorepo_layout` | 5 | Fixture with `packages/a/src/...`, `apps/web/src/...`. Non-empty subsystems |
| `test_bootstrap_min_entities_default` | 5 | `BootstrapOptions::default().min_entities == 1` |
| `test_bootstrap_fallback_to_domains` | 5 | All groups below threshold → one subsystem per domain |
| `test_bootstrap_skips_src_intermediary` | 5 | `packages/auth/src/handler.ts` → subsystem is not named "src" |

### 7.3 Unit Tests — `schema.rs`

| Test | Bug | Asserts |
|------|-----|---------|
| `test_schema_init_registered` | 8 | `schema_for_command("init")` returns `Some(...)` |
| `test_all_command_names_includes_init` | 8 | `all_command_names()` contains `"init"` |

### 7.4 Integration Tests

| Test | Bug | Description |
|------|-----|-------------|
| `test_write_back_preserves_system_manifest` | 1 | Bootstrap SystemManifest → write to temp file → match --write-back → re-read → assert all sections preserved |
| `test_bootstrap_then_match_monorepo` | 5 | Bootstrap on multi-package fixture → match → coverage > 0% |

### 7.5 Snapshot Tests (insta)

- `serialize_system_manifest()` on sample SystemManifest (Bug 1)
- Bootstrap output on monorepo-shaped fixture (Bug 5)

---

## 8. Verification

After all phases complete, re-run the dogfood scenario:

```bash
# 1. Bootstrap
domain-scan init --bootstrap --name "octospark-services" \
  --root /path/to/octospark-services --dry-run

# 2. Verify bootstrap produces non-empty output with subsystems

# 3. Apply and write
domain-scan init --bootstrap --name "octospark-services" \
  --root /path/to/octospark-services -o system.json

# 4. Match with relative paths
domain-scan match --manifest system.json --root /path/to/octospark-services \
  --fields coverage_percent
# → coverage should be > 0% (relative paths work)

# 5. Write-back
domain-scan match --manifest system.json --root /path/to/octospark-services \
  --write-back --dry-run
# → preview should include meta, domains, connections, subsystems

# 6. Apply write-back
domain-scan match --manifest system.json --root /path/to/octospark-services \
  --write-back

# 7. Verify no data loss
python3 -c "import json; d=json.load(open('system.json')); print(list(d.keys()))"
# → ['meta', 'domains', 'subsystems', 'connections']

# 8. Verify status preserved
python3 -c "
import json
d = json.load(open('system.json'))
statuses = set(s['status'] for s in d['subsystems'])
print(statuses)
# → should contain 'new', not auto-upgraded to 'built'
"

# 9. Final coverage
domain-scan match --manifest system.json --root /path/to/octospark-services \
  --fields coverage_percent
# → > 90%
```
