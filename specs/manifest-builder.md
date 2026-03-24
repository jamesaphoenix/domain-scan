# Manifest Builder — LLM-Driven Subsystem Discovery

> Incrementally build a `system.json` manifest by showing an LLM the entity census and letting it propose subsystem groupings, then mapping interfaces/operations/tables/events into each group.

---

## 1. The Problem

The tube map requires a `system.json` manifest defining domains, subsystems, and connections. Today this must be hand-authored — hours of work for a new codebase. Nobody will do this.

The insight: **domain-scan already extracts everything an LLM needs to propose subsystems.** It knows every interface, service, class, method, schema, import, and implementation in the codebase. An LLM can look at that census and say "these 15 interfaces + 3 services belong to an Auth subsystem."

## 2. The Workflow

A 4-step conversational loop, each step producing a more complete manifest:

```
┌──────────────────────────────────────────────────────────┐
│ Step 1: SCAN                                             │
│ domain-scan scan --root ./my-project --output json       │
│ → ScanIndex with all entities                            │
└──────────────────┬───────────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────────┐
│ Step 2: PROPOSE DOMAINS                                  │
│ domain-scan init --root ./my-project                     │
│                                                          │
│ Shows LLM a summary:                                     │
│   "45 interfaces, 12 services, 89 classes across         │
│    src/auth/, src/billing/, src/media/, src/api/..."     │
│                                                          │
│ LLM proposes:                                            │
│   Domain 1: "Auth & Identity" (auth/, sessions/)         │
│   Domain 2: "Billing" (billing/, payments/)              │
│   Domain 3: "Media Pipeline" (media/, upload/, cdn/)     │
│   ...                                                    │
│                                                          │
│ User: approves, renames, merges, splits                  │
└──────────────────┬───────────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────────┐
│ Step 3: MAP ENTITIES                                     │
│ For each approved domain, LLM sees entities in those     │
│ directories and proposes subsystems:                     │
│                                                          │
│   Auth & Identity:                                       │
│     Subsystem "auth-jwt":                                │
│       interfaces: [AuthPrincipal, JWTClaims]             │
│       operations: [signToken(), verifyToken()]           │
│       tables: [users]                                    │
│     Subsystem "auth-sessions":                           │
│       interfaces: [SessionToken, RefreshToken]           │
│       operations: [createSession(), revokeSession()]     │
│       tables: [auth_sessions]                            │
│                                                          │
│ User: approves, moves entities between subsystems        │
└──────────────────┬───────────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────────┐
│ Step 4: INFER CONNECTIONS                                │
│ LLM sees the import graph between subsystems:            │
│   auth-sessions imports from auth-jwt → depends_on       │
│   billing imports from auth → depends_on                 │
│   publisher triggers notifications → triggers            │
│                                                          │
│ Proposes connection labels:                              │
│   billing → auth: "Validates payment tokens"             │
│                                                          │
│ User: approves, adjusts types, adds labels               │
└──────────────────┬───────────────────────────────────────┘
                   │
┌──────────────────▼───────────────────────────────────────┐
│ OUTPUT: system.json                                      │
│ Complete manifest ready for tube map rendering            │
└──────────────────────────────────────────────────────────┘
```

## 3. Two Interfaces

### 3.1 CLI: `domain-scan init`

Generates structured prompts at each step. The user runs them through an LLM (Claude, GPT, etc.) and pastes back the response. The CLI parses the response and builds the manifest incrementally.

```bash
# Step 1: Scan + generate domain proposal prompt
domain-scan init --root ./my-project

# Outputs a prompt to stdout. User sends it to an LLM.
# LLM responds with JSON: { domains: [...], subsystem_proposals: [...] }

# Step 2: Feed LLM response back
domain-scan init --root ./my-project --apply-proposals proposals.json

# Outputs a mapping prompt for each domain. User sends to LLM.
# LLM responds with entity mappings per subsystem.

# Step 3: Feed mappings back
domain-scan init --root ./my-project --apply-mappings mappings.json

# Outputs connection inference prompt. User sends to LLM.
# LLM responds with connections.

# Step 4: Finalize
domain-scan init --root ./my-project --apply-connections connections.json -o system.json

# Writes final system.json
```

