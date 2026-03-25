import { useState } from "react";

const AGENT_PROMPT = `# Generate a system.json manifest for this codebase

## Prerequisites

Install domain-scan (if not already installed):
  cargo install domain-scan-cli

## Step 1: Scan the codebase

Run domain-scan to extract all interfaces, services, schemas, and functions:

  domain-scan scan . --output json > scan-output.json
  domain-scan interfaces . --output json
  domain-scan schemas . --output json

Review the output to understand what entities exist and where they live.

## Step 2: Get the manifest schema

Dump the JSON schema so you know exactly what fields are required:

  domain-scan schema init

Use this schema as the authoritative reference for system.json structure.

## Step 3: Identify domains and subsystems

From the scan output, identify:
- **Domains**: The 3-7 major logical groupings (e.g. "auth", "billing", "api", "media")
- **Subsystems**: Concrete modules within each domain. Use the directory structure
  and file paths from the scan to determine boundaries.

## Step 4: Write system.json

Create system.json with this structure:

{
  "meta": { "name": "<project>", "version": "0.1", "description": "<one line>" },
  "domains": {
    "<domain-id>": { "label": "<Display Name>", "color": "<hex>" }
  },
  "subsystems": [
    {
      "id": "<kebab-case-id>",
      "name": "<Display Name>",
      "domain": "<domain-id>",
      "status": "new",
      "filePath": "<real directory path from scan output>",
      "description": "<one sentence>",
      "interfaces": ["PascalCaseNames from scan"],
      "operations": ["camelCase() methods from scan"],
      "tables": ["snake_case schema/table names from scan"],
      "events": [],
      "dependencies": ["<other-subsystem-ids>"]
    }
  ],
  "connections": [
    { "from": "<id>", "to": "<id>", "label": "<why>", "type": "depends_on" }
  ]
}

## Rules
- filePath MUST be a real directory in the codebase (domain-scan matches entities by path prefix)
- Populate interfaces/operations/tables from the actual domain-scan output, not guesses
- Every id in dependencies and connections must reference an existing subsystem
- Use status "new" for all subsystems — only the user can upgrade to "built" after review
- Color palette: #3b82f6 #8b5cf6 #22c55e #f97316 #ef4444 #eab308 #06b6d4 #ec4899

## Step 5: Validate

  domain-scan match --manifest system.json
  domain-scan match --manifest system.json --unmatched-only

Fix any unmatched entities by adjusting filePath or adding missing subsystems.

## Step 6: Iterative refinement with --write-back

After validation, use --write-back to automatically populate matched entities into
your manifest (adding discovered interfaces, operations, and tables):

  domain-scan match --manifest system.json --write-back --dry-run
  domain-scan match --manifest system.json --write-back

The --dry-run flag previews changes without writing. Once satisfied, run without
--dry-run to update system.json in place. Re-run match to verify coverage:

  domain-scan match --manifest system.json --fields coverage_percent

Repeat steps 5-6 until coverage is satisfactory. Focus on unmatched entities:

  domain-scan match --manifest system.json --unmatched-only

For each unmatched group, either adjust an existing subsystem's filePath or add a
new subsystem to cover them.`;

const PREVIEW_LINES = AGENT_PROMPT.split("\n").slice(0, 12).join("\n") + "\n...";

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
        <p className="text-sm text-slate-400 mb-6 leading-relaxed">
          Load a system manifest (JSON) to visualize your subsystems as a tube
          map. The manifest defines domains, subsystems, and their connections.
        </p>

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
          {onStartWizard && (
            <>
              <span className="text-xs text-slate-600">or</span>
              <button
                onClick={onStartWizard}
                className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg
                           border border-slate-600 hover:border-slate-500
                           text-slate-300 hover:text-white text-sm font-medium
                           transition-colors duration-150"
              >
                Create with Wizard
              </button>
            </>
          )}
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
                A <strong className="text-slate-300">system manifest</strong> (<code className="text-slate-300">system.json</code>) describes
                your codebase's architecture as a graph of <strong className="text-slate-300">domains</strong>, <strong className="text-slate-300">subsystems</strong>,
                and <strong className="text-slate-300">connections</strong>.
              </p>
              <ul className="list-disc list-inside space-y-1 text-slate-500">
                <li><strong className="text-slate-400">Domains</strong> are high-level groupings (e.g. "Auth", "Billing", "API"). Each domain gets a unique color on the tube map.</li>
                <li><strong className="text-slate-400">Subsystems</strong> are concrete modules within a domain (e.g. "auth-jwt", "billing-stripe"). Each subsystem maps to a directory and its extracted interfaces, schemas, and operations.</li>
                <li><strong className="text-slate-400">Connections</strong> describe dependencies between subsystems (e.g. "billing-stripe depends_on auth-jwt").</li>
              </ul>
              <p>
                domain-scan matches entities from your codebase to subsystems by file path prefix. The tube map then visualizes this mapping — domains as colored lines, subsystems as stations, and connections as edges.
              </p>
              <p className="text-slate-500">
                You can create a manifest manually, use the built-in wizard, or copy the agent prompt below and let an AI agent generate one from your scan output.
              </p>
            </div>
          )}
        </div>

        <div className="mt-6 pt-6 border-t border-slate-700/50">
          <p className="text-xs text-slate-500 mb-2">
            Don't have a manifest? Copy this prompt and give it to Claude Code, Codex, or any AI agent:
          </p>
          <div className="relative">
            <pre className="text-left text-[11px] leading-relaxed bg-slate-900/80 border border-slate-700/50 rounded-lg px-4 py-3 text-slate-400 overflow-y-auto max-h-52 scrollbar-thin whitespace-pre-wrap">
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
                {copied ? "Copied!" : "Copy full prompt"}
              </button>
            </div>
          </div>
          <p className="text-[11px] text-slate-600 mt-2.5">
            The agent will run <span className="text-slate-400 font-mono">domain-scan scan .</span> to analyze your code, then generate a real manifest based on your actual interfaces, schemas, and file structure.
          </p>
        </div>
      </div>
    </div>
  );
}
