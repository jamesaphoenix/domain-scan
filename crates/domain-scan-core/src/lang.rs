use std::path::Path;

use crate::ir::Language;

/// Detect the programming language of a file by its extension.
/// Returns `None` for unsupported or extensionless files.
pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some(Language::TypeScript),
        "py" | "pyi" => Some(Language::Python),
        "rs" => Some(Language::Rust),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "kt" | "kts" => Some(Language::Kotlin),
        "cs" => Some(Language::CSharp),
        "swift" => Some(Language::Swift),
        "php" => Some(Language::PHP),
        "rb" => Some(Language::Ruby),
        "scala" | "sc" => Some(Language::Scala),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h" => Some(Language::Cpp),
        _ => None,
    }
}

/// Returns all recognized file extensions for a language.
pub fn extensions_for(language: Language) -> &'static [&'static str] {
    match language {
        Language::TypeScript => &["ts", "tsx", "js", "jsx", "mjs", "cjs"],
        Language::Python => &["py", "pyi"],
        Language::Rust => &["rs"],
        Language::Go => &["go"],
        Language::Java => &["java"],
        Language::Kotlin => &["kt", "kts"],
        Language::CSharp => &["cs"],
        Language::Swift => &["swift"],
        Language::PHP => &["php"],
        Language::Ruby => &["rb"],
        Language::Scala => &["scala", "sc"],
        Language::Cpp => &["cpp", "cc", "cxx", "hpp", "hxx", "h"],
    }
}

/// Returns all supported languages.
pub fn all_languages() -> &'static [Language] {
    &[
        Language::TypeScript,
        Language::Python,
        Language::Rust,
        Language::Go,
        Language::Java,
        Language::Kotlin,
        Language::CSharp,
        Language::Swift,
        Language::PHP,
        Language::Ruby,
        Language::Scala,
        Language::Cpp,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("foo.ts", Some(Language::TypeScript))]
    #[case("foo.tsx", Some(Language::TypeScript))]
    #[case("foo.js", Some(Language::TypeScript))]
    #[case("foo.jsx", Some(Language::TypeScript))]
    #[case("foo.mjs", Some(Language::TypeScript))]
    #[case("foo.cjs", Some(Language::TypeScript))]
    #[case("foo.py", Some(Language::Python))]
    #[case("foo.pyi", Some(Language::Python))]
    #[case("foo.rs", Some(Language::Rust))]
    #[case("foo.go", Some(Language::Go))]
    #[case("foo.java", Some(Language::Java))]
    #[case("foo.kt", Some(Language::Kotlin))]
    #[case("foo.kts", Some(Language::Kotlin))]
    #[case("foo.cs", Some(Language::CSharp))]
    #[case("foo.swift", Some(Language::Swift))]
    #[case("foo.php", Some(Language::PHP))]
    #[case("foo.rb", Some(Language::Ruby))]
    #[case("foo.scala", Some(Language::Scala))]
    #[case("foo.sc", Some(Language::Scala))]
    #[case("foo.cpp", Some(Language::Cpp))]
    #[case("foo.cc", Some(Language::Cpp))]
    #[case("foo.cxx", Some(Language::Cpp))]
    #[case("foo.hpp", Some(Language::Cpp))]
    #[case("foo.hxx", Some(Language::Cpp))]
    #[case("foo.h", Some(Language::Cpp))]
    fn test_detect_language(#[case] filename: &str, #[case] expected: Option<Language>) {
        assert_eq!(detect_language(Path::new(filename)), expected);
    }

    #[rstest]
    #[case("foo.txt", None)]
    #[case("foo.md", None)]
    #[case("foo.json", None)]
    #[case("foo.yaml", None)]
    #[case("Makefile", None)]
    fn test_detect_unsupported(#[case] filename: &str, #[case] expected: Option<Language>) {
        assert_eq!(detect_language(Path::new(filename)), expected);
    }

    #[test]
    fn test_no_extension() {
        assert_eq!(detect_language(Path::new("Makefile")), None);
        assert_eq!(detect_language(Path::new("README")), None);
    }

    #[test]
    fn test_nested_path() {
        assert_eq!(
            detect_language(Path::new("src/components/App.tsx")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            detect_language(Path::new("pkg/server/handler.go")),
            Some(Language::Go)
        );
    }

    #[test]
    fn test_all_languages_covered() {
        // Every language should have at least one extension that maps back to it
        for lang in all_languages() {
            let exts = extensions_for(*lang);
            assert!(!exts.is_empty(), "{lang} has no extensions");
            for ext in exts {
                let path = format!("test.{ext}");
                assert_eq!(
                    detect_language(Path::new(&path)),
                    Some(*lang),
                    "Extension {ext} should map to {lang}"
                );
            }
        }
    }
}
