# Subsystem Tube Map — Spec for domain-scan Tauri App

> Add a second tab to the domain-scan desktop app: a London Underground-style tube map showing subsystems as stations, domains as colored lines, and dependencies as edges. Powered by `domain-scan match --manifest` data rendered via React Flow.

---

## 1. Overview

### 1.1 Current State

The domain-scan Tauri app has a single view: a three-panel entity browser (Entity Tree | Source Preview | Details Panel). It scans codebases with tree-sitter and shows interfaces, services, classes, methods, schemas, and impls.

It has **no graph visualization**, no subsystem grouping, and no dependency visualization. The manifest matching engine (`manifest.rs`) exists in the core library but is not exposed via Tauri IPC — it's CLI-only today.

### 1.2 Target State

Two tabs in the desktop app:

- **Tab 1: Entities/Types** — the existing three-panel layout (unchanged)
- **Tab 2: Subsystem Tube Map** — a React Flow canvas showing:
  - Domains as colored "tube lines" (like the Central Line, District Line, etc.)
  - Subsystems as "stations" placed along each line
  - Dependencies as styled edges between stations
  - Drill-in: click a station to see its entities
  - Dependency trace: click a station to highlight all upstream/downstream connections
  - Search, filter by domain/status, keyboard navigation

### 1.3 Reference Implementation

The octospark-visualizer (`/Users/jamesaphoenix/Desktop/projects/just-understanding-data/octospark-visualizer/`) is the design reference. It uses React Flow with custom SubsystemNode/DependencyEdge components, a hard-coded tube map layout, and a drill-in InterfaceExplorer. This spec generalizes that approach for dynamic data from domain-scan.

---

## 2. Bug Fix: Open Directory Button

### 2.1 Root Cause

The "Open Directory" button in App.tsx calls `@tauri-apps/plugin-dialog`'s `open()` function. The plugin is installed on both Rust and JS sides and registered in the Tauri builder. However, **the Tauri 2 capabilities file is missing**. Without `dialog:allow-open` permission, the IPC call is silently blocked.

### 2.2 Fix

Create `crates/domain-scan-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capabilities for domain-scan",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:allow-open",
    "shell:allow-open"
  ]
}
```

Also add `shell:allow-open` for the "Open in Editor" feature which likely has the same issue.

### 2.3 Acceptance Criteria

- [ ] Clicking "Open Directory" opens a native macOS/Linux/Windows folder picker
- [ ] Selecting a directory triggers a scan
- [ ] No code changes needed — only the capabilities file

---

## 3. Tab System

### 3.1 Insertion Point

The tab bar inserts between the status bar (App.tsx line 222) and the three-panel div (line 225). The layout becomes:

```
┌──────────────────────────────────────────────────────────┐
│ Status Bar: "domain-scan" | "Open Directory" | Stats     │
├──────────────────────────────────────────────────────────┤
│ [ Entities/Types ]  [ Subsystem Tube Map ]               │
├──────────────────────────────────────────────────────────┤
│ Tab content (full width, flex-1)                         │
└──────────────────────────────────────────────────────────┘
```

### 3.2 State

```typescript
type Tab = "entities" | "tube-map";
const [activeTab, setActiveTab] = useState<Tab>("entities");
```

### 3.3 Shared vs Tab-Specific State

**Shared (stays at App root):**
- `useScan` hook — scan results, stats, entities, IPC methods
- `currentFilters: FilterParams`
- `handleOpenDirectory` — status bar + file dialog

**Tab-specific (scoped to each tab component):**
- Entities tab: `useTreeState`, `selectedDetail`, `sourceCode`, `promptOutput`, `exportOutput`
- Tube Map tab: `useTubeMapState` (new hook), `focusedSubsystemId`, `breadcrumbs`, `manifestPath`

### 3.4 Keyboard Shortcuts

Add `activeTab` parameter to `useKeyboard`. Entities-tab shortcuts (j/k/h/l/p/e) only fire when `activeTab === "entities"`. Tube-map shortcuts (f/i/0/1-9/Escape) only fire when `activeTab === "tube-map"`. `/` (search) fires on both.

### 3.5 Tab Bar Styling

```
flex items-center gap-1 bg-gray-800 border-b border-gray-700 px-4
```

Active tab: `bg-gray-700 text-white font-medium rounded-t-md px-4 py-2 text-sm`
Inactive tab: `text-gray-400 hover:text-gray-200 px-4 py-2 text-sm`

---

## 4. New Tauri IPC Commands

### 4.1 AppState Extension

```rust
pub struct AppState {
    pub current_index: Mutex<Option<ScanIndex>>,
    pub current_root: Mutex<Option<PathBuf>>,
    // NEW:
    pub current_manifest: Mutex<Option<SystemManifest>>,
    pub current_match_result: Mutex<Option<MatchResult>>,
}
```

### 4.2 Extended Manifest Parsing

The current `Manifest` struct only reads `subsystems`. Extend it to a `SystemManifest` that also reads `meta`, `domains`, and `connections` from the system.json format:

```rust
pub struct SystemManifest {
    pub meta: ManifestMeta,
    pub domains: HashMap<String, DomainDef>,
    pub subsystems: Vec<ManifestSubsystem>,  // existing
    pub connections: Vec<Connection>,
}

pub struct ManifestMeta {
    pub name: String,
    pub version: String,
    pub description: String,
}

pub struct DomainDef {
    pub label: String,
    pub color: String,
}

pub struct Connection {
    pub from: String,
    pub to: String,
    pub label: String,
    pub connection_type: ConnectionType,  // depends_on | uses | triggers
}
```

### 4.3 New Commands

| Command | Params | Returns | Purpose |
|---------|--------|---------|---------|
| `load_manifest` | `path: String` | `SystemManifest` | Parse manifest file, store in AppState |
| `match_manifest` | (none — uses AppState) | `MatchResult` | Run `match_entities` on loaded index + manifest |
| `get_tube_map_data` | (none) | `TubeMapData` | Composite: returns subsystems, domains, connections, match counts |
| `get_subsystem_entities` | `subsystem_id: String` | `Vec<EntitySummary>` | Entities belonging to a specific subsystem |
| `get_subsystem_detail` | `subsystem_id: String` | `SubsystemDetail` | Full subsystem with children, matched entity counts |

