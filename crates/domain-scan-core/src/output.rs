use crate::ir::{IrFile, ScanIndex};
use crate::DomainScanError;

/// Output format for CLI and API responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
    Compact,
}

/// Format a `ScanIndex` in the given output format.
pub fn format_scan_index(index: &ScanIndex, format: OutputFormat) -> Result<String, DomainScanError> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(index)?),
        OutputFormat::Table => Ok(format_table(index)),
        OutputFormat::Compact => Ok(format_compact(index)),
    }
}

/// Format a single `IrFile` in the given output format.
pub fn format_ir_file(file: &IrFile, format: OutputFormat) -> Result<String, DomainScanError> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(file)?),
        OutputFormat::Table => Ok(format_ir_file_table(file)),
        OutputFormat::Compact => Ok(format_ir_file_compact(file)),
    }
}

fn format_table(index: &ScanIndex) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Scan: {} ({} files)\n",
        index.root.display(),
        index.stats.total_files
    ));
    out.push_str(&format!("Version: {}\n", index.version));
    out.push_str(&format!(
        "Interfaces: {} | Services: {} | Classes: {} | Functions: {}\n",
        index.stats.total_interfaces,
        index.stats.total_services,
        index.stats.total_classes,
        index.stats.total_functions
    ));
    out.push_str(&format!(
        "Schemas: {} | Type Aliases: {} | Implementations: {}\n",
        index.stats.total_schemas,
        index.stats.total_type_aliases,
        index.stats.total_implementations
    ));
    out
}

fn format_compact(index: &ScanIndex) -> String {
    format!(
        "{} files | {} iface | {} svc | {} cls | {} fn | {} schema",
        index.stats.total_files,
        index.stats.total_interfaces,
        index.stats.total_services,
        index.stats.total_classes,
        index.stats.total_functions,
        index.stats.total_schemas,
    )
}

fn format_ir_file_table(file: &IrFile) -> String {
    format!(
        "{}: {} ({}, {})",
        file.path.display(),
        file.language,
        file.build_status,
        file.confidence,
    )
}

fn format_ir_file_compact(file: &IrFile) -> String {
    format!("{} [{}]", file.path.display(), file.language)
}

impl std::fmt::Display for crate::ir::Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BuildStatus, Language, ScanIndex};
    use std::path::PathBuf;

    #[test]
    fn test_format_scan_index_json() -> Result<(), Box<dyn std::error::Error>> {
        let index = ScanIndex::new(PathBuf::from("/tmp/test"));
        let json = format_scan_index(&index, OutputFormat::Json)?;
        // Should be valid JSON
        let _: serde_json::Value = serde_json::from_str(&json)?;
        Ok(())
    }

    #[test]
    fn test_format_scan_index_table() -> Result<(), Box<dyn std::error::Error>> {
        let index = ScanIndex::new(PathBuf::from("/tmp/test"));
        let table = format_scan_index(&index, OutputFormat::Table)?;
        assert!(table.contains("Scan:"));
        assert!(table.contains("0 files"));
        Ok(())
    }

    #[test]
    fn test_format_scan_index_compact() -> Result<(), Box<dyn std::error::Error>> {
        let index = ScanIndex::new(PathBuf::from("/tmp/test"));
        let compact = format_scan_index(&index, OutputFormat::Compact)?;
        assert!(compact.contains("0 files"));
        Ok(())
    }

    #[test]
    fn test_format_ir_file_json() -> Result<(), Box<dyn std::error::Error>> {
        let file = crate::ir::IrFile::new(
            PathBuf::from("test.ts"),
            Language::TypeScript,
            "hash123".to_string(),
            BuildStatus::Built,
        );
        let json = format_ir_file(&file, OutputFormat::Json)?;
        let _: serde_json::Value = serde_json::from_str(&json)?;
        Ok(())
    }

    #[test]
    fn test_format_ir_file_table() -> Result<(), Box<dyn std::error::Error>> {
        let file = crate::ir::IrFile::new(
            PathBuf::from("test.ts"),
            Language::TypeScript,
            "hash123".to_string(),
            BuildStatus::Built,
        );
        let table = format_ir_file(&file, OutputFormat::Table)?;
        assert!(table.contains("test.ts"));
        assert!(table.contains("TypeScript"));
        assert!(table.contains("Built"));
        Ok(())
    }
}