### 3.2 Tauri App: Guided Wizard

Interactive wizard in the tube map tab when no manifest is loaded:

```
┌──────────────────────────────────────────────────────────┐
│ [Entities/Types]  [Subsystem Tube Map]                   │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  Build Your Subsystem Map                          │  │
│  │                                                    │  │
│  │  Step 1 of 4: Propose Domains          [●○○○]     │  │
│  │                                                    │  │
│  │  domain-scan found 45 interfaces, 12 services,    │  │
│  │  89 classes across these directories:              │  │
│  │                                                    │  │
│  │  src/auth/         (8 interfaces, 3 services)      │  │
│  │  src/billing/      (5 interfaces, 2 services)      │  │
│  │  src/media/        (12 interfaces, 4 services)     │  │
│  │  src/api/          (6 interfaces, 1 service)       │  │
│  │  ...                                               │  │
│  │                                                    │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │ Proposed Domains:                            │  │  │
│  │  │                                              │  │  │
│  │  │ ● Auth & Identity    [auth/, sessions/]      │  │  │
│  │  │ ● Billing            [billing/, payments/]   │  │  │
│  │  │ ● Media Pipeline     [media/, upload/]       │  │  │
│  │  │ ● API Gateway        [api/]                  │  │  │
│  │  │                                              │  │  │
│  │  │ [Edit] [Merge] [Split] [Add Domain]          │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  │                                                    │  │
│  │  [← Back]                    [Approve & Continue →] │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

Each step shows the LLM's proposals. The user edits/approves. On "Continue", the next prompt is generated and sent to the LLM.

---

## 4. Prompt Design

### 4.1 Step 1: Domain Proposal Prompt

```markdown
# Codebase Subsystem Discovery

You are analyzing a codebase to identify its natural subsystem boundaries.

## Entity Census

Total: {total_files} files, {total_interfaces} interfaces, {total_services} services,
{total_classes} classes, {total_functions} functions, {total_schemas} schemas

## Directory Structure with Entity Counts

{for each top-level directory:}
### {dir_name}/
- Files: {count}
- Interfaces: {names, max 10}
- Services: {names, max 5}
- Schemas: {names, max 5}
{end}

## Import Graph Summary

{top 20 most-imported files/modules}
{cross-directory import counts: "auth/ → billing/: 12 imports"}

## Your Task

Propose 3-15 domains (high-level architectural groupings). Each domain should:
1. Group related directories that work together
2. Have a clear name and one-line description
3. Be assigned a color from: blue, green, orange, purple, red, yellow, cyan, pink, teal

Respond with JSON:
```json
{
  "domains": [
    {
      "id": "auth",
      "label": "Auth & Identity",
      "color": "#3b82f6",
      "description": "Authentication, authorization, sessions, OAuth",
      "directories": ["src/auth", "src/sessions", "src/oauth"]
    }
  ]
}
```
```

### 4.2 Step 2: Entity Mapping Prompt (per domain)

```markdown
# Map Entities to Subsystems: {domain.label}

## Domain Context
{domain.description}
Directories: {domain.directories}

## Entities in This Domain

### Interfaces
{for each interface in these directories:}
- {name} ({file}:{line}) — methods: {method_names}
{end}

### Services
{for each service:}
- {name} ({kind}) — methods: {method_names}, routes: {routes}
{end}

### Classes
{list}

### Schemas
{for each schema:}
- {name} ({framework}) — fields: {field_names}
{end}

## Your Task

Break this domain into 1-8 subsystems. Each subsystem is a cohesive unit
that could be developed/deployed independently. Map every entity above
into exactly one subsystem.

