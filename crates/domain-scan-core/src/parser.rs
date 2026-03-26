use std::cell::RefCell;
use std::path::Path;

use tree_sitter::Parser;

use tree_sitter_language::LanguageFn;

use crate::ir::Language;
use crate::DomainScanError;

/// Convert a `LanguageFn` (from newer grammar crates) to a tree-sitter `Language`.
///
/// The `LanguageFn` wraps `extern "C" fn() -> *const ()` which returns a `*const TSLanguage`.
/// The tree-sitter `Language` struct is a newtype around the same pointer.
///
/// # Safety
/// This relies on `Language` being a transparent wrapper around `*const TSLanguage`,
/// which is guaranteed by tree-sitter's stable C ABI.
fn language_from_fn(lang_fn: LanguageFn) -> tree_sitter::Language {
    let raw_fn = lang_fn.into_raw();
    let ptr = unsafe { raw_fn() };
    // SAFETY: Language is repr(transparent) over *const TSLanguage,
    // and the grammar function returns a valid *const TSLanguage as *const ().
    unsafe { std::mem::transmute(ptr) }
}

/// Get tree-sitter Language for Kotlin (exposed for query compilation).
pub fn kotlin_language() -> tree_sitter::Language {
    language_from_fn(tree_sitter_kotlin_ng::LANGUAGE)
}

/// Get tree-sitter Language for Scala (exposed for query compilation).
pub fn scala_language() -> tree_sitter::Language {
    language_from_fn(tree_sitter_scala::LANGUAGE)
}

/// Get tree-sitter Language for C# (exposed for query compilation).
pub fn csharp_language() -> tree_sitter::Language {
    language_from_fn(tree_sitter_c_sharp::LANGUAGE)
}

/// Get tree-sitter Language for Swift (exposed for query compilation).
pub fn swift_language() -> tree_sitter::Language {
    tree_sitter_swift::language()
}

/// Get tree-sitter Language for C++ (exposed for query compilation).
pub fn cpp_language() -> tree_sitter::Language {
    language_from_fn(tree_sitter_cpp::LANGUAGE)
}

/// Get tree-sitter Language for PHP (exposed for query compilation).
pub fn php_language() -> tree_sitter::Language {
    language_from_fn(tree_sitter_php::LANGUAGE_PHP)
}

/// Get tree-sitter Language for Ruby (exposed for query compilation).
pub fn ruby_language() -> tree_sitter::Language {
    language_from_fn(tree_sitter_ruby::LANGUAGE)
}

thread_local! {
    static PARSER: RefCell<Parser> = RefCell::new(Parser::new());
}

