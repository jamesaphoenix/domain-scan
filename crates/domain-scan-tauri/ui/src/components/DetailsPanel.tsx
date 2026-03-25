import type {
  EntitySummary,
  Entity,
  BuildStatus,
  Confidence,
} from "../types";

interface DetailsPanelProps {
  entity: EntitySummary | null;
  detail: Entity | null;
  onOpenInEditor: (file: string, line: number) => void;
}

const STATUS_LABELS: Record<BuildStatus, { label: string; color: string }> = {
  built: { label: "Built", color: "text-green-400" },
  unbuilt: { label: "Unbuilt", color: "text-yellow-400" },
  error: { label: "Error", color: "text-red-400" },
  rebuild: { label: "Rebuild", color: "text-orange-400" },
};

const CONFIDENCE_LABELS: Record<
  Confidence,
  { label: string; color: string }
> = {
  high: { label: "High", color: "text-green-400" },
  medium: { label: "Medium", color: "text-yellow-400" },
  low: { label: "Low", color: "text-red-400" },
};

export function DetailsPanel({
  entity,
  detail,
  onOpenInEditor,
}: DetailsPanelProps) {
  if (!entity) {
    return (
      <div className="flex items-center justify-center h-full text-gray-600 text-sm">
        Select an entity to view details
      </div>
    );
  }

  const statusInfo = STATUS_LABELS[entity.build_status];
  const confidenceInfo = CONFIDENCE_LABELS[entity.confidence];
  const isNonBuilt = entity.build_status !== "built";

  return (
    <div className="text-sm space-y-4">
      {/* Warning banner for non-Built entities */}
      {isNonBuilt && (
        <div className="bg-yellow-900/30 border border-yellow-700/50 rounded px-3 py-2 text-xs text-yellow-300">
          This module does not currently build. Extracted entities are
          best-effort. Use &quot;Generate Prompt&quot; to dispatch LLM agents
          for enrichment.
        </div>
      )}

      {/* Entity header */}
      <div>
        <h2 className="text-base font-semibold text-white">{entity.name}</h2>
        <span className="text-xs text-gray-500 capitalize">{entity.kind}</span>
      </div>

      {/* Metadata */}
      <div className="space-y-2 text-xs">
        <MetaRow label="Build Status">
          <span className={statusInfo.color}>{statusInfo.label}</span>
        </MetaRow>
        <MetaRow label="Confidence">
          <span className={confidenceInfo.color}>{confidenceInfo.label}</span>
        </MetaRow>
        <MetaRow label="Language">
          <span className="text-gray-300">{entity.language}</span>
        </MetaRow>
        <MetaRow label="File">
          <button
            className="text-blue-400 hover:text-blue-300 truncate text-left"
            onClick={() => onOpenInEditor(entity.file, entity.line)}
            title={entity.file}
          >
            {shortenPath(entity.file)}:{entity.line}
          </button>
        </MetaRow>

        {/* Detail-specific metadata */}
        {detail && <DetailMeta detail={detail} />}
      </div>

    </div>
  );
}

function MetaRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex justify-between items-center">
      <span className="text-gray-500">{label}</span>
      {children}
    </div>
  );
}