Respond with JSON:
```json
{
  "subsystems": [
    {
      "id": "auth-jwt",
      "name": "JWT Provider",
      "description": "Issue and verify JWTs",
      "filePath": "src/auth/jwt/",
      "interfaces": ["AuthPrincipal", "JWTClaims"],
      "operations": ["signToken()", "verifyToken()"],
      "tables": ["users"],
      "events": []
    }
  ]
}
```
```

### 4.3 Step 3: Connection Inference Prompt

```markdown
# Infer Cross-Subsystem Connections

## Subsystems
{for each subsystem: id, name, domain, interface/operation summary}

## Import Graph Between Subsystems
{aggregated: subsystem A imports N symbols from subsystem B}

## Your Task

For each significant dependency, create a connection with:
- `type`: "depends_on" (hard dependency), "uses" (soft/optional), or "triggers" (event-driven)
- `label`: one-line description of WHY this connection exists

Respond with JSON:
```json
{
  "connections": [
    {
      "from": "billing",
      "to": "auth",
      "type": "depends_on",
      "label": "Validates payment tokens via auth middleware"
    }
  ]
}
```
```

---

## 5. Data Flow

### 5.1 New Core Functions

```rust
// In domain_scan_core::manifest_builder (new module)

/// Generate the Step 1 prompt from a ScanIndex
pub fn generate_domain_proposal_prompt(index: &ScanIndex) -> String

/// Parse LLM's domain proposal response
pub fn parse_domain_proposals(json: &str) -> Result<Vec<DomainProposal>, DomainScanError>

/// Generate Step 2 prompts (one per domain)
pub fn generate_entity_mapping_prompts(
    index: &ScanIndex,
    domains: &[DomainProposal],
) -> Vec<(String, String)>  // (domain_id, prompt)

/// Parse LLM's entity mapping response for one domain
pub fn parse_entity_mappings(json: &str) -> Result<Vec<SubsystemProposal>, DomainScanError>

/// Generate Step 3 prompt from all subsystems + resolver import graph
pub fn generate_connection_prompt(
    index: &ScanIndex,
    subsystems: &[SubsystemProposal],
) -> String

/// Parse LLM's connection response
pub fn parse_connections(json: &str) -> Result<Vec<Connection>, DomainScanError>

/// Assemble everything into a SystemManifest
pub fn build_manifest(
    meta: ManifestMeta,
    domains: Vec<DomainProposal>,
    subsystems: Vec<SubsystemProposal>,
    connections: Vec<Connection>,
) -> SystemManifest
```

### 5.2 New Types

```rust
pub struct DomainProposal {
    pub id: String,
    pub label: String,
    pub color: String,
    pub description: String,
    pub directories: Vec<String>,
}

pub struct SubsystemProposal {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub description: String,
    pub file_path: String,
    pub interfaces: Vec<String>,
    pub operations: Vec<String>,
    pub tables: Vec<String>,
    pub events: Vec<String>,
}
```

### 5.3 New CLI Command

```
domain-scan init [OPTIONS]

OPTIONS:
    --root <PATH>           Codebase root (default: .)
    --step <1|2|3|4>        Which step to run (default: 1)
    --apply <FILE>          Apply LLM response from previous step
    -o, --out <FILE>        Output file (default: system.json)
    --output json|prompt    Output format (default: prompt)
```

### 5.4 New Tauri IPC Commands

```rust
/// Generate the prompt for the current step of manifest building
#[tauri::command]
fn generate_init_prompt(
    step: u8,
    proposals: Option<String>,  // JSON from previous step
    state: State<AppState>,
) -> Result<String, CommandError>

/// Parse and apply LLM response for the current step
#[tauri::command]
fn apply_init_response(
    step: u8,
    response: String,  // raw LLM JSON response
    state: State<AppState>,
) -> Result<InitStepResult, CommandError>

/// Get the current state of the manifest being built
#[tauri::command]
fn get_init_progress(
    state: State<AppState>,
) -> Result<InitProgress, CommandError>