### 4.4 TubeMapData Type

```rust
pub struct TubeMapData {
    pub meta: ManifestMeta,
    pub domains: HashMap<String, DomainDef>,
    pub subsystems: Vec<TubeMapSubsystem>,
    pub connections: Vec<Connection>,
    pub coverage_percent: f64,
    pub unmatched_count: usize,
}

pub struct TubeMapSubsystem {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub status: String,
    pub description: String,
    pub file_path: String,
    pub matched_entity_count: usize,
    pub interface_count: usize,
    pub operation_count: usize,
    pub table_count: usize,
    pub event_count: usize,
    pub has_children: bool,
    pub child_count: usize,
    pub dependency_count: usize,
}
```

---

## 5. Dynamic Tube Map Layout Engine

### 5.1 Overview

Replace octospark's hard-coded `TUBE_LINES` with a runtime-computed equivalent. The layout is fully deterministic (same input → same output) with no force-directed simulation.

### 5.2 Constants

```typescript
const STATION_GAP = 420;       // px between station centers (inherited from octospark)
const LINE_GAP = 320;          // px between parallel lines
const NODE_WIDTH = 360;        // station node width
const COL_MARGIN = 300;        // px between parallel line groups in same row
const LINE_ROW_HEIGHT = 640;   // px per row (double LINE_GAP for edge room)
const MAX_STATIONS_PER_SEGMENT = 10;  // U-bend wrapping threshold
const BUNDLE_THRESHOLD = 3;    // edges between same two domains before bundling
```

### 5.3 Algorithm: Phase 1 — Domain Row Assignment

1. **Count inter-domain edges.** Build `cross[di][dj] = count` from manifest connections.
2. **Topological sort on domain DAG.** Use Kahn's algorithm. Break cycles by removing the lower-weight edge. Record breaks in `cycleBreaks[]`.
3. **Grid packing.** `MAX_COLS = ceil(sqrt(N))`. Domains at the same topo layer share a row. Within a row, sort by descending station count. Assign `(row, col)` to each domain.
4. **Compute origins.** `origin.x = col * (maxStationsInRow * STATION_GAP + COL_MARGIN)`, `origin.y = row * LINE_ROW_HEIGHT`.

### 5.4 Algorithm: Phase 2 — Station Ordering

