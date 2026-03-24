use std::cell::RefCell;
use std::path::Path;

use tree_sitter::Parser;

use crate::ir::Language;
use crate::DomainScanError;

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new(Parser::new());
}

/// Parse source code into a tree-sitter syntax tree.
///
/// Uses a thread-local parser pool so this is safe to call from rayon workers.
pub fn parse_source(source: &[u8], language: Language) -> Result<tree_sitter::Tree, DomainScanError> {
    let ts_lang = get_tree_sitter_language(language)?;
    PARSER.with(|parser| {
        let mut parser = parser.borrow_mut();
        parser
            .set_language(&ts_lang)
            .map_err(|e| DomainScanError::TreeSitterLanguage(e.to_string()))?;
        parser
            .parse(source, None)
            .ok_or_else(|| DomainScanError::ParseFailed(std::path::PathBuf::from("<source>")))
    })
}

/// Parse a file from disk into a tree-sitter syntax tree.
pub fn parse_file(path: &Path, language: Language) -> Result<(tree_sitter::Tree, Vec<u8>), DomainScanError> {
    let source = std::fs::read(path)?;
    let tree = parse_source(&source, language)?;
    Ok((tree, source))
}

/// Get the tree-sitter Language for a supported language.
/// Returns UnsupportedLanguage for languages whose grammars aren't yet linked.
fn get_tree_sitter_language(language: Language) -> Result<tree_sitter::Language, DomainScanError> {
    match language {
        Language::TypeScript => Ok(tree_sitter_typescript::language_typescript()),
        other => Err(DomainScanError::UnsupportedLanguage(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_typescript_interface() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"interface Foo { bar(): string; }";
        let tree = parse_source(source, Language::TypeScript)?;
        let root = tree.root_node();
        assert_eq!(root.kind(), "program");
        assert!(!root.has_error());
        Ok(())
    }

    #[test]
    fn test_parse_typescript_class() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"class MyService { async getData(): Promise<string> { return ''; } }";
        let tree = parse_source(source, Language::TypeScript)?;
        let root = tree.root_node();
        assert_eq!(root.kind(), "program");
        assert!(!root.has_error());
        assert!(root.child_count() > 0);
        Ok(())
    }

    #[test]
    fn test_parse_typescript_function() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"function add(a: number, b: number): number { return a + b; }";
        let tree = parse_source(source, Language::TypeScript)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_unsupported_language() {
        let result = parse_source(b"fn main() {}", Language::Rust);
        assert!(result.is_err());
        let err = result.err();
        assert!(
            matches!(err, Some(DomainScanError::UnsupportedLanguage(Language::Rust)))
        );
    }

    #[test]
    fn test_parse_file_from_disk() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::TempDir::new()?;
        let path = dir.path().join("test.ts");
        std::fs::write(&path, "export const x: number = 42;")?;

        let (tree, source) = parse_file(&path, Language::TypeScript)?;
        assert!(!tree.root_node().has_error());
        assert_eq!(source, b"export const x: number = 42;");
        Ok(())
    }

    #[test]
    fn test_parser_thread_local_reuse() -> Result<(), Box<dyn std::error::Error>> {
        // Parse two files in sequence; the thread-local parser should be reused
        let source1 = b"const a = 1;";
        let source2 = b"interface B { c: string; }";
        let tree1 = parse_source(source1, Language::TypeScript)?;
        let tree2 = parse_source(source2, Language::TypeScript)?;
        assert!(!tree1.root_node().has_error());
        assert!(!tree2.root_node().has_error());
        Ok(())
    }
}
