import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Types for the Rust IPC response
// ---------------------------------------------------------------------------

interface ReleaseAsset {
  name: string;
  download_url: string;
  size: number;
}

interface PlatformReleaseInfo {
  os: string;
  arch: string;
  latest_tag: string | null;
  assets: ReleaseAsset[];
  matching_asset: ReleaseAsset | null;
  cargo_install_cmd: string;
  recommended_install_cmd: string;
  recommended_update_cmd: string;
  scanned_root: string | null;
  installed_path: string | null;
  installed_version: string | null;
  doctor_supported: boolean;
  update_available: boolean | null;
}

// ---------------------------------------------------------------------------
// Dynamic prompt builder
// ---------------------------------------------------------------------------

function buildAgentPrompt(info: PlatformReleaseInfo | null): string {
  // Install section — adapt to detected platform + available release
  let installSection: string;

  if (info?.installed_path && info.doctor_supported) {
    const installedVersion = info.installed_version
      ? ` (version ${info.installed_version})`
      : "";
    const updateLine = info.update_available === false
      ? "If `update_available` is `false`, keep the existing install and continue to Step 2."
      : "If `update_available` is `true`, update the CLI before continuing to Step 2.";
    installSection = `## Step 1 — Check the installed domain-scan CLI (${info.os}/${info.arch})

\`domain-scan\` is already installed at \`${info.installed_path}\`${installedVersion}.
Start by inspecting the existing install instead of reinstalling blindly:

\`\`\`bash
domain-scan doctor --output json
\`\`\`

${updateLine}

If an update is needed, run:

\`\`\`bash
${info.recommended_update_cmd}

# Verify
domain-scan doctor --output json
\`\`\``;
  } else if (info?.installed_path) {
    const installedVersion = info.installed_version
      ? ` (version ${info.installed_version})`
      : "";
    installSection = `## Step 1 — Upgrade the installed domain-scan CLI (${info.os}/${info.arch})

\`domain-scan\` is already installed at \`${info.installed_path}\`${installedVersion}, but this
install predates the \`doctor\` command. Upgrade it first so the rest of the
workflow can use \`domain-scan doctor --output json\` deterministically:

\`\`\`bash
${info.recommended_update_cmd}

# Verify
domain-scan doctor --output json
\`\`\``;
  } else if (info?.matching_asset) {
    const a = info.matching_asset;
    const tag = info.latest_tag ?? "latest";
    installSection = `## Step 1 — Install the domain-scan CLI (${info.os}/${info.arch})

A pre-built binary is available for your platform. Download and install it:

\`\`\`bash
# Download ${a.name} (${tag}, ${(a.size / 1024 / 1024).toFixed(1)} MB)
${info.recommended_install_cmd}

# Verify
domain-scan doctor --output json
\`\`\`

If the download fails, fall back to building from source:

\`\`\`bash
${info.cargo_install_cmd}
\`\`\``;
  } else if (info) {
    // We know the platform but no matching asset — list what IS available
    const assetList = info.assets.length > 0
      ? `Available release assets (none match ${info.os}/${info.arch}):\n${info.assets.map(a => `  - ${a.name}`).join("\n")}\n\n`
      : "";
    installSection = `## Step 1 — Install the domain-scan CLI (${info.os}/${info.arch})

${assetList}No pre-built binary is available for your platform. Build from source:

\`\`\`bash
${info.cargo_install_cmd}

# Verify
domain-scan doctor --output json
\`\`\``;
  } else {
    // Couldn't detect anything — generic instructions
    installSection = `## Step 1 — Install the domain-scan CLI

Check for a pre-built binary at:
  https://github.com/jamesaphoenix/domain-scan/releases/latest

Download the archive matching your OS and architecture, extract it, and move
the \`domain-scan\` binary to somewhere on your PATH (e.g. ~/.local/bin/).

If no binary matches, build from source:

\`\`\`bash
cargo install --force domain-scan-cli --git https://github.com/jamesaphoenix/domain-scan.git

# Verify
domain-scan doctor --output json
\`\`\``;
  }

  return `# Build a system.json manifest for this codebase

You are an AI agent (Claude Code, Codex, or similar) tasked with generating a
**system.json manifest** for the codebase you are currently inside.

**domain-scan** is a structural code intelligence CLI. It uses tree-sitter to
extract every interface, service, class, function, schema, and type alias from
source code — then maps them to a manifest of domains, subsystems, and
connections. The manifest powers a "tube map" visualization of the codebase's
architecture.

Your job: install the CLI, install agent skills, scan the codebase, bootstrap a
starter manifest, and then improve it aggressively until coverage exceeds 90%.
The refinement work matters more than the bootstrap: merge duplicates, reorder
domains, tighten or widen subsystem boundaries, and clean up noisy connections.
You will use 5-10 parallel sub-agents depending on codebase size.
${info?.scanned_root ? `
## Project directory

All commands below MUST be run from this absolute path:

\`\`\`
${info.scanned_root}
\`\`\`

Write \`system.json\` to: \`${info.scanned_root}/system.json\`
Run all \`domain-scan\` commands with: \`--root ${info.scanned_root}\`
` : ""}
---