Within each domain line, sort subsystems by:
1. Topological depth within domain (Kahn's on intra-domain dependencies) — sources left, sinks right
2. Cross-domain fan-out count (ascending) — highly-depended-on stations go to the middle
3. Alphabetical ID as stable tiebreaker

### 5.5 Algorithm: Phase 3 — U-Bend Wrapping

For lines with more than `MAX_STATIONS_PER_SEGMENT` stations, generate segments that wrap:

```typescript
segments: [
  { steps: 9, dx: 1, dy: 0 },   // right for 9
  { steps: 1, dx: 0, dy: 1 },   // down
  { steps: 9, dx: -1, dy: 0 },  // left for 9
  { steps: 1, dx: 0, dy: 1 },   // down again
  // ... repeat
]
```

The existing `buildStationPositions()` segment walker handles this without modification.

### 5.6 Algorithm: Compact Mode (Filtered)

Same as octospark's compact algorithm:
1. Group visible stations by computed line, preserving order
2. `maxLineWidth = max(stationCount - 1) * STATION_GAP`
3. Center each line: `offsetX = (maxLineWidth - lineWidth) / 2`
4. Stack lines: `currentY += LINE_GAP`
5. Fallback row for unrecognized stations

### 5.7 Color Assignment

If manifest has `domains` with colors: use them.
Otherwise: assign from a 12-color static palette (`blue, green, orange, purple, red, yellow, cyan, pink, teal, amber, indigo, lime`). If N > 12, cycle with `d3.hsl(hue, 0.65, 0.55)` using `hue = (i / N) * 360`.

### 5.8 Files to Create

- `ui/src/layout/types.ts` — `ComputedLine`, `LayoutGrid`, `DomainLayer`
- `ui/src/layout/colors.ts` — `assignDomainColors(domains) → Map<string, string>`
- `ui/src/layout/tubeMap.ts` — `buildDynamicLayout(manifest, options) → { nodes, edges }`

---

## 6. React Flow Integration

### 6.1 Dependencies

```
npm install @xyflow/react@^12.10.1
```

No dagre or elkjs needed. The layout is computed by the algorithm above.

### 6.2 Provider Setup

Wrap the tube map tab content in `<ReactFlowProvider>`:

```tsx
// Module-level (outside component)
const nodeTypes = { subsystem: SubsystemNode };
const edgeTypes = { dependency: DependencyEdge };

// Inside TubeMapView component
<ReactFlow
  nodes={nodes}
  edges={edges}
  nodeTypes={nodeTypes}
  edgeTypes={edgeTypes}
  fitView
  fitViewOptions={{ padding: 0.15, maxZoom: 1 }}
  minZoom={0.1}
  maxZoom={2}
  proOptions={{ hideAttribution: true }}
>
  <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#1e293b" />
  <Controls showInteractive={false} />
  <MiniMap pannable zoomable />
</ReactFlow>
```

### 6.3 CSP

No changes needed. The existing `style-src 'self' 'unsafe-inline'` in `tauri.conf.json` is compatible with React Flow's runtime style injection.

### 6.4 CSS

Import once in `main.tsx` or `App.tsx`:
```typescript
import "@xyflow/react/dist/style.css";
```

---

## 7. React Components

### 7.1 Components to Port from octospark (adapted)

| Component | Changes Needed |
|-----------|---------------|
| `SubsystemNode.tsx` | Replace octospark-specific type imports. Wire callbacks to Tauri IPC. Add `matchedEntityCount` display. |
| `DependencyEdge.tsx` | Swap `ConnectionType` import to local type. No structural changes. |
| `EdgeTooltip.tsx` | Swap type imports. No structural changes. |
| `Legend.tsx` | Make `DomainId` a `string` (dynamic domains). No structural changes. |
| `Breadcrumbs.tsx` | Reuse as-is. |
| `TubeMapStatusBar.tsx` | Adapt from octospark's StatusBar. Add coverage % display. |
| `SearchBar.tsx` | Adapt domain/status filters for dynamic domains. |

### 7.2 New Components

| Component | Purpose |
|-----------|---------|
| `TubeMapView.tsx` | Top-level tube map container. Wraps ReactFlowProvider + canvas. Manages tube map state. |
| `TabBar.tsx` | Tab switcher (Entities/Types, Subsystem Tube Map). |
| `ManifestLoader.tsx` | UI for loading a manifest file (button + file picker). Shown when no manifest is loaded. |
| `SubsystemDrillIn.tsx` | Drill-in view when a subsystem station is clicked. Shows matched entities in a card grid. Replaces octospark's InterfaceExplorer with domain-scan entity data. |
| `CoverageOverlay.tsx` | Shows match coverage %, unmatched entity count, and "Generate prompts for unmatched" button. |

### 7.3 New Hooks

| Hook | Purpose |
|------|---------|
| `useTubeMapState.ts` | Manages manifest loading, matching, tube map data, focused subsystem, breadcrumbs, dependency trace. Calls new Tauri IPC commands. |
| `useTubeLayout.ts` | Computes React Flow nodes/edges from `TubeMapData` via the dynamic layout algorithm. Pure layout computation, memoized. |

---

## 8. Interaction Model

### 8.1 Keyboard Shortcuts (Tube Map Tab)

| Key | Action |
|-----|--------|
| `f` | Fit view (zoom to show all visible nodes) |
| `i` / `I` | Toggle interface/entity side panel |
| `/` | Focus search input |
| `Escape` | Priority cascade: close panel → clear search → clear dependency trace → clear filters → pop breadcrumb |
| `0` | Clear all filters |
| `1`-`9` | Toggle domain filter (by order of domain in manifest) |
| `?` | Toggle shortcut help overlay |

### 8.2 Dependency Trace

Click a station's "trace" button → sets `focusedSubsystemId`. BFS walks upstream/downstream connections (using the `connections[]` array). Nodes not in the chain are dimmed (`opacity: 0.2`). Edges not in the chain are hidden.

Direction toggle: upstream / downstream / both (rendered in search bar when trace is active).

### 8.3 View Switching

- **Tube map** (default): React Flow canvas with all stations visible
- **Drill-in**: Click a station with children → push to breadcrumbs → show children in a 3-column grid layout with entity cards
- **Back**: Click breadcrumb ancestor or press Escape → pop back to tube map

### 8.4 Manifest Loading Flow

1. User clicks "Load Manifest" button (in tube map tab header, or prompted if no manifest loaded)
2. Native file picker opens (via `@tauri-apps/plugin-dialog`)
3. Selected path passed to `load_manifest` IPC command
4. If scan is loaded, automatically runs `match_manifest`
5. Tube map renders from `TubeMapData`

If no manifest is loaded, the tube map tab shows a centered "Load Manifest" CTA with explanation text.

---

## 9. Build Phases

### Phase A: Foundation — Bug Fix + Tab Shell + IPC Commands

- [x] Create `capabilities/default.json` with `dialog:allow-open` and `shell:allow-open`
- [x] Add `TabBar.tsx` component
- [x] Add `activeTab` state to App.tsx, render tab bar between status bar and content
- [x] Wrap existing three-panel layout in entities tab conditional
- [x] Create placeholder `TubeMapView.tsx` (empty state: "Load a manifest to view the subsystem tube map")
- [x] Extend `Manifest` struct → `SystemManifest` (parse `meta`, `domains`, `connections`)
- [x] Extend `AppState` with `current_manifest` and `current_match_result`
- [x] Implement `load_manifest` IPC command
- [x] Implement `match_manifest` IPC command
- [x] Implement `get_tube_map_data` IPC command
- [x] Implement `get_subsystem_entities` IPC command
- [x] Implement `get_subsystem_detail` IPC command

**Acceptance criteria:**
- Open Directory button works (native file picker appears)
- Tab bar renders, switching between entities view and tube map placeholder
- `load_manifest` + `match_manifest` return valid data from Rust backend
- `get_tube_map_data` returns subsystems with matched entity counts

### Phase B: Layout Engine

- [x] Create `ui/src/layout/types.ts`
- [x] Create `ui/src/layout/colors.ts` with static palette + HSL cycling
- [x] Create `ui/src/layout/tubeMap.ts` with all 5 functions
- [x] Implement `assignDomainLayers` (Kahn's topo sort on domain DAG, cycle breaking)
- [x] Implement `assignDomainGrid` (bin-packing: MAX_COLS, row/col assignment)
- [x] Implement `orderStationsWithinLine` (intra-domain topo sort)
- [x] Implement `buildCanonicalPositions` (segment walker)
- [x] Implement `applyCompactLayout` (filtered compact mode)
- [x] Unit tests for all 5 functions
- [x] Test with octospark's system.json as a reference fixture

**Acceptance criteria:**
- `buildDynamicLayout(octosparkManifest)` produces positions that match the hand-crafted layout within ±1 station gap
- Compact mode correctly centers filtered lines
- Cycle-breaking produces a valid layout for circular dependencies
- 20-domain, 50-station-per-domain synthetic fixture produces no overlapping lines

### Phase C: React Flow Canvas

- [x] `npm install @xyflow/react@^12.10.1`
- [x] Import `@xyflow/react/dist/style.css` in `main.tsx`
- [x] Port `SubsystemNode.tsx` from octospark (adapt types, wire Tauri IPC callbacks)
- [x] Port `DependencyEdge.tsx` from octospark (swap type imports)
- [x] Port `EdgeTooltip.tsx` from octospark
- [x] Create `TubeMapView.tsx` with ReactFlowProvider, canvas, Background, Controls, MiniMap
- [x] Create `useTubeMapState.ts` hook (manifest loading, matching, focused subsystem, breadcrumbs)
- [x] Create `useTubeLayout.ts` hook (memoized layout computation)
- [x] Wire `ManifestLoader.tsx` (file picker + load button)
- [x] Render tube map nodes/edges from `useTubeLayout` output

**Acceptance criteria:**
- Loading a manifest renders stations on a React Flow canvas
- Stations show name, status badge, domain color border, entity counts
- Edges render between connected stations with correct styling (solid/dashed, colored)
- Pan/zoom works with mouse, MiniMap renders
- `fitView` fires on initial load

### Phase D: Interaction — Search, Filter, Trace, Drill-In

- [x] Port `SearchBar.tsx` (adapt for dynamic domains)
- [x] Port `Legend.tsx` (dynamic domain colors)
- [x] Port `Breadcrumbs.tsx` (reuse as-is)
- [x] Create `TubeMapStatusBar.tsx` (zoom %, visible/total nodes, coverage %)
- [x] Implement domain filter (click legend line → filter to that domain)
- [x] Implement status filter (built/rebuild/new/boilerplate toggles)
- [x] Implement text search (filter stations by name/description/interfaces)
- [x] Implement compact re-layout on filter change
- [x] Implement dependency trace (BFS walk, dimming, edge filtering)
- [x] Implement direction toggle (upstream/downstream/both)
- [x] Create `SubsystemDrillIn.tsx` (click station → show entities in card grid)
- [x] Wire breadcrumb navigation (tube map ↔ drill-in)
- [x] Create `CoverageOverlay.tsx` (match coverage %, unmatched count)
- [x] Wire keyboard shortcuts for tube map tab
- [x] Create `ShortcutHelp.tsx` overlay

**Acceptance criteria:**
- Clicking a domain in the legend filters to that domain only
- Searching "auth" highlights/filters stations matching "auth"
- Clicking "trace" on a station highlights the full dependency chain
- Direction toggle switches between upstream/downstream/both
- Clicking a station with children opens drill-in view with entity cards
- Breadcrumbs navigate back to tube map
- Coverage overlay shows match % and unmatched count
- All keyboard shortcuts work (f, i, /, Escape, 0, 1-9, ?)

### Phase E: Polish

- [x] Edge bundling for dense inter-domain connections (>3 edges → single bundle edge with count)
- [x] Tube line stripe rendering (colored SVG paths behind stations)
- [x] Animate fitView transitions (duration: 300ms)
- [x] Toast notifications for file opening, manifest loading
- [x] Open in editor from station node (uses `open_in_editor` IPC command)
- [x] "Generate Prompt" button on drill-in view (scoped to subsystem entities)
- [x] Handle missing-domain case: unassigned entities on gray "unassigned" line
- [x] Performance: `onlyRenderVisibleElements={true}` when total nodes > 500
- [x] Snapshot tests for layout algorithm output

**Acceptance criteria:**
- Dense edges are bundled with count badges
- Tube line stripes render behind stations with domain colors
- Transitions are smooth (no flicker on filter/trace changes)
- Open in editor works for station file paths
- App remains responsive with 500+ nodes

### Phase F: Hardening — E2E Tests, Bug Hunting, Edge Cases

Automated end-to-end tests using Playwright + Tauri's WebDriver bridge, plus targeted stress tests and adversarial scenarios.

#### F.0 CI Fix (HIGH PRIORITY — CI is currently broken)

- [x] Fix clippy `panic!` violations in `manifest.rs` test code (lines 849, 916, 931, 940, 965, 980, 983) — replace `.unwrap_or_else(|e| panic!(...))` with proper error handling or `#[allow(clippy::panic)]` on test functions
- [x] Fix clippy `unnecessary_map_or` in `resolver.rs:642` — replace `.map_or(false, |v| ...)` with `.is_some_and(|v| ...)`
- [x] Fix `unused_comparisons` in `index.rs:1010` — remove `assert!(index.stats.total_files >= 0)` (usize is always >= 0)
- [x] Verify: `cargo clippy --all-targets -- -D warnings` passes with zero errors
- [x] Verify: CI passes on GitHub Actions after push

#### F.1 E2E Test Infrastructure

- [x] Add `@playwright/test` and `@tauri-apps/driver` to `ui/` dev dependencies
- [x] Create `e2e/` directory in `crates/domain-scan-tauri/ui/`
- [x] Create `e2e/fixtures/` with test manifests:
  - `octospark-system.json` — copy of real octospark manifest (7 domains, 18 subsystems, 50 connections)
  - `minimal.json` — 1 domain, 2 subsystems, 1 connection (smoke test)
  - `large.json` — 20 domains, 200 subsystems, 500 connections (stress test)
  - `empty.json` — valid manifest with zero subsystems
  - `malformed.json` — invalid JSON for error handling
  - `circular-deps.json` — subsystems with mutual circular dependencies
  - `no-domains.json` — manifest with subsystems but no `domains` field
  - `orphan-subsystems.json` — subsystems whose `domain` doesn't exist in `domains` map
- [x] Create `e2e/helpers.ts` with utilities: `launchApp()`, `openDirectory(path)`, `loadManifest(path)`, `switchTab(name)`, `waitForScan()`, `waitForTubeMap()`
- [x] Configure Playwright to launch the Tauri app via `cargo tauri dev` with `TAURI_TEST=1` env var

#### F.2 E2E: Open Directory & Scan Flow

- [x] Test: click "Open Directory" → native dialog appears (or mock path injection in test mode)
- [x] Test: scan a fixture directory → stats bar shows correct file/entity counts
- [x] Test: scan completes → entities tab shows tree with nodes
- [x] Test: scan empty directory → structured error shown (not a crash)
- [x] Test: scan non-existent path → structured error shown

#### F.3 E2E: Tab Switching

- [x] Test: app starts on Entities tab by default
- [x] Test: click Tube Map tab → tube map placeholder renders (no manifest loaded)
- [x] Test: switch back to Entities → tree state preserved (selection, expansion)
- [x] Test: rapid tab switching (10x in 1 second) → no crash, no leaked state

#### F.4 E2E: Manifest Loading & Matching

- [x] Test: load `minimal.json` → 2 subsystem nodes render on canvas
- [x] Test: load `octospark-system.json` → 18 nodes render, 50 edges visible
- [x] Test: load `empty.json` → "No subsystems found" message, no crash
- [x] Test: load `malformed.json` → structured error toast, tube map stays on loader view
- [x] Test: load manifest before scan → matching skipped gracefully, entities show as unmatched
- [x] Test: load manifest after scan → matching runs, coverage % shown
- [x] Test: reload different manifest → old match results cleared, new data renders

#### F.5 E2E: Tube Map Interactions

- [x] Test: pan canvas with mouse drag → viewport moves
- [x] Test: zoom with scroll wheel → zoom level changes, StatusBar updates
- [x] Test: click station node → details panel shows subsystem info
- [x] Test: click station with children → drill-in view opens, breadcrumbs update
- [x] Test: click breadcrumb → navigates back, tube map restores
- [x] Test: click domain in legend → filters to that domain only, compact layout triggers
- [x] Test: type in search bar → stations filter by name, layout re-compacts
- [x] Test: clear search → all stations reappear at canonical positions
- [x] Test: click "trace" on a station → dependency chain highlighted, non-chain nodes dimmed
- [x] Test: press Escape during trace → trace clears, all nodes restore opacity

#### F.6 E2E: Keyboard Shortcuts (Tube Map Tab)

- [x] Test: press `f` → fitView fires, all nodes visible
- [x] Test: press `/` → search input focused
- [x] Test: press `1`-`7` → corresponding domain filter toggles
- [x] Test: press `0` → all filters cleared
- [x] Test: press `?` → shortcut help overlay appears
- [x] Test: press `Escape` → overlay/search/trace/filter cleared in priority order
- [x] Test: keyboard shortcuts do NOT fire when typing in search input

#### F.7 Stress Tests & Edge Cases

- [x] Test: load `large.json` (200 subsystems) → renders within 3 seconds, pan/zoom stays smooth
- [x] Test: load `circular-deps.json` → cycle-breaking produces valid layout, warning badge shown
- [x] Test: load `no-domains.json` → all subsystems render on gray "unassigned" line
- [x] Test: load `orphan-subsystems.json` → orphan subsystems placed in fallback row, no crash
- [x] Test: window resize → layout reflows, no overlapping nodes, MiniMap updates
- [x] Test: minimize/restore window → React Flow canvas re-renders correctly
- [x] Test: scan a 1000-file codebase → match against large manifest → tube map renders without OOM
- [x] Test: double-click station rapidly → no duplicate drill-in views, breadcrumbs don't double-push

#### F.8 Data Integrity Checks

- [x] Test: `get_tube_map_data` entity counts match `filter_entities` counts for each subsystem
- [x] Test: `match_manifest` coverage % is consistent: `matched.len() / total_entities * 100`
- [x] Test: `get_subsystem_entities(id)` returns only entities whose files fall under subsystem filePath
- [x] Test: connections reference only valid subsystem IDs (no dangling `from`/`to`)
- [x] Test: after scan + match, switching to Entities tab still works (shared state not corrupted)
- [x] Test: generate prompt from tube map drill-in → valid prompt text, scoped to subsystem entities

#### F.9 Error Recovery

- [x] Test: Tauri IPC command fails (e.g., file deleted mid-scan) → structured error shown, app stays functional
- [x] Test: manifest file deleted after loading → next `match_manifest` call returns error, tube map shows "reload manifest" CTA
- [x] Test: corrupt cache directory → scan falls back to no-cache mode, completes successfully
- [x] Test: extremely long subsystem names (500+ chars) → node renders without overflow, tooltip shows full name

#### F.10 Manifest Builder — CLI Integration Tests

- [x] Test: `domain-scan init --bootstrap -o system.json` on fixture codebase → produces valid JSON matching system.json schema
- [x] Test: `domain-scan init --bootstrap` on empty directory → produces manifest with zero subsystems, no crash
- [x] Test: `domain-scan init --apply-manifest system.json --dry-run` → shows coverage %, validation errors, writes nothing
- [x] Test: `domain-scan init --apply-manifest system.json` → writes file, re-reading it produces identical SystemManifest
- [x] Test: `domain-scan init --apply-manifest malformed.json` → structured error, no file written
- [x] Test: `domain-scan schema init` → output is valid JSON Schema, validates octospark system.json
- [x] Test: bootstrap → match pipeline: `--bootstrap` output piped to `match --manifest` → coverage > 0%
- [x] Test: heuristic domains match directory structure (each top-level src/ dir → one domain candidate)
- [x] Test: heuristic connections inferred from cross-directory imports (if A imports B → connection exists)
- [x] Test: bootstrap on domain-scan's own codebase → produces ≥2 domains (core, cli at minimum)

#### F.11 Manifest Builder — Tauri Wizard Integration Tests

- [x] Test: wizard step 1 (domains) renders directory census from scan data
- [x] Test: editing a domain name in wizard → reflected in generated manifest
- [x] Test: wizard step 2 (subsystems) shows entities grouped by domain
- [x] Test: moving an entity between subsystems in wizard → manifest updated correctly
- [x] Test: wizard step 3 (connections) shows inferred connections from imports
- [x] Test: wizard step 4 (review) → "Save Manifest" writes file and switches to tube map view
- [x] Test: wizard → save → tube map renders matching stations/edges from saved manifest
- [x] Test: re-opening wizard after saving → loads existing manifest, not blank slate

#### F.12 Skill Bootstrapping Tests

- [x] Test: `domain-scan skills list` → outputs all embedded skill names
- [x] Test: `domain-scan skills show domain-scan-init` → outputs valid YAML frontmatter + markdown
- [x] Test: `domain-scan skills dump` → concatenated output contains all skills
- [x] Test: `domain-scan skills install --claude-code` → creates `.claude/skills/domain-scan-init.md` in project root
- [x] Test: `domain-scan skills install --codex` → creates `.codex/skills/domain-scan-init.md` in project root
- [x] Test: `domain-scan skills install --dir custom/path/` → creates `custom/path/domain-scan-init.md`
- [x] Test: running install twice → files overwritten, no duplicates
- [x] Test: `.gitignore` updated to include skills directory after install
- [x] Test: `domain-scan --help` output contains "AGENT SKILLS" section
- [x] Test: installed skill file content matches embedded `skills show` output exactly

**Acceptance criteria:**
- All E2E tests pass in CI (GitHub Actions with `cargo tauri build` + Playwright)
- Zero crashes across all adversarial fixtures
- Circular dependencies produce valid layouts with warning badges
- 200-subsystem manifest renders and is interactive within 3 seconds
- Tab switching preserves state correctly (no cross-tab contamination)
- All keyboard shortcuts work only in their correct tab context
- Error states show structured messages (never blank screens or stack traces)
- Data integrity: entity counts, coverage %, and match results are all consistent
- Bootstrap produces valid manifests for any scanned codebase
- Wizard round-trips correctly (save → reload → identical manifest)
- Skill files install to project directory, not global config

---

## 10. Testing Strategy

### Unit Tests

- Layout algorithm: `assignDomainLayers`, `assignDomainGrid`, `orderStationsWithinLine`, `buildCanonicalPositions`, `applyCompactLayout`
- Color assignment: palette cycling, d3-hsl fallback
- Cycle breaking: mutual dependency between 2 domains, 3-way cycle

### Integration Tests

- Tauri IPC: `load_manifest` → `match_manifest` → `get_tube_map_data` pipeline
- Octospark system.json as a reference fixture (18 subsystems, 50 connections, 7 domains)

### Snapshot Tests (insta)

- Layout positions for octospark fixture
- Layout positions for a synthetic 20-domain fixture
- Compact layout positions for a filtered subset

### Manual Test Scenarios

- Load octospark's system.json → tube map matches the octospark-visualizer layout
- Scan domain-scan's own codebase → load a minimal manifest → see entities grouped by subsystem
- Filter to single domain → compact layout centers correctly
- Trace dependencies from a central station → upstream+downstream chain highlighted
- Drill into a station with children → see entity cards → navigate back via breadcrumbs

---

## 11. Dependencies

### New npm packages (Tauri UI)
- `@xyflow/react: ^12.10.1`

### New Rust crate dependencies
- None — all manifest/matching logic already exists in `domain-scan-core`

### Files to create (estimated)

| File | Purpose | Est. LOC |
|------|---------|----------|
| `capabilities/default.json` | Tauri ACL permissions | 10 |
| `ui/src/components/TabBar.tsx` | Tab switcher | 40 |
| `ui/src/components/TubeMapView.tsx` | Tube map container | 200 |
| `ui/src/components/SubsystemNode.tsx` | Custom React Flow node (ported) | 400 |
| `ui/src/components/DependencyEdge.tsx` | Custom React Flow edge (ported) | 170 |
| `ui/src/components/EdgeTooltip.tsx` | Edge hover tooltip (ported) | 90 |
| `ui/src/components/Legend.tsx` | Domain legend (ported) | 130 |
| `ui/src/components/TubeMapSearchBar.tsx` | Search + filters (ported) | 160 |
| `ui/src/components/TubeMapStatusBar.tsx` | Bottom status bar | 80 |
| `ui/src/components/Breadcrumbs.tsx` | Navigation breadcrumbs (ported) | 35 |
| `ui/src/components/SubsystemDrillIn.tsx` | Drill-in entity card grid | 400 |
| `ui/src/components/ManifestLoader.tsx` | Manifest file picker UI | 60 |
| `ui/src/components/CoverageOverlay.tsx` | Match coverage display | 80 |
| `ui/src/components/ShortcutHelp.tsx` | Keyboard shortcut overlay | 60 |
| `ui/src/hooks/useTubeMapState.ts` | Tube map state + IPC | 150 |
| `ui/src/hooks/useTubeLayout.ts` | Memoized layout computation | 80 |
| `ui/src/layout/types.ts` | Layout type definitions | 30 |
| `ui/src/layout/colors.ts` | Domain color assignment | 40 |
| `ui/src/layout/tubeMap.ts` | Dynamic layout algorithm | 300 |
| `src/commands.rs` (extend) | 5 new IPC commands | 200 |
| `core/src/manifest.rs` (extend) | SystemManifest, Connection types | 100 |
| **Total** | | **~2,815** |

---

## 12. Manifest Builder — LLM-Driven Subsystem Discovery

The tube map requires a `system.json` manifest. Today this must be hand-authored. The manifest builder closes that gap: the agent (Claude Code / Codex) reads scan output, proposes subsystems, and writes the manifest. No separate LLM provider needed — the agent IS the LLM.

### 12.1 Workflow

```
Scan → Agent reads entity census → Agent proposes system.json → Validate → Tube map renders
```

```
user> build me a tube map for this repo

claude> [reads skills/domain-scan-init.md]

        Step 1 — Scan + Bootstrap:
        Running: domain-scan scan --root . --output json -o /tmp/scan.json
        Running: domain-scan init --bootstrap -o system.json
        [bootstrap produces: 5 domains, 12 subsystems, 18 connections — rough draft]

        Step 2 — Refine each domain (sub-agents in parallel):
        [spawns sub-agent for "auth" domain: reviews auth-jwt, auth-sessions boundaries]
        [spawns sub-agent for "billing" domain: reviews billing-stripe, billing-plans]
        [spawns sub-agent for "media" domain: reviews media-uploader, media-enrichment]
        ... one sub-agent per domain, each refining names, descriptions, entity placement

        Step 3 — Merge sub-agent proposals, validate:
        Running: domain-scan init --apply-manifest system.json --dry-run
        [shows: "5 domains, 14 subsystems, 22 connections — 91% coverage"]

        Step 4 — Confirm with user (REQUIRED):
        "Here are the proposed subsystems. 3 are marked 'built' — these need your confirmation:
         - auth-jwt (built): 4 interfaces, 2 operations — correct?
         - billing-stripe (built): 3 interfaces — correct?
         - media-uploader (built): 5 interfaces — correct?
        The rest are marked 'new' (unconfirmed). Approve?"

        User: "yes, but move SessionToken from auth-jwt to auth-sessions"

        Step 5 — Apply edits, re-validate:
        [edits system.json, moves SessionToken]
        Running: domain-scan match --manifest system.json --output json --fields coverage_percent
        Coverage: 91% → 92%
        Running: domain-scan init --apply-manifest system.json
        Done.
```

### 12.2 Skill Files

Two skill files teach the agent the full workflow and what good manifests look like:

**`skills/domain-scan-init.md`** — Build/refine manifests:
- **Always start with `--bootstrap`** — never write system.json from scratch
- **Use sub-agents per domain** — each sub-agent focuses on one domain's subsystem boundaries
- **Never auto-confirm `built` status** — `built` means "source code is truth, high confidence". Only the user can mark a subsystem as `built`. The agent proposes `new` for everything, and the user upgrades to `built` after review.
- Naming conventions (kebab-case IDs, verb-first connection labels)
- Grouping principles (independently deployable, one responsibility, 3-8 per domain, schemas anchor subsystems)
- Connection semantics (`depends_on` vs `uses` vs `triggers`)
- Anti-patterns (no utility domains, no duplicates, no test-only connections)

**Critical rule: `built` status requires human confirmation.**
- `--bootstrap` sets all subsystems to `new` by default
- Sub-agents refine boundaries but NEVER upgrade status to `built`
- The agent presents subsystem candidates and asks: "which of these are `built`?"
- Only after explicit user confirmation does the manifest get `status: "built"`
- This is non-negotiable — `built` means the tube map treats source code as authoritative. A wrong `built` label causes the system to skip LLM enrichment on entities that need it.

**`skills/domain-scan-tube-map.md`** — View/interact with tube map data:
- `domain-scan match --manifest system.json` for coverage checking
- `--unmatched-only` to find gaps
- `--prompt-unmatched` to generate prompts for unmapped entities

### 12.3 Skill Bootstrapping

Skills are embedded in the binary via `include_str!` and installed to the **project directory** (not global):

```bash
domain-scan skills install --claude-code   # → .claude/skills/domain-scan-*.md
domain-scan skills install --codex         # → .codex/skills/domain-scan-*.md
domain-scan skills install --dir .cursor/skills/  # custom path

domain-scan skills list                    # list available skills
domain-scan skills show domain-scan-init   # print a skill to stdout
domain-scan skills dump                    # all skills concatenated (for context injection)
```

`--help` includes an `AGENT SKILLS:` section so any agent can self-bootstrap.

### 12.4 Smart Defaults (`--bootstrap`)

Before the agent even proposes anything, heuristics generate a starter manifest:

- **Directory grouping**: top-level `src/` directories → domain candidates, merge dirs sharing >50% imports
- **Import clustering**: entities that import each other heavily → same subsystem
- **Connection inference**: cross-subsystem imports → `depends_on` edges

```bash
domain-scan init --bootstrap -o system.json   # heuristic starter manifest
```

The agent then refines the bootstrap output (better names, better groupings, connection labels).

---

## 13. Manifest Builder Build Phases

### Phase G.1: Smart Defaults (Heuristic) + `--bootstrap`

- [x] Implement `infer_domains_from_directories(index)` — directory grouping heuristic
- [x] Implement `infer_subsystems_from_imports(index, domain)` — import clustering
- [x] Implement `infer_connections_from_imports(index, subsystems)` — cross-subsystem import counting
- [x] Test: scan domain-scan's own codebase → heuristics produce reasonable domains/subsystems
- [x] Test: scan octospark fixtures → heuristics approximate the hand-crafted system.json

### Phase G.2: CLI `domain-scan init`

- [x] Add `init` subcommand with `--bootstrap`, `--apply-manifest`, `-o` flags
- [x] `--bootstrap` generates starter manifest from heuristic defaults
- [x] `--apply-manifest <PATH>` checks a system.json is well-formed:
  - All subsystem IDs unique
  - All `domain` fields reference a key in `domains` map
  - All `connections.from`/`connections.to` reference valid subsystem IDs
  - All `dependencies` reference valid subsystem IDs
  - No orphan domains (defined but no subsystems use them)
  - Outputs structured JSON: `{ valid: bool, errors: [...], warnings: [...] }`
- [x] `domain-scan schema init` dumps the system.json JSON Schema (so agents can validate before writing)
- [x] CLI integration tests with assert_cmd

The agent's workflow for editing manifests is just:
1. Read `system.json` → edit it directly (split, merge, rename, move entities)
2. Run `domain-scan init --validate system.json` → fix any errors
3. Run `domain-scan match --manifest system.json` → check coverage
No special patch API needed — the agent edits JSON files natively.

### Phase G.3: Tauri Wizard UI

- [x] Create `ManifestWizard.tsx` — step navigation, progress indicator
- [x] Create `WizardStepDomains.tsx` — directory census + domain proposal cards
- [x] Create `WizardStepSubsystems.tsx` — per-domain entity mapping
- [x] Create `WizardStepConnections.tsx` — connection list with type/label editing
- [x] Create `WizardStepReview.tsx` — final review + save + tube map preview
- [x] Wire wizard into tube map tab (replaces "Load Manifest" CTA)
- [x] On "Save Manifest" → immediately load into tube map view

### Phase G.4: Agent Skill Files + Bootstrapping

- [x] Create `skills/domain-scan-init.md` with patch guidelines
- [x] Create `skills/domain-scan-tube-map.md`
- [x] Update `skills/domain-scan-scan.md` — add init workflow reference
- [x] Embed skill files in CLI binary via `include_str!`
- [x] Add `domain-scan skills list|show|dump|install` subcommand
- [x] `--claude-code` installs to `.claude/skills/` in project root
- [x] `--codex` installs to `.codex/skills/` in project root
- [x] `--dir <PATH>` for custom install directory
- [x] Auto-add skills directory to `.gitignore`
- [x] Add "AGENT SKILLS" section to `--help` output
- [x] Test: Claude Code can create a manifest from scratch using the skill
- [x] Test: Claude Code can refine an existing manifest via direct system.json edits

**Acceptance criteria (all G phases):**
- `domain-scan init --bootstrap -o system.json` generates a usable starter manifest
- `domain-scan init --apply-manifest system.json --dry-run` shows coverage % and validation errors
- `domain-scan schema init` outputs the JSON Schema for system.json
- `domain-scan skills install --claude-code` writes skills to `.claude/skills/` in project root
- An agent can go from `git clone` to a rendered tube map in under 10 minutes
- The skill file teaches the agent what good manifests look like

---

## 14. Entities/Types Tab — UI Improvements

Fixes and enhancements to the Entities/Types tab based on real-world usage.

### Phase H.1: Monaco Editor for Source Preview

Replace the plain `<pre>` source preview with a Monaco editor (same engine as VS Code) for syntax highlighting, line numbers, and code navigation.

- [x] `npm install @monaco-editor/react` in `ui/`
- [x] Create `MonacoPreview.tsx` — read-only Monaco editor component
  - Read-only mode (no editing)
  - Auto-detect language from file extension
  - Dark theme matching the app (`vs-dark` or custom theme)
  - Scroll to the entity's start line on selection
  - Show the full file content (not just the entity span) with the entity highlighted
- [x] Replace `SourcePreview.tsx` usage in `App.tsx` with `MonacoPreview`
- [x] Update `get_entity_source` IPC command to return full file content (not just byte range)
  - Add new IPC command `get_file_source(file: String) → String` that returns the entire file
  - Keep byte range for highlighting the entity span within the full file
- [x] Handle large files gracefully (>10k lines — lazy loading or truncation warning)
- [x] Minimap enabled for quick navigation within long files
- [x] **Tab bar for open files** — VSCode-style tab strip above the editor
  - Selecting an entity opens its file as a tab (or switches to existing tab if already open)
  - Tabs show the file name (with parent directory for disambiguation, e.g. `auth/provider.ts`)
  - Click a tab to switch to that file; middle-click or × button to close
  - Active tab highlighted; inactive tabs dimmed
  - Maximum ~10 open tabs — oldest auto-closed when limit exceeded (LRU)
  - Selecting a different entity in the same file does NOT open a duplicate tab — just scrolls
  - Tabs persist across entity selections (switching entities doesn't close other tabs)

**Acceptance criteria:**
- Selecting an entity opens the full file in Monaco with syntax highlighting
- The entity's span is highlighted/scrolled to
- Language detection works for TypeScript, Rust, Go, Python, Java, etc.
- No performance degradation for files up to 10k lines
- Multiple files can be open as tabs simultaneously
- Clicking a tab switches the editor to that file
- Closing a tab removes it without affecting other open tabs

### Phase H.2: Monaco Editor E2E Tests

- [x] Test: select entity → Monaco editor renders with syntax highlighting (not plain text)
- [x] Test: select entity → editor scrolls to the entity's start line
- [x] Test: select different entity in same file → editor stays open, scrolls to new position
- [x] Test: select entity in different file → editor loads new file content
- [x] Test: Monaco shows correct language mode (TypeScript for .ts, Rust for .rs, etc.)
- [x] Test: editor is read-only (typing does not modify content)
- [x] Test: selecting entity shows source (center panel is not empty / "Select an entity")
- [x] Test: rapid entity switching (10 entities in 2 seconds) → no crash, editor updates correctly
- [x] Test: large file (1000+ lines) → editor renders without freezing
- [x] Test: minimap visible and reflects file structure

### Phase H.3: Export All — Top-Right Status Bar

Move the Export action from the per-entity details panel to a global "Export All" button in the status bar.

- [x] Remove export dropdown + button from `DetailsPanel.tsx`
- [x] Add "Export All" button to the status bar (next to scan stats)
- [x] Export All exports all currently visible/filtered entities (respects active filters)
- [x] Export output opens in center panel with Copy + Close buttons
- [x] Support JSON, CSV, Markdown formats via dropdown next to the button

**Acceptance criteria:**
- "Export All" button appears in the top-right status bar after a scan
- Clicking it exports all entities matching current filters
- Export respects kind filters (e.g., if "Interfaces" is active, only interfaces exported)

### Phase H.4: Agent Manifest Bootstrap Prompt

The Tube Map "Load Manifest" screen includes a copyable agent prompt that teaches AI agents how to create a `system.json` manifest from scratch using domain-scan CLI commands.

- [x] Replace static `cat > system.json` template with structured agent instructions
- [ ] Agent prompt includes: scan commands, schema reference, validation commands
- [ ] Agent prompt references `domain-scan match --write-back` for iterative refinement
- [ ] Copy button copies the full prompt (not just a preview)
- [ ] Add "What is a manifest?" expandable section explaining the purpose

**Acceptance criteria:**
- Copying the prompt and pasting it into Claude Code / Codex produces a valid system.json
- The prompt uses only commands that exist today (no references to unimplemented `init --bootstrap`)
- The generated manifest passes `domain-scan validate --manifest system.json`

### Phase H.5: Known Bugs

- [ ] **Open Directory button**: Clicking "Open Directory" should open a native folder picker, trigger a scan, and populate the entity tree. Currently requires the capabilities file fix (see Section 2).
- [ ] **Source preview not loading**: Selecting an entity sometimes does not load source code in the center panel. Root cause: `useEffect` dependency array missed `selectedIndex`, causing stale closures. Fixed by adding `tree.selectedIndex` to deps.
- [x] **Editor "No editor available" error**: Bundled macOS apps don't inherit shell PATH, so `cursor`/`code` CLI commands fail. Fixed by falling back to `open -a "Cursor"` / `open -a "Visual Studio Code"` via macOS.