/// Finalize and save the manifest
#[tauri::command]
fn finalize_manifest(
    path: String,
    state: State<AppState>,
) -> Result<SystemManifest, CommandError>
```

### 5.5 New AppState Extension

```rust
pub struct AppState {
    // existing...
    pub current_index: Mutex<Option<ScanIndex>>,
    pub current_root: Mutex<Option<PathBuf>>,
    pub current_manifest: Mutex<Option<SystemManifest>>,
    pub current_match_result: Mutex<Option<MatchResult>>,
    // NEW:
    pub init_progress: Mutex<Option<InitProgress>>,
}

pub struct InitProgress {
    pub step: u8,  // 1-4
    pub domains: Vec<DomainProposal>,
    pub subsystems: Vec<SubsystemProposal>,
    pub connections: Vec<Connection>,
    pub unmatched_entities: Vec<EntitySummary>,
}
```

---

## 6. Tauri App: Wizard UI

### 6.1 Components

| Component | Purpose |
|-----------|---------|
| `ManifestWizard.tsx` | Top-level wizard container, step navigation |
| `WizardStepDomains.tsx` | Step 1: show directory census, display/edit domain proposals |
| `WizardStepSubsystems.tsx` | Step 2: per-domain entity mapping, drag-and-drop entities between subsystems |
| `WizardStepConnections.tsx` | Step 3: show inferred connections, edit types/labels |
| `WizardStepReview.tsx` | Step 4: final review, save manifest, render tube map preview |
| `DomainCard.tsx` | Editable domain card (name, color, directories) |
| `SubsystemCard.tsx` | Editable subsystem card (name, entities, move entities) |
| `ConnectionRow.tsx` | Editable connection row (from → to, type, label) |
| `LlmPromptPanel.tsx` | Shows the generated prompt, "Copy to clipboard" button, paste-back area for LLM response |

### 6.2 Wizard Flow in the App

When tube map tab is opened with no manifest:

1. Show "Build Your Subsystem Map" wizard instead of the empty "Load Manifest" CTA
2. If scan exists: auto-generate Step 1 prompt, show directory census
3. User clicks "Generate with AI" → prompt copied to clipboard (or sent via configured API key)
4. User pastes LLM response → domains populate
5. User edits/approves → Step 2 auto-generates per-domain prompts
6. Repeat for subsystems and connections
7. Step 4: "Save Manifest" → writes `system.json` → tube map renders immediately

### 6.3 Progressive Enhancement

The wizard works at three levels:

**Level 1: Fully manual** — User creates domains/subsystems by hand using the wizard forms. No LLM needed. Good for small codebases (< 20 files).

**Level 2: Copy-paste LLM** — Wizard generates prompts, user copies to Claude/ChatGPT, pastes response back. No API key needed.

**Level 3: Direct LLM API** — If user configures an API key (Anthropic/OpenAI) in settings, the wizard calls the LLM directly. Fully automated with human-in-the-loop approval at each step.

---

## 7. Smart Defaults (No LLM Needed)

Before the LLM even proposes anything, domain-scan can generate smart defaults from the scan data alone:

### 7.1 Directory-Based Domain Heuristics

```typescript
// Group top-level src/ directories into domain candidates
// src/auth/ + src/sessions/ → "auth" domain
// src/billing/ + src/payments/ → "billing" domain

function inferDomainsFromDirectories(index: ScanIndex): DomainProposal[] {
  // 1. Group entities by top-level directory
  // 2. Merge directories that share >50% of their imports
  // 3. Assign names from directory names
  // 4. Assign colors from palette
}
```

### 7.2 Import-Graph Clustering

```typescript
// Entities that import each other heavily belong to the same subsystem
// Use the resolver's import graph to cluster entities

function inferSubsystemsFromImports(
  index: ScanIndex,
  domain: DomainProposal
): SubsystemProposal[] {
  // 1. Build import adjacency matrix for entities in this domain
  // 2. Find connected components (strongly connected = same subsystem)
  // 3. Name subsystems from the most common interface/class prefix
}
```

### 7.3 Connection Inference from Imports

```typescript
// If subsystem A has files that import from subsystem B's files:
// → create a "depends_on" connection