${installSection}

---

## Step 2 — Install agent skills for your coding tool

domain-scan ships embedded skill files that teach AI agents the full workflow.
Install them for **every** AI tool the user might be using:

\`\`\`bash
# Install for Claude Code (writes to .claude/skills/)
domain-scan skills install --claude-code

# Install for Codex (writes to .codex/skills/)
domain-scan skills install --codex

# Or install to a custom directory
# domain-scan skills install --dir /path/to/skills
\`\`\`

Verify the skills were installed:

\`\`\`bash
domain-scan skills list
\`\`\`

You should see skills like: domain-scan-cli, domain-scan-scan, domain-scan-init,
domain-scan-match, domain-scan-query, domain-scan-validate, domain-scan-prompt,
domain-scan-tube-map, domain-scan-cache, domain-scan-safety, domain-scan-schema.

These skills contain detailed workflows and rules. Read \`domain-scan-init\`,
\`domain-scan-validate\`, and \`domain-scan-match\` before proceeding — they
contain critical constraints:

\`\`\`bash
domain-scan skills show domain-scan-init
domain-scan skills show domain-scan-validate
domain-scan skills show domain-scan-match
\`\`\`

---

## Step 3 — Scan the codebase

Run domain-scan to extract all structural entities:

\`\`\`bash
domain-scan scan . --output json --fields stats
\`\`\`

This prints a JSON stats summary. Note the key numbers:
- **total_files**: How many source files were parsed
- **total_interfaces / total_services / total_schemas**: Entity counts
- **languages**: Which languages were detected
- **parse_duration_ms**: How long parsing took

> **Scaling rule**: Use these stats to decide how many sub-agents to spawn:
> - < 100 files → 5 sub-agents
> - 100-500 files → 6 sub-agents
> - 500-1000 files → 7 sub-agents
> - 1000-2000 files → 8 sub-agents
> - 2000+ files → 10 sub-agents

To see the full entity list (not just stats):

\`\`\`bash
domain-scan scan . --output json > /tmp/scan-output.json
domain-scan interfaces . --output json
domain-scan services . --output json
domain-scan schemas . --output json
\`\`\`

---

## Step 4 — Understand the manifest schema

Before writing system.json, dump the schema so you know exactly what fields
exist and which are required:

\`\`\`bash
domain-scan schema init
\`\`\`

The manifest has this structure:

\`\`\`json
{
  "meta": {
    "name": "<project-name>",
    "version": "1.0.0",
    "description": "<one-sentence description>"
  },
  "domains": {
    "<domain-id>": { "label": "<Display Name>", "color": "<hex>" }
  },
  "subsystems": [
    {
      "id": "<kebab-case-id>",
      "name": "<Display Name>",
      "domain": "<domain-id>",
      "status": "new",
      "filePath": "<real-directory-path-from-scan>",
      "description": "<one sentence describing what this subsystem does>",
      "interfaces": ["PascalCaseNames"],
      "operations": ["camelCaseMethods"],
      "tables": ["snake_case_schemas"],
      "events": [],
      "dependencies": ["<other-subsystem-ids>"],
      "children": [
        {
          "id": "<child-kebab-case-id>",
          "name": "<Child Display Name>",
          "domain": "<same-domain-id>",
          "status": "new",
          "filePath": "<more-specific-child-path>",
          "interfaces": [],
          "operations": [],
          "tables": [],
          "events": [],
          "dependencies": []
        }
      ]
    }
  ],
  "connections": [
    {
      "from": "<subsystem-id>",
      "to": "<subsystem-id>",
      "label": "<verb phrase: e.g. authenticates-via>",
      "type": "depends_on"
    }
  ]
}
\`\`\`

---

## Step 5 — Bootstrap then Refine the manifest

Always start with \`--bootstrap\`. It gives the agent a usable first draft based
on directory structure and import analysis:

\`\`\`bash
domain-scan init --bootstrap --name "<project-name>" -o system.json
\`\`\`

Do not stop at the bootstrap output. Review domains, subsystems, and
connections, then improve them:

### 5a. Refine domains (3-7 max)

- Group related subsystems by **business concern**, not file location
- Good domains: "auth", "billing", "media", "api", "data-pipeline", "notifications"
- Bad domains: "utils", "shared", "common", "lib", "src" (these are not domains)
- Each domain gets a unique color. Use this palette:
  \`#3b82f6 #8b5cf6 #22c55e #f97316 #ef4444 #eab308 #06b6d4 #ec4899\`

### 5b. Refine subsystems (3-8 per domain)

- Each subsystem MUST map to a **real directory** in the codebase
- Use the scan output to verify paths: \`domain-scan scan . --output json\`
- Subsystem IDs must be kebab-case: "auth-jwt", "billing-stripe", "api-rest"
- Merge tiny subsystems (< 3 entities) into a parent
- De-dupe overlapping subsystems and widen or narrow \`filePath\` boundaries when bootstrap cut them badly

### 5c. Populate entity arrays

Use \`--write-back\` (Step 7) to auto-populate from matched results. To verify
entity names manually:
\`\`\`bash
domain-scan interfaces . --output json --fields name,file
\`\`\`
Do not guess or hallucinate entity names.

### 5d. Define connections

- Use the import graph: if subsystem A imports from subsystem B, add a connection
- Label with a verb phrase: "authenticates-via", "persists-to", "reads-from"
- Remove redundant or low-signal connections if bootstrap added too many
- Cap at 3-5 connections per subsystem to keep the tube map readable

### 5e. Write system.json

Write the complete manifest to \`system.json\` in the repository root.

---

## Step 6 — Validate the manifest

After writing system.json, validate it immediately:

\`\`\`bash
# Check semantic manifest integrity
domain-scan validate --manifest system.json --output json

# Run matching — this maps scanned entities to your subsystems
domain-scan match --manifest system.json

# Check coverage percentage
domain-scan match --manifest system.json --fields coverage_percent

# List unmatched entities (these need to be assigned to subsystems)
domain-scan match --manifest system.json --unmatched-only
\`\`\`

---

## Step 7 — Write-back and refine

Use \`--write-back\` to auto-populate entity arrays from matched results:

\`\`\`bash
# Preview what will change
domain-scan match --manifest system.json --write-back --dry-run

# Apply the write-back
domain-scan match --manifest system.json --write-back
\`\`\`

Re-check coverage:

\`\`\`bash
domain-scan match --manifest system.json --fields coverage_percent
\`\`\`

---

## Step 8 — Iterate until coverage > 90%

For each batch of unmatched entities:

1. Run: \`domain-scan match --manifest system.json --unmatched-only\`
2. Group unmatched entities by their file directory
3. For each group, decide:
   - **Expand**: Widen an existing subsystem's \`filePath\` to cover the directory
   - **Create**: Add a new subsystem if the group represents a distinct concern
   - **Merge**: If only 1-2 entities, merge into the nearest existing subsystem
4. Edit system.json accordingly. Prefer patching the bootstrap output over rebuilding whole sections.
5. Re-validate: \`domain-scan validate --manifest system.json --output json\`
6. Re-match: \`domain-scan match --manifest system.json --fields coverage_percent\`
7. Repeat until coverage > 90%

---

## Sub-agent orchestration

When working in parallel, assign each sub-agent a domain or set of domains:

- **Sub-agent 1**: Install CLI + skills (Step 1-2). Scan codebase (Step 3).
- **Sub-agent 2-N**: Each takes 1-2 domains. Analyzes scan output for that domain's
  directories, identifies subsystems, populates entity arrays, defines connections.
- **Coordinator**: Merges all sub-agent outputs into one system.json. Resolves
  cross-domain connections. Runs validation + matching. Iterates on unmatched.

---

## Hard rules (do not violate)

1. **filePath must be a real directory** — verify with \`ls\` before adding
2. **Entity names must come from scan output** — never guess or hallucinate names
3. **Status is always "new"** — only the human user upgrades to "built"
4. **Always --dry-run before --write-back** — never write without previewing
5. **No utility domains** — "utils", "shared", "common", "helpers" are not domains.
   Assign each entity to the domain that owns the concern.
6. **3-8 subsystems per domain** — fewer means you're too coarse, more means you're
   splitting unnecessarily
7. **kebab-case IDs only** — "auth-jwt" not "authJwt" or "auth_jwt"
8. **Verb-first connection labels** — "authenticates-via" not "auth connection"
9. **Cap connections** — max 3-5 outgoing connections per subsystem
10. **Validate after every edit** — run \`domain-scan validate --manifest system.json --output json\`
    after every change to system.json
11. **Write system.json to the project root**${info?.scanned_root ? ` — \`${info.scanned_root}/system.json\`` : " — not /tmp, not a subdirectory"}
12. **Install skills for BOTH Claude Code and Codex** — the user may use either`;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface ManifestLoaderProps {
  onLoadManifest: () => void;
  onOpenDirectory?: () => void;
  loading: boolean;
  scanLoaded?: boolean | null;
  openDirectoryLoading?: boolean;
  error: string | null;
  onStartWizard?: () => void;
}

export function ManifestLoader({
  onLoadManifest,
  onOpenDirectory,
  loading,
  scanLoaded = null,
  openDirectoryLoading = false,
  error,
  onStartWizard,
}: ManifestLoaderProps) {
  const [copied, setCopied] = useState(false);
  const [showFullPrompt, setShowFullPrompt] = useState(false);
  const [manifestInfoOpen, setManifestInfoOpen] = useState(false);
  const [releaseInfo, setReleaseInfo] = useState<PlatformReleaseInfo | null>(
    null,
  );
  const [releaseLoading, setReleaseLoading] = useState(true);

  // Fetch platform + release info on mount
  useEffect(() => {
    let cancelled = false;
    invoke<PlatformReleaseInfo>("get_platform_release_info")
      .then((info) => {
        if (!cancelled) setReleaseInfo(info);
      })
      .catch(() => {
        // Offline or API error — prompt will use generic fallback
      })
      .finally(() => {
        if (!cancelled) setReleaseLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const agentPrompt = buildAgentPrompt(releaseInfo);
  const previewLines =
    agentPrompt.split("\n").slice(0, 16).join("\n") + "\n...";

  const handleCopy = async () => {
    await navigator.clipboard.writeText(agentPrompt);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Release badge text
  const releaseBadge = releaseInfo?.latest_tag
    ? `${releaseInfo.latest_tag} — ${releaseInfo.os}/${releaseInfo.arch}`
    : releaseLoading
      ? "detecting platform..."
      : "offline";

  const hasMatchingBinary = !!releaseInfo?.matching_asset;
  const hasInstalledCli = !!releaseInfo?.installed_path;
  const installSummary = hasInstalledCli
    ? releaseInfo?.doctor_supported
      ? releaseInfo?.update_available === false
        ? `Existing CLI detected at ${releaseInfo.installed_path}. The prompt starts with domain-scan doctor and keeps the current install if it is already up to date.`
        : `Existing CLI detected at ${releaseInfo?.installed_path}. The prompt starts with domain-scan doctor and upgrades the CLI first when an update is available.`
      : `Existing CLI detected at ${releaseInfo?.installed_path}, but it predates the doctor command. The prompt upgrades it first, then switches to domain-scan doctor.`
    : hasMatchingBinary
      ? `The prompt includes a direct download path for ${releaseInfo?.matching_asset?.name}. The agent will install the CLI, bootstrap skills for both Claude Code and Codex, scan your code, bootstrap a starter manifest, and then refine it using 5-10 parallel sub-agents.`
      : "The agent will build the CLI from source, bootstrap skills, scan your code, bootstrap a starter manifest, and then refine it using 5-10 parallel sub-agents.";

  return (
    <div className="flex-1 flex items-center justify-center overflow-y-auto">
      <div className="text-center max-w-2xl py-8 px-4">
        <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-blue-500/20 to-purple-500/20 border border-slate-700/50 flex items-center justify-center">
          <svg
            width="32"
            height="32"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-slate-400"
          >
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="12" y1="18" x2="12" y2="12" />
            <line x1="9" y1="15" x2="15" y2="15" />
          </svg>
        </div>

        <h2 className="text-lg font-semibold text-slate-200 mb-1">
          Subsystem Tube Map
        </h2>
        <p className="text-sm text-slate-400 mb-4 leading-relaxed">
          Visualize your codebase architecture as a tube map. Use an AI agent to
          bootstrap and refine a manifest, or load an existing one.
        </p>

        {scanLoaded === false && (
          <p className="text-xs text-slate-500 mb-4 leading-relaxed">
            No scan is loaded yet. Open a directory to scan the codebase first,
            or load an existing manifest directly if you already have one.
          </p>
        )}

        {/* Release + Platform badge */}
        <div className="flex items-center justify-center gap-2 mb-4">
          <span
            className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium border ${
              hasMatchingBinary
                ? "bg-green-950/40 border-green-700/40 text-green-400"
                : releaseLoading
                  ? "bg-slate-800/50 border-slate-700/40 text-slate-500"
                  : "bg-yellow-950/40 border-yellow-700/40 text-yellow-400"
            }`}
          >
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                hasMatchingBinary
                  ? "bg-green-400"
                  : releaseLoading
                    ? "bg-slate-500 animate-pulse"
                    : "bg-yellow-400"
              }`}
            />
            {releaseBadge}
          </span>
          {hasMatchingBinary && (
            <span className="text-[10px] text-slate-500">
              binary available
            </span>
          )}
          {hasInstalledCli && (
            <span className="text-[10px] text-slate-500">
              installed locally
            </span>
          )}
        </div>

        {/* Recommended: Agent prompt section */}
        <div className="mb-6 text-left">
          <div className="flex items-center gap-2 mb-2">
            <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">
              Recommended
            </span>
            <span className="text-xs text-slate-500">
              Copy this prompt into Claude Code, Codex, or any AI agent
            </span>
          </div>
          <div className="relative">
            <pre className="text-left text-[11px] leading-relaxed bg-slate-900/80 border border-blue-500/30 rounded-lg px-4 py-3 text-slate-400 overflow-y-auto max-h-64 scrollbar-thin whitespace-pre-wrap">
              <code>{showFullPrompt ? agentPrompt : previewLines}</code>
            </pre>
            <div className="absolute top-2 right-2 flex items-center gap-1.5">
              <button
                onClick={() => setShowFullPrompt(!showFullPrompt)}
                className="px-2.5 py-1.5 rounded text-xs font-medium
                           border border-slate-600 hover:border-slate-500
                           text-slate-300 hover:text-white bg-slate-800
                           transition-colors duration-150"
              >
                {showFullPrompt ? "Collapse" : "Expand"}
              </button>
              <button
                onClick={handleCopy}
                disabled={releaseLoading}
                className="px-3 py-1.5 rounded text-xs font-medium
                           bg-blue-600 hover:bg-blue-500 disabled:bg-blue-600/50
                           text-white transition-colors duration-150"
              >
                {copied ? "Copied!" : "Copy prompt"}
              </button>
            </div>
          </div>
          <p className="text-[11px] text-slate-600 mt-2">
            {installSummary}
          </p>
        </div>

        <div className="flex items-center gap-3 mb-4">
          <div className="flex-1 border-t border-slate-700/50" />
          <span className="text-xs text-slate-600">
            choose your starting point
          </span>
          <div className="flex-1 border-t border-slate-700/50" />
        </div>

        <div className="flex items-center gap-3 justify-center flex-wrap">
          {onOpenDirectory && (
            <button
              onClick={onOpenDirectory}
              disabled={openDirectoryLoading}
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg
                         bg-slate-700 hover:bg-slate-600 disabled:bg-slate-700/50
                         text-white text-sm font-medium
                         transition-colors duration-150"
            >
              {openDirectoryLoading ? "Scanning..." : "Open Directory"}
            </button>
          )}
          <button
            onClick={onLoadManifest}
            disabled={loading}
            className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg
                       bg-blue-600 hover:bg-blue-500 disabled:bg-blue-600/50
                       text-white text-sm font-medium
                       transition-colors duration-150"
          >
            {loading ? "Loading..." : "Open Manifest"}
          </button>
        </div>

        {error && (
          <p className="mt-4 text-xs text-red-400 bg-red-950/50 border border-red-800/50 rounded-md px-3 py-2">
            {error}
          </p>
        )}

        {/* What is a manifest? — expandable section */}
        <div className="mt-6 text-left">
          <button
            onClick={() => setManifestInfoOpen(!manifestInfoOpen)}
            className="flex items-center gap-2 text-xs text-slate-400 hover:text-slate-300 transition-colors duration-150"
          >
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              className={`transition-transform duration-150 ${manifestInfoOpen ? "rotate-90" : ""}`}
            >
              <path d="M4 2l4 4-4 4" />
            </svg>
            What is a manifest?
          </button>
          {manifestInfoOpen && (
            <div className="mt-2 text-left text-[11px] leading-relaxed text-slate-400 bg-slate-900/60 border border-slate-700/50 rounded-lg px-4 py-3 space-y-2">
              <p>
                A{" "}
                <strong className="text-slate-300">system manifest</strong> (
                <code className="text-slate-300">system.json</code>) describes
                your codebase's architecture as a graph of{" "}
                <strong className="text-slate-300">domains</strong>,{" "}
                <strong className="text-slate-300">subsystems</strong>, and{" "}
                <strong className="text-slate-300">connections</strong>.
              </p>
              <ul className="list-disc list-inside space-y-1 text-slate-500">
                <li>
                  <strong className="text-slate-400">Domains</strong> are
                  high-level groupings (e.g. "Auth", "Billing", "API"). Each
                  domain gets a unique color on the tube map.
                </li>
                <li>
                  <strong className="text-slate-400">Subsystems</strong> are
                  concrete modules within a domain (e.g. "auth-jwt",
                  "billing-stripe"). Each subsystem maps to a directory and its
                  extracted interfaces, schemas, and operations.
                </li>
                <li>
                  <strong className="text-slate-400">Connections</strong>{" "}
                  describe dependencies between subsystems (e.g.
                  "billing-stripe depends_on auth-jwt").
                </li>
              </ul>
              <p>
                domain-scan matches entities from your codebase to subsystems by
                file path prefix. The tube map then visualizes this mapping —
                domains as colored lines, subsystems as stations, and
                connections as edges.
              </p>
            </div>
          )}
        </div>

        {/* Create with Wizard — tertiary option at the bottom */}
        {onStartWizard && (
          <div className="mt-4 pt-4 border-t border-slate-700/50">
            <button
              onClick={onStartWizard}
              className="text-xs text-slate-500 hover:text-slate-300 transition-colors duration-150"
            >
              or create manually with the wizard
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