/// Parse source code into a tree-sitter syntax tree.
///
/// Uses a thread-local parser pool so this is safe to call from rayon workers.
pub fn parse_source(
    source: &[u8],
    language: Language,
) -> Result<tree_sitter::Tree, DomainScanError> {
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
pub fn parse_file(
    path: &Path,
    language: Language,
) -> Result<(tree_sitter::Tree, Vec<u8>), DomainScanError> {
    let source = std::fs::read(path)?;
    let tree = parse_source(&source, language)?;
    Ok((tree, source))
}

/// Get the tree-sitter Language for a supported language.
/// Returns UnsupportedLanguage for languages whose grammars aren't yet linked.
fn get_tree_sitter_language(language: Language) -> Result<tree_sitter::Language, DomainScanError> {
    match language {
        Language::TypeScript => Ok(tree_sitter_typescript::language_typescript()),
        Language::Rust => Ok(tree_sitter_rust::language()),
        Language::Go => Ok(tree_sitter_go::language()),
        Language::Python => Ok(tree_sitter_python::language()),
        Language::Java => Ok(tree_sitter_java::language()),
        Language::Kotlin => Ok(language_from_fn(tree_sitter_kotlin_ng::LANGUAGE)),
        Language::Scala => Ok(language_from_fn(tree_sitter_scala::LANGUAGE)),
        Language::CSharp => Ok(language_from_fn(tree_sitter_c_sharp::LANGUAGE)),
        Language::Swift => Ok(tree_sitter_swift::language()),
        Language::Cpp => Ok(language_from_fn(tree_sitter_cpp::LANGUAGE)),
        Language::PHP => Ok(language_from_fn(tree_sitter_php::LANGUAGE_PHP)),
        Language::Ruby => Ok(language_from_fn(tree_sitter_ruby::LANGUAGE)),
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
    fn test_parse_csharp() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"public class Foo { public void Bar() {} }";
        let tree = parse_source(source, Language::CSharp)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_java() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"public class Foo { public void bar() {} }";
        let tree = parse_source(source, Language::Java)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_kotlin() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"fun main(args: Array<String>) { println(\"Hello\") }";
        let tree = parse_source(source, Language::Kotlin)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_scala() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"object Main { def main(args: Array[String]): Unit = {} }";
        let tree = parse_source(source, Language::Scala)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_swift() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"protocol Foo { func bar() }";
        let tree = parse_source(source, Language::Swift)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_cpp() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"class Foo { public: void bar() {} };";
        let tree = parse_source(source, Language::Cpp)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_php() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"<?php class Foo { public function bar(): void {} }";
        let tree = parse_source(source, Language::PHP)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_ruby() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"class Foo\n  def bar\n    42\n  end\nend";
        let tree = parse_source(source, Language::Ruby)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_rust_function() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"fn main() { let x = 1; }";
        let tree = parse_source(source, Language::Rust)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_go_function() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"package main\nfunc main() {}";
        let tree = parse_source(source, Language::Go)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_python_function() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"def hello():\n    pass";
        let tree = parse_source(source, Language::Python)?;
        assert!(!tree.root_node().has_error());
        Ok(())
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

    // -----------------------------------------------------------------------
    // Edge case: empty source
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_empty_source_typescript() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"";
        let tree = parse_source(source, Language::TypeScript)?;
        let root = tree.root_node();
        assert_eq!(root.kind(), "program");
        assert!(!root.has_error());
        assert_eq!(root.child_count(), 0);
        Ok(())
    }

    #[test]
    fn test_parse_empty_source_rust() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"";
        let tree = parse_source(source, Language::Rust)?;
        let root = tree.root_node();
        assert_eq!(root.kind(), "source_file");
        assert!(!root.has_error());
        assert_eq!(root.child_count(), 0);
        Ok(())
    }

    #[test]
    fn test_parse_empty_source_python() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"";
        let tree = parse_source(source, Language::Python)?;
        let root = tree.root_node();
        assert!(!root.has_error());
        Ok(())
    }

    #[test]
    fn test_parse_empty_source_go() -> Result<(), Box<dyn std::error::Error>> {
        // Go requires `package` declaration, so empty source has error
        let source = b"";
        let tree = parse_source(source, Language::Go)?;
        let root = tree.root_node();
        // Go grammar may produce an error node for empty input — that's fine
        assert_eq!(root.kind(), "source_file");
        Ok(())
    }

    #[test]
    fn test_parse_empty_source_all_languages() -> Result<(), Box<dyn std::error::Error>> {
        // Every language should parse empty source without panicking
        let languages = [
            Language::TypeScript,
            Language::Rust,
            Language::Go,
            Language::Python,
            Language::Java,
            Language::Kotlin,
            Language::Scala,
            Language::CSharp,
            Language::Swift,
            Language::Cpp,
            Language::PHP,
            Language::Ruby,
        ];
        for lang in languages {
            let tree = parse_source(b"", lang)?;
            // Root node should exist regardless
            let _root = tree.root_node();
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: whitespace-only source
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_whitespace_only() -> Result<(), Box<dyn std::error::Error>> {
        let source = b"   \n\n\t\t   \n";
        let tree = parse_source(source, Language::TypeScript)?;
        let root = tree.root_node();
        assert!(!root.has_error());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: binary content (should not panic)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_binary_content_no_panic() -> Result<(), Box<dyn std::error::Error>> {
        // PNG header + random bytes — tree-sitter should not panic
        let source: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00,
            0x0D, 0x49, 0x48, 0x44, 0x52, 0xFF, 0xFE, 0x00, 0x01,
        ];
        // tree-sitter may produce a tree with errors, or may fail to parse
        // The important thing is no panic
        let result = parse_source(source, Language::TypeScript);
        // Either Ok or Err is fine, as long as no panic
        let _ = result;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: language switching on thread-local parser
    // -----------------------------------------------------------------------

    #[test]
    fn test_language_switching() -> Result<(), Box<dyn std::error::Error>> {
        // Parse TypeScript, then Rust, then Python — the thread-local parser
        // must correctly switch languages between calls.
        let ts_tree = parse_source(b"const x = 1;", Language::TypeScript)?;
        assert_eq!(ts_tree.root_node().kind(), "program");
        assert!(!ts_tree.root_node().has_error());

        let rs_tree = parse_source(b"fn main() {}", Language::Rust)?;
        assert_eq!(rs_tree.root_node().kind(), "source_file");
        assert!(!rs_tree.root_node().has_error());

        let py_tree = parse_source(b"x = 1", Language::Python)?;
        assert!(!py_tree.root_node().has_error());

        // Go back to TypeScript to verify parser reuse
        let ts_tree2 = parse_source(b"interface Foo {}", Language::TypeScript)?;
        assert_eq!(ts_tree2.root_node().kind(), "program");
        assert!(!ts_tree2.root_node().has_error());

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: parse_file with nonexistent path
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_file_nonexistent() {
        let result = parse_file(
            std::path::Path::new("/nonexistent/path/file.ts"),
            Language::TypeScript,
        );
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Edge case: parse_file with empty file on disk
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_file_empty_on_disk() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::TempDir::new()?;
        let path = dir.path().join("empty.ts");
        std::fs::write(&path, "")?;

        let (tree, source) = parse_file(&path, Language::TypeScript)?;
        assert!(source.is_empty());
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: source with syntax errors (partial recovery)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_source_with_syntax_errors() -> Result<(), Box<dyn std::error::Error>> {
        // Incomplete TypeScript — tree-sitter does partial recovery
        let source = b"interface { }"; // missing name
        let tree = parse_source(source, Language::TypeScript)?;
        // tree-sitter should still produce a tree (with error markers)
        let root = tree.root_node();
        assert!(root.has_error()); // should have error nodes
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: Unicode content
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_unicode_identifiers() -> Result<(), Box<dyn std::error::Error>> {
        let source = "interface \u{30E6}\u{30FC}\u{30B6}\u{30FC} { name: string; }";
        let tree = parse_source(source.as_bytes(), Language::TypeScript)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    #[test]
    fn test_parse_unicode_strings() -> Result<(), Box<dyn std::error::Error>> {
        let source = "const greeting = \"\u{4F60}\u{597D}\u{4E16}\u{754C}\";";
        let tree = parse_source(source.as_bytes(), Language::TypeScript)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: very long single line
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_very_long_line() -> Result<(), Box<dyn std::error::Error>> {
        // Build a TypeScript file with a very long array literal
        let mut source = String::from("const arr = [");
        for i in 0..10_000 {
            if i > 0 {
                source.push_str(", ");
            }
            source.push_str(&i.to_string());
        }
        source.push_str("];");

        let tree = parse_source(source.as_bytes(), Language::TypeScript)?;
        assert!(!tree.root_node().has_error());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Edge case: all language_from_fn conversions
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_language_fn_conversions() {
        // Each public language function should return a valid Language without panic
        let _kotlin = kotlin_language();
        let _scala = scala_language();
        let _csharp = csharp_language();
        let _swift = swift_language();
        let _cpp = cpp_language();
        let _php = php_language();
        let _ruby = ruby_language();
    }
}