function DetailMeta({ detail }: { detail: Entity }) {
  if ("Interface" in detail) {
    const iface = detail.Interface;
    return (
      <>
        {iface.extends.length > 0 && (
          <MetaRow label="Extends">
            <span className="text-gray-300">{iface.extends.join(", ")}</span>
          </MetaRow>
        )}
        {iface.generics.length > 0 && (
          <MetaRow label="Generics">
            <span className="text-gray-300">
              {"<"}
              {iface.generics.join(", ")}
              {">"}
            </span>
          </MetaRow>
        )}
        <MetaRow label="Methods">
          <span className="text-gray-300">{iface.methods.length}</span>
        </MetaRow>
        <MetaRow label="Properties">
          <span className="text-gray-300">{iface.properties.length}</span>
        </MetaRow>
        <MetaRow label="Kind">
          <span className="text-gray-300 capitalize">
            {iface.language_kind}
          </span>
        </MetaRow>
        {iface.decorators.length > 0 && (
          <MetaRow label="Decorators">
            <span className="text-gray-300">
              {iface.decorators.join(", ")}
            </span>
          </MetaRow>
        )}
      </>
    );
  }

  if ("Service" in detail) {
    const svc = detail.Service;
    return (
      <>
        <MetaRow label="Kind">
          <span className="text-gray-300 capitalize">
            {typeof svc.kind === "string"
              ? svc.kind.replace(/_/g, " ")
              : `custom: ${svc.kind.custom}`}
          </span>
        </MetaRow>
        <MetaRow label="Methods">
          <span className="text-gray-300">{svc.methods.length}</span>
        </MetaRow>
        {svc.routes.length > 0 && (
          <MetaRow label="Routes">
            <span className="text-gray-300">{svc.routes.length}</span>
          </MetaRow>
        )}
        {svc.dependencies.length > 0 && (
          <MetaRow label="Dependencies">
            <span className="text-gray-300">
              {svc.dependencies.join(", ")}
            </span>
          </MetaRow>
        )}
      </>
    );
  }

  if ("Class" in detail) {
    const cls = detail.Class;
    return (
      <>
        {cls.extends && (
          <MetaRow label="Extends">
            <span className="text-gray-300">{cls.extends}</span>
          </MetaRow>
        )}
        {cls.implements.length > 0 && (
          <MetaRow label="Implements">
            <span className="text-gray-300">
              {cls.implements.join(", ")}
            </span>
          </MetaRow>
        )}
        <MetaRow label="Methods">
          <span className="text-gray-300">{cls.methods.length}</span>
        </MetaRow>
        {cls.is_abstract && (
          <MetaRow label="Abstract">
            <span className="text-yellow-400">Yes</span>
          </MetaRow>
        )}
      </>
    );
  }

  if ("Function" in detail) {
    const func = detail.Function;
    return (
      <>
        {func.is_async && (
          <MetaRow label="Async">
            <span className="text-yellow-400">Yes</span>
          </MetaRow>
        )}
        <MetaRow label="Parameters">
          <span className="text-gray-300">{func.parameters.length}</span>
        </MetaRow>
        {func.return_type && (
          <MetaRow label="Returns">
            <span className="text-gray-300">{func.return_type}</span>
          </MetaRow>
        )}
      </>
    );
  }

  if ("Schema" in detail) {
    const schema = detail.Schema;
    return (
      <>
        <MetaRow label="Framework">
          <span className="text-gray-300">{schema.source_framework}</span>
        </MetaRow>
        <MetaRow label="Fields">
          <span className="text-gray-300">{schema.fields.length}</span>
        </MetaRow>
        {schema.table_name && (
          <MetaRow label="Table">
            <span className="text-gray-300">{schema.table_name}</span>
          </MetaRow>
        )}
      </>
    );
  }

  if ("Impl" in detail) {
    const impl = detail.Impl;
    return (
      <>
        <MetaRow label="Target">
          <span className="text-gray-300">{impl.target}</span>
        </MetaRow>
        {impl.trait_name && (
          <MetaRow label="Trait">
            <span className="text-gray-300">{impl.trait_name}</span>
          </MetaRow>
        )}
        <MetaRow label="Methods">
          <span className="text-gray-300">{impl.methods.length}</span>
        </MetaRow>
      </>
    );
  }

  if ("TypeAlias" in detail) {
    const alias = detail.TypeAlias;
    return (
      <>
        <MetaRow label="Target">
          <span className="text-gray-300">{alias.target}</span>
        </MetaRow>
        {alias.generics.length > 0 && (
          <MetaRow label="Generics">
            <span className="text-gray-300">
              {"<"}
              {alias.generics.join(", ")}
              {">"}
            </span>
          </MetaRow>
        )}
      </>
    );
  }

  return null;
}

function shortenPath(path: string): string {
  const parts = path.split("/");
  if (parts.length <= 3) return path;
  return `.../${parts.slice(-3).join("/")}`;
}
