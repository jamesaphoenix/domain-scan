import type { TreeNode, TreeChild, BuildStatus, EntityKind } from "../types";

interface EntityTreeProps {
  nodes: TreeNode[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onToggleExpand: (index: number) => void;
}

const BUILD_STATUS_COLORS: Record<BuildStatus, string> = {
  built: "bg-green-500",
  unbuilt: "bg-yellow-500",
  error: "bg-red-500",
  rebuild: "bg-orange-500",
};

const KIND_ICONS: Record<EntityKind, string> = {
  interface: "I",
  service: "S",
  class: "C",
  function: "F",
  schema: "D",
  impl: "M",
  type_alias: "T",
  method: "m",
};

const KIND_COLORS: Record<EntityKind, string> = {
  interface: "text-blue-400",
  service: "text-purple-400",
  class: "text-yellow-400",
  function: "text-green-400",
  schema: "text-pink-400",
  impl: "text-cyan-400",
  type_alias: "text-gray-400",
  method: "text-gray-500",
};

function StatusDot({ status }: { status: BuildStatus }) {
  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${BUILD_STATUS_COLORS[status]} flex-shrink-0`}
      title={status}
    />
  );
}

function KindBadge({ kind }: { kind: EntityKind }) {
  return (
    <span
      className={`inline-flex items-center justify-center w-4 h-4 text-[10px] font-bold ${KIND_COLORS[kind]} flex-shrink-0`}
      title={kind}
    >
      {KIND_ICONS[kind]}
    </span>
  );
}

function ChildRow({ child }: { child: TreeChild }) {
  return (
    <div className="flex items-center gap-1.5 pl-8 py-0.5 text-xs text-gray-400 hover:text-gray-200 hover:bg-gray-800/50 cursor-default">
      <span className="text-gray-600 w-3 text-center">
        {child.kind === "method"
          ? "m"
          : child.kind === "property"
            ? "p"
            : child.kind === "route"
              ? "r"
              : "f"}
      </span>
      <span className="truncate">
        {child.name}
        {child.is_async && (
          <span className="text-yellow-600 ml-1">async</span>
        )}
        {child.return_type && (
          <span className="text-gray-600 ml-1">: {child.return_type}</span>
        )}
      </span>
    </div>
  );
}

export function EntityTree({
  nodes,
  selectedIndex,
  onSelect,
  onToggleExpand,
}: EntityTreeProps) {
  if (nodes.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-600 text-xs">
        No entities found
      </div>
    );
  }

  return (
    <div className="text-sm select-none">
      {nodes.map((node, index) => (
        <div key={`${node.entity.name}-${node.entity.file}-${index}`}>
          {/* Parent node */}
          <div
            className={`flex items-center gap-1.5 px-2 py-1 cursor-pointer rounded-sm ${
              index === selectedIndex
                ? "bg-blue-900/50 text-white"
                : "hover:bg-gray-800/50 text-gray-300"
            }`}
            onClick={() => {
              onSelect(index);
              onToggleExpand(index);
            }}
          >
            {/* Expand indicator */}
            <span className="w-3 text-gray-500 text-xs flex-shrink-0">
              {node.children.length > 0 || hasChildren(node.entity.kind)
                ? node.expanded
                  ? "v"
                  : ">"
                : " "}
            </span>

            <StatusDot status={node.entity.build_status} />
            <KindBadge kind={node.entity.kind} />

            <span className="truncate font-medium">{node.entity.name}</span>

            <span className="ml-auto text-[10px] text-gray-600 flex-shrink-0">
              {node.entity.language}
            </span>
          </div>

          {/* Children (methods, properties, routes) */}
          {node.expanded &&
            node.children.map((child, ci) => (
              <ChildRow
                key={`${child.name}-${ci}`}
                child={child}
              />
            ))}
        </div>
      ))}
    </div>
  );
}

/** Entity kinds that can have children */
function hasChildren(kind: EntityKind): boolean {
  return (
    kind === "interface" ||
    kind === "service" ||
    kind === "class" ||
    kind === "schema" ||
    kind === "impl"
  );
}