function inferConnectionsFromImports(
  index: ScanIndex,
  subsystems: SubsystemProposal[]
): Connection[] {
  // 1. For each pair of subsystems, count cross-subsystem imports
  // 2. If count > 0, create connection
  // 3. Type: "depends_on" if imports are in main code,
  //          "uses" if only in tests,
  //          "triggers" if the imported symbol is an event emitter
}
```

These heuristics run first, then the LLM refines them (better names, better groupings, connection labels).

---

## 8. Build Phases

### Phase G.1: Core Prompt Generation

- [ ] Create `crates/domain-scan-core/src/manifest_builder.rs` module
- [ ] Implement `generate_domain_proposal_prompt(index)` — directory census, entity summary, import graph
- [ ] Implement `parse_domain_proposals(json)` — validate LLM JSON response
- [ ] Implement `generate_entity_mapping_prompts(index, domains)` — per-domain entity listing
- [ ] Implement `parse_entity_mappings(json)` — validate subsystem proposals
- [ ] Implement `generate_connection_prompt(index, subsystems)` — import-graph-based
- [ ] Implement `parse_connections(json)` — validate connection proposals
- [ ] Implement `build_manifest(meta, domains, subsystems, connections)` — assemble SystemManifest
- [ ] Unit tests for each function with fixture data
- [ ] Integration test: generate prompts → parse mock LLM responses → produce valid system.json

### Phase G.2: Smart Defaults (Heuristic)

- [ ] Implement `infer_domains_from_directories(index)` — directory grouping heuristic
- [ ] Implement `infer_subsystems_from_imports(index, domain)` — import clustering
- [ ] Implement `infer_connections_from_imports(index, subsystems)` — cross-subsystem import counting
- [ ] Test: scan domain-scan's own codebase → heuristics produce reasonable domains/subsystems
- [ ] Test: scan octospark fixtures → heuristics produce domains matching the hand-crafted system.json

### Phase G.3: CLI `domain-scan init`

- [ ] Add `init` subcommand to CLI with `--step`, `--apply`, `-o` flags
- [ ] Step 1: generate domain proposal prompt to stdout
- [ ] Step 2: `--apply proposals.json` → generate entity mapping prompts
- [ ] Step 3: `--apply mappings.json` → generate connection prompt
- [ ] Step 4: `--apply connections.json -o system.json` → write final manifest
- [ ] `--output json` mode: output structured JSON instead of prompt text
- [ ] CLI integration tests with assert_cmd

### Phase G.4: Tauri IPC Commands

- [ ] Add `init_progress` to AppState
- [ ] Implement `generate_init_prompt` IPC command
- [ ] Implement `apply_init_response` IPC command
- [ ] Implement `get_init_progress` IPC command
- [ ] Implement `finalize_manifest` IPC command

### Phase G.5: Tauri Wizard UI

- [ ] Create `ManifestWizard.tsx` — step navigation, progress indicator
- [ ] Create `WizardStepDomains.tsx` — directory census + domain proposal cards
- [ ] Create `DomainCard.tsx` — editable domain (name, color picker, directory list)
- [ ] Create `WizardStepSubsystems.tsx` — per-domain entity mapping
- [ ] Create `SubsystemCard.tsx` — editable subsystem (name, entity pills, move entities)
- [ ] Create `WizardStepConnections.tsx` — connection list with type/label editing
- [ ] Create `ConnectionRow.tsx` — editable connection row
- [ ] Create `WizardStepReview.tsx` — final review + save + tube map preview
- [ ] Create `LlmPromptPanel.tsx` — copy prompt / paste response
- [ ] Wire wizard into tube map tab (replaces "Load Manifest" CTA when no manifest)
- [ ] On "Save Manifest" → immediately load into tube map view

**Acceptance criteria:**
- `domain-scan init` on a real codebase generates a valid prompt that produces reasonable subsystems when sent to Claude
- The heuristic defaults produce at least 70% overlap with an LLM-refined version on octospark
- The Tauri wizard walks through all 4 steps and produces a valid system.json
- The saved manifest immediately renders as a tube map
- The wizard works without an LLM (manual entry mode)
- The wizard works with copy-paste LLM (prompt copy + response paste)

---

## 9. Claude Code / Codex Skill — The Actual Interface

There is no LLM provider abstraction to build. The user is already inside Claude Code or Codex — *that agent IS the LLM*. It reads the scan output, proposes the manifest JSON, and runs the CLI commands to apply it. All we need is:

1. **CLI commands** that output scan data as JSON and accept manifest patches as JSON
2. **A skill file** that teaches the agent how to use those commands and what a good manifest looks like

### 9.1 How It Works

```
user> build me a tube map for this repo

