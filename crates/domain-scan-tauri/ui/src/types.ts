// Types matching the Rust IR types for IPC communication

export type Language =
  | "TypeScript"
  | "Python"
  | "Rust"
  | "Go"
  | "Java"
  | "Kotlin"
  | "CSharp"
  | "Swift"
  | "PHP"
  | "Ruby"
  | "Scala"
  | "Cpp";

export type BuildStatus = "built" | "unbuilt" | "error" | "rebuild";

export type Confidence = "high" | "medium" | "low";

export type EntityKind =
  | "interface"
  | "service"
  | "class"
  | "function"
  | "schema"
  | "impl"
  | "type_alias"
  | "method";

export type Visibility = "public" | "private" | "protected" | "internal";

export type InterfaceKind =
  | "interface"
  | "trait"
  | "protocol"
  | "abstract_class"
  | "pure_virtual"
  | "module";

export type ServiceKind =
  | "http_controller"
  | "grpc_service"
  | "graphql_resolver"
  | "worker"
  | "microservice"
  | "cli_command"
  | "event_handler"
  | "middleware"
  | "repository"
  | "effect_service"
  | { custom: string };

export type SchemaKind =
  | "zod"
  | "effect_schema"
  | "pydantic"
  | "dataclass"
  | "typed_dict"
  | "sqlalchemy"
  | "drizzle"
  | "serde"
  | "go_struct"
  | "java_entity"
  | "java_record"
  | "kotlin_data_class"
  | { custom: string };

export interface Span {
  start_line: number;
  start_col: number;
  end_line: number;
  end_col: number;
  byte_range: [number, number];
}

export interface EntitySummary {
  name: string;
  kind: EntityKind;
  file: string;
  line: number;
  language: Language;
  build_status: BuildStatus;
  confidence: Confidence;
}

export interface FilterParams {
  languages?: Language[];
  name_pattern?: string;
  kind?: EntityKind[];
  build_status?: BuildStatus;
  visibility?: string;
}

export interface ScanStats {
  total_files: number;
  files_by_language: Record<string, number>;
  total_interfaces: number;
  total_services: number;
  total_classes: number;
  total_methods: number;
  total_functions: number;
  total_schemas: number;
  total_type_aliases: number;
  total_implementations: number;
  parse_duration_ms: number;
  cache_hits: number;
  cache_misses: number;
}

// Detail types for get_entity_detail responses

export interface Parameter {
  name: string;
  type_annotation: string | null;
  is_optional: boolean;
  default_value: string | null;
}

export interface PropertyDef {
  name: string;
  type_annotation: string | null;
  is_optional: boolean;
  is_readonly: boolean;
  visibility: Visibility;
}

export interface MethodSignature {
  name: string;
  span: Span;
  is_async: boolean;
  parameters: Parameter[];
  return_type: string | null;
}

export interface MethodDef {
  name: string;
  file: string;
  span: Span;
  visibility: Visibility;
  is_async: boolean;
  is_static: boolean;
  is_generator: boolean;
  parameters: Parameter[];
  return_type: string | null;
  decorators: string[];
  owner: string | null;
  implements: string | null;
}

export interface RouteDef {
  method: string;
  path: string;
  handler: string;
}

export interface SchemaField {
  name: string;
  type_annotation: string | null;
  is_optional: boolean;
}

export interface InterfaceDef {
  name: string;
  file: string;
  span: Span;
  visibility: Visibility;
  generics: string[];
  extends: string[];
  methods: MethodSignature[];
  properties: PropertyDef[];
  language_kind: InterfaceKind;
  decorators: string[];
}

export interface ServiceDef {
  name: string;
  file: string;
  span: Span;
  kind: ServiceKind;
  methods: MethodDef[];
  dependencies: string[];
  decorators: string[];
  routes: RouteDef[];
}

export interface ClassDef {
  name: string;
  file: string;
  span: Span;
  visibility: Visibility;
  generics: string[];
  extends: string | null;
  implements: string[];
  methods: MethodDef[];
  properties: PropertyDef[];
  is_abstract: boolean;
  decorators: string[];
}

export interface FunctionDef {
  name: string;
  file: string;
  span: Span;
  visibility: Visibility;
  is_async: boolean;
  is_generator: boolean;
  parameters: Parameter[];
  return_type: string | null;
  decorators: string[];
}

export interface SchemaDef {
  name: string;
  file: string;
  span: Span;
  kind: SchemaKind;
  fields: SchemaField[];
  source_framework: string;
  table_name: string | null;
  derives: string[];
  visibility: Visibility;
}

export interface ImplDef {
  target: string;
  trait_name: string | null;
  file: string;
  span: Span;
  methods: MethodDef[];
}

export interface TypeAlias {
  name: string;
  file: string;
  span: Span;
  target: string;
  generics: string[];
  visibility: Visibility;
}

// Tagged union for Entity (matches Rust's serde externally-tagged enum)
export type Entity =
  | { Interface: InterfaceDef }
  | { Service: ServiceDef }
  | { Class: ClassDef }
  | { Function: FunctionDef }
  | { Schema: SchemaDef }
  | { Impl: ImplDef }
  | { TypeAlias: TypeAlias };

// ---------------------------------------------------------------------------
// Tube Map IPC types (matches Rust DTOs in commands.rs)
// ---------------------------------------------------------------------------

export type ConnectionType = "depends_on" | "uses" | "triggers";

export interface ManifestMeta {
  name: string;
  version: string;
  description: string;
}

export interface DomainDef {
  label: string;
  color: string;
}

export interface TubeMapConnection {
  from: string;
  to: string;
  label: string;
  type: ConnectionType;
}

export interface TubeMapSubsystem {
  id: string;
  name: string;
  domain: string;
  status: string;
  description: string;
  file_path: string;
  matched_entity_count: number;
  interface_count: number;
  operation_count: number;
  table_count: number;
  event_count: number;
  has_children: boolean;
  child_count: number;
  dependency_count: number;
}

export interface TubeMapData {
  meta: ManifestMeta;
  domains: Record<string, DomainDef>;
  subsystems: TubeMapSubsystem[];
  connections: TubeMapConnection[];
  coverage_percent: number;
  unmatched_count: number;
}

export interface SubsystemDetail {
  id: string;
  name: string;
  domain: string;
  status: string;
  file_path: string;
  interfaces: string[];
  operations: string[];
  tables: string[];
  events: string[];
  dependencies: string[];
  children: SubsystemDetail[];
  matched_entities: EntitySummary[];
}

// Tree node for entity tree display
export interface TreeNode {
  entity: EntitySummary;
  expanded: boolean;
  children: TreeChild[];
}

export interface TreeChild {
  name: string;
  kind: "method" | "property" | "route" | "field";
  line: number;
  is_async?: boolean;
  return_type?: string | null;
}
