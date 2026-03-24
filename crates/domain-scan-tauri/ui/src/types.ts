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