claude> [reads skills/domain-scan-init.md]
        1. Running: domain-scan scan --root . --output json --fields files.path,files.language,stats
        2. [reads the scan output, sees 45 interfaces across src/auth/, src/billing/, src/media/...]
        3. [proposes system.json based on directory structure and entity names]
        4. Running: domain-scan init --apply-manifest system.json --dry-run
        5. [shows the user: "5 domains, 12 subsystems, 18 connections — 87% coverage"]
        6. User approves
        7. Running: domain-scan init --apply-manifest system.json
        Done. Open the Tauri app to see your tube map.

user> the auth subsystem is too big, split JWT and OAuth

claude> [reads current system.json]
        [edits it: splits auth into auth-jwt and auth-oauth, moves entities]
        Running: domain-scan match --manifest system.json --output json --fields coverage_percent,unmatched
        Coverage: 87% → 89% (2 previously unmatched entities now mapped)
        [writes updated system.json]
```

The agent doesn't call any LLM API. It *is* the LLM. It reads files, writes JSON, and runs CLI commands. The structured output is just "write valid system.json" — which Claude Code already does perfectly.

### 9.2 Skill File: `skills/domain-scan-init.md`

```yaml
---
name: domain-scan-init
version: 1.0.0
description: Build and refine a subsystem tube map manifest via LLM-driven discovery
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Build & Refine Subsystem Maps

## When to use
When the user wants to create, update, or refine a system.json manifest
for visualizing their codebase as a subsystem tube map.

## Workflow

### First time (no system.json exists)
1. `domain-scan scan --root . --output json -o /tmp/scan.json`
2. `domain-scan init --root . --output json` → generates domain proposals
3. Review proposals with user, apply edits
4. `domain-scan init --root . --step 2 --apply domains.json --output json` → entity mappings
5. Review, iterate
6. `domain-scan init --root . --step 3 --apply mappings.json --output json` → connections
7. `domain-scan init --root . --step 4 --apply connections.json -o system.json`

### Refining an existing manifest
Use `domain-scan init --refine` with natural language instructions:

```bash
# Split a subsystem
domain-scan init --refine --manifest system.json \
  --instruction "split auth into auth-jwt and auth-oauth, move JWT interfaces to auth-jwt"

# Merge subsystems
domain-scan init --refine --manifest system.json \
  --instruction "merge media-uploader and media-enrichment into a single media-pipeline subsystem"

# Add a connection
domain-scan init --refine --manifest system.json \
  --instruction "add a triggers connection from publisher to notifications"

# Rename a domain
domain-scan init --refine --manifest system.json \
  --instruction "rename the 'services' domain to 'external-integrations'"
```

## What makes a good manifest patch

When proposing or refining subsystems, follow these principles:

### Naming conventions
- Domain IDs: kebab-case, 1-3 words (`platform-core`, `media-storage`)
- Subsystem IDs: kebab-case, prefixed by parent if nested (`auth-jwt`, `auth-sessions`)
- Subsystem names: human-readable, title case (`JWT Provider`, `Session Manager`)
- Connection labels: start with a verb, describe WHY not WHAT (`Validates payment tokens`, not `calls auth`)

