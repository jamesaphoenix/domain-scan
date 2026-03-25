import { useState } from "react";

const AGENT_PROMPT = `# Bootstrap a system.json manifest for this codebase

You are setting up **domain-scan** — a structural code intelligence tool that
extracts interfaces, services, schemas, and functions via tree-sitter and maps
them to a subsystem manifest. Follow these steps exactly.

---

## Step 1: Install the domain-scan CLI from the latest GitHub release

Fetch the latest release and install the correct binary for this machine:

\`\`\`bash
# Get the latest release tag
LATEST=$(curl -sL https://api.github.com/repos/jamesaphoenix/domain-scan/releases/latest | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\\1/')
echo "Latest release: $LATEST"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
esac

# Download and install
ASSET="domain-scan-cli-\${OS}-\${ARCH}"
curl -sL "https://github.com/jamesaphoenix/domain-scan/releases/download/\${LATEST}/\${ASSET}.tar.gz" -o /tmp/domain-scan.tar.gz
tar -xzf /tmp/domain-scan.tar.gz -C /tmp
chmod +x /tmp/domain-scan-cli
sudo mv /tmp/domain-scan-cli /usr/local/bin/domain-scan
\`\`\`

Verify the install:

\`\`\`bash
domain-scan --version
\`\`\`

If the binary release is not available for this platform, fall back to:

\`\`\`bash
cargo install domain-scan-cli --git https://github.com/jamesaphoenix/domain-scan.git
\`\`\`

## Step 2: Install agent skills

Bootstrap the domain-scan skill files into your AI coding tool:

\`\`\`bash
# For Claude Code:
domain-scan skills install --claude-code

# For Codex:
domain-scan skills install --codex
\`\`\`

This installs structured skill files that teach the agent the full domain-scan
workflow (scanning, matching, manifest refinement, validation).

Verify the installed skills:

\`\`\`bash
domain-scan skills list
\`\`\`

## Step 3: Scan the codebase

\`\`\`bash
domain-scan scan . --output json --fields stats
\`\`\`

Review the stats to understand what was extracted (interfaces, services,
schemas, functions, file count, languages).

## Step 4: Bootstrap a draft manifest

\`\`\`bash
domain-scan init --bootstrap -o system.json
\`\`\`

This runs heuristic analysis on the scan output and generates a draft
system.json with inferred domains, subsystems, and connections.

## Step 5: Validate and refine

\`\`\`bash
# Check coverage
domain-scan match --manifest system.json --fields coverage_percent

# See what's unmatched
domain-scan match --manifest system.json --unmatched-only

# Dry-run write-back (auto-populates interfaces/operations/tables)
domain-scan match --manifest system.json --write-back --dry-run

# Apply write-back
domain-scan match --manifest system.json --write-back
\`\`\`

## Step 6: Iterate until coverage > 90%

For each group of unmatched entities:
1. Check which directory they live in
2. Either expand an existing subsystem's filePath or add a new subsystem
3. Re-run: \`domain-scan match --manifest system.json --fields coverage_percent\`

## Rules
- filePath in each subsystem MUST be a real directory (domain-scan matches by path prefix)
- Use status "new" for all subsystems — only the user upgrades to "built"
- Keep 3-8 subsystems per domain (merge tiny ones, split huge ones)
- No utility/shared/common domains — assign to the domain that owns the concern
- Always \`--dry-run\` before any \`--write-back\`
- Validate after every edit: \`domain-scan init --apply-manifest system.json --dry-run\``;

const PREVIEW_LINES = AGENT_PROMPT.split("\n").slice(0, 14).join("\n") + "\n...";

interface ManifestLoaderProps {
  onLoadManifest: () => void;
  loading: boolean;
  error: string | null;
  onStartWizard?: () => void;
}

export function ManifestLoader({
  onLoadManifest,
  loading,
  error,
  onStartWizard,
}: ManifestLoaderProps) {
  const [copied, setCopied] = useState(false);
  const [showFullPrompt, setShowFullPrompt] = useState(false);
  const [manifestInfoOpen, setManifestInfoOpen] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(AGENT_PROMPT);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="flex-1 flex items-center justify-center overflow-y-auto">
      <div className="text-center max-w-lg py-8">
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

        <h2 className="text-lg font-semibold text-slate-200 mb-2">
          Subsystem Tube Map
        </h2>
        <p className="text-sm text-slate-400 mb-4 leading-relaxed">
          Visualize your codebase as a tube map of domains, subsystems, and
          connections. Use an AI agent to generate a manifest, or load an
          existing one.
        </p>

        {/* Recommended: Agent prompt section */}
        <div className="mb-6 text-left">
          <div className="flex items-center gap-2 mb-2">
            <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">
              Recommended
            </span>
            <span className="text-xs text-slate-500">
              Copy this prompt into Claude Code, Codex, or any AI agent:
            </span>
          </div>
          <div className="relative">
            <pre className="text-left text-[11px] leading-relaxed bg-slate-900/80 border border-blue-500/30 rounded-lg px-4 py-3 text-slate-400 overflow-y-auto max-h-52 scrollbar-thin whitespace-pre-wrap">
              <code>{showFullPrompt ? AGENT_PROMPT : PREVIEW_LINES}</code>
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
                className="px-3 py-1.5 rounded text-xs font-medium
                           bg-blue-600 hover:bg-blue-500 text-white
                           transition-colors duration-150"
              >
                {copied ? "Copied!" : "Copy prompt"}
              </button>
            </div>
          </div>
          <p className="text-[11px] text-slate-600 mt-2">
            The agent will install the CLI from the{" "}
            <span className="text-slate-400">latest GitHub release</span>,
            bootstrap skills, scan your code, and generate a real manifest.
          </p>
        </div>

        {/* Divider */}
        <div className="flex items-center gap-3 mb-4">
          <div className="flex-1 border-t border-slate-700/50" />
          <span className="text-xs text-slate-600">
            already have a manifest?
          </span>
          <div className="flex-1 border-t border-slate-700/50" />
        </div>

        {/* Load existing manifest */}
        <div className="flex items-center gap-3 justify-center">
          <button
            onClick={onLoadManifest}
            disabled={loading}
            className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg
                       bg-blue-600 hover:bg-blue-500 disabled:bg-blue-600/50
                       text-white text-sm font-medium
                       transition-colors duration-150"
          >
            {loading ? "Loading..." : "Load Manifest"}
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