### Grouping principles
- A subsystem should be **independently deployable** — if you can't ship it alone, it's too small
- A subsystem should have **one clear responsibility** — if the description needs "and", consider splitting
- 3-8 subsystems per domain is the sweet spot. 1 = too broad, 15+ = too granular
- Interfaces and operations that share the same file path prefix almost always belong together
- Schemas (Drizzle tables, Pydantic models) anchor subsystems — the table owner IS the subsystem

### Connection semantics
- `depends_on`: subsystem B MUST exist and be correct for A to work (hard dependency)
- `uses`: A calls B but could degrade gracefully without it (soft dependency)
- `triggers`: A fires events that B consumes (event-driven, one-way, async)
- When in doubt, use `depends_on` — it's the safest default
- Connections are between top-level subsystems, NOT between children

### What to avoid
- Don't create subsystems for utility/helper modules — they belong inside the subsystem that uses them
- Don't create a "shared" or "common" domain — force entities into specific subsystems
- Don't duplicate entities across subsystems — each entity belongs to exactly one
- Don't add connections for test-only dependencies

## Rules
- Always `--dry-run` before applying refinements
- Always show the user what will change before writing
- Use `--fields` on scan commands to limit context
- After refining, run `domain-scan match --manifest system.json` to verify coverage
- The manifest is the user's intent — never overwrite without approval

## Common mistakes
- Running `init` without scanning first → scan must exist for entity mapping
- Applying refinements to wrong manifest file → always confirm path
- Forgetting to re-match after refinement → coverage % may change
- Creating too many small subsystems → merge aggressively, split only when the user says to
- Naming subsystems after files instead of capabilities → `auth-jwt` not `jwt-provider-ts`
```

### 10.3 Skill File: `skills/domain-scan-tube-map.md`

```yaml
---
name: domain-scan-tube-map
version: 1.0.0
description: View and interact with subsystem tube maps in the terminal
metadata:
  openclaw:
    requires:
      bins: ["domain-scan"]
---

# Subsystem Tube Map

## When to use
When the user wants to understand the architecture of a codebase,
view subsystem dependencies, or check manifest coverage.

## Key commands

```bash
# View tube map data as JSON
domain-scan match --manifest system.json --output json

# Check coverage
domain-scan match --manifest system.json --output table

# Show unmatched entities
domain-scan match --manifest system.json --unmatched-only

# Generate prompts for unmatched items
domain-scan match --manifest system.json --prompt-unmatched --agents 3

# Validate manifest
domain-scan validate --manifest system.json --strict
```

## Rules
- Always check coverage after manifest changes
- Use `--fail-on-unmatched` in CI to enforce full mapping
- Unmatched entities indicate either missing subsystems or incorrect filePaths
```

### 10.4 CLI `--refine` Flag

The key addition for vibe-coding. Accepts natural language instructions:

```
domain-scan init --refine [OPTIONS]

OPTIONS:
    --manifest <PATH>       Existing manifest to refine (required)
    --instruction <TEXT>    Natural language instruction (what to change)
    --provider <PROVIDER>   LLM provider (default: from config)
    --model <MODEL>         LLM model (default: from config)
    --dry-run               Show what would change without writing
    -o, --out <FILE>        Output file (default: overwrites --manifest)
```

Under the hood:
1. Loads existing manifest + scan index
2. Builds a `ManifestRefinementRequest` with the instruction + current state
3. Sends to LLM with structured output schema (`ManifestRefinementResponse`)
4. Validates the response (all referenced entities/subsystems exist)
5. Applies splits, merges, renames, entity moves, connection changes
6. `--dry-run`: shows diff of what would change
7. Without `--dry-run`: writes updated manifest

This is what makes the vibe-coding loop work — the user says "split auth" in Claude Code, the skill runs `domain-scan init --refine --instruction "split auth"`, and the manifest updates.

---

## 10. Build Phases (Updated)

### Phase G.1: Core Prompt Generation (unchanged)

### Phase G.2: Smart Defaults (unchanged)

### Phase G.3: CLI `domain-scan init` (unchanged)

### Phase G.4: Tauri IPC Commands (unchanged)

### Phase G.5: Tauri Wizard UI (unchanged)

### Phase G.6: Agent Skill Files + `--apply-manifest` CLI

- [ ] Add `--apply-manifest <PATH>` flag to `domain-scan init` — validates and writes a system.json
- [ ] Add `--dry-run` to `--apply-manifest` — shows coverage and validation without writing
- [ ] `domain-scan schema init` — dumps the system.json JSON Schema so the agent can validate before writing
- [ ] Create `skills/domain-scan-init.md` — full manifest building/refining skill with patch guidelines
- [ ] Create `skills/domain-scan-tube-map.md` — tube map viewing/interaction skill
- [ ] Update `skills/domain-scan-scan.md` — add init workflow reference
- [ ] `domain-scan init --bootstrap` — generates a starter system.json from heuristic defaults (directory grouping + import clustering) that the agent then refines
- [ ] Embed skill files in the CLI binary via `include_str!`
- [ ] Add `domain-scan skills list|show|dump|install` subcommand
- [ ] `--claude-code` flag installs skills to `~/.claude/skills/`
- [ ] `--codex` flag installs skills to `~/.codex/skills/` (or equivalent)
- [ ] Add "AGENT SKILLS" section to `--help` output pointing to `domain-scan skills`
- [ ] Test: Claude Code session using skill files can create a manifest from scratch
- [ ] Test: Claude Code session can refine an existing manifest with natural language edits to system.json

**Acceptance criteria:**
- `domain-scan init --bootstrap -o system.json` generates a usable starter manifest from scan data
- `domain-scan init --apply-manifest system.json --dry-run` shows coverage % and validation errors
- `domain-scan schema init` outputs the JSON Schema for system.json
- A Claude Code user can say "build me a tube map for this repo" and the skill file guides the full workflow
- A Claude Code user can edit system.json directly and run `domain-scan match` to verify coverage
- The skill file teaches the agent what a good manifest looks like (naming, grouping, connections)

---

---

## 11. Skill Bootstrapping

The skills should be auto-discoverable. When a user runs `domain-scan` for the first time, the agent should be able to find and install the skills.

### 11.1 `domain-scan skills` CLI Command

```bash
# List available skills
domain-scan skills list

# Print a skill to stdout (agent reads it)
domain-scan skills show domain-scan-init

# Print all skills concatenated (for injecting into agent context)
domain-scan skills dump

# Install skills to Claude Code config
domain-scan skills install --claude-code

# Install skills to Codex config
domain-scan skills install --codex
```

Under the hood, skills are embedded in the binary at compile time via `include_str!("../../skills/*.md")`. No external file dependencies.

### 11.2 Auto-Discovery

When the agent runs `domain-scan --help` or `domain-scan schema init`, the output includes:

```
AGENT SKILLS:
  Run `domain-scan skills show domain-scan-init` to learn how to build a tube map manifest.
  Run `domain-scan skills dump` to load all skills into your context.
```

This teaches any agent (Claude Code, Codex, Gemini CLI) how to bootstrap itself — it reads `--help`, sees the skills hint, loads the skill, and knows the full workflow.

### 11.3 Skill Installation

`domain-scan skills install --claude-code` writes the skill files to `~/.claude/skills/` (or whatever the configured skills directory is). This makes them available in every Claude Code session without manual setup.

---

## 12. Why This Order Matters

The whole point of domain-scan is: **scan → understand → map → visualize.** Without the manifest builder, the "map" step is manual. With it:

1. **Scan** — `domain-scan scan` (already done, deterministic)
2. **Understand** — LLM reads the entity census (generated prompts)
3. **Map** — LLM proposes subsystems, user approves (wizard)
4. **Visualize** — tube map renders immediately (already built)

This closes the loop. A developer can go from `git clone` to a tube map in under 10 minutes.
