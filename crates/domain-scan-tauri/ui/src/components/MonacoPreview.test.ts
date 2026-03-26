import { describe, it, expect } from "vitest";
import { detectLanguage } from "./MonacoPreview";

// ---------------------------------------------------------------------------
// detectLanguage — language detection from domain-scan Language + file ext
// ---------------------------------------------------------------------------

describe("detectLanguage", () => {
  // --- LANGUAGE_MAP-based detection ---

  it("returns typescript for Language = 'TypeScript'", () => {
    expect(detectLanguage("TypeScript", null)).toBe("typescript");
  });

  it("returns python for Language = 'Python'", () => {
    expect(detectLanguage("Python", null)).toBe("python");
  });

  it("returns rust for Language = 'Rust'", () => {
    expect(detectLanguage("Rust", null)).toBe("rust");
  });

  it("returns go for Language = 'Go'", () => {
    expect(detectLanguage("Go", null)).toBe("go");
  });

  it("returns java for Language = 'Java'", () => {
    expect(detectLanguage("Java", null)).toBe("java");
  });

  it("returns kotlin for Language = 'Kotlin'", () => {
    expect(detectLanguage("Kotlin", null)).toBe("kotlin");
  });

  it("returns csharp for Language = 'CSharp'", () => {
    expect(detectLanguage("CSharp", null)).toBe("csharp");
  });

  it("returns swift for Language = 'Swift'", () => {
    expect(detectLanguage("Swift", null)).toBe("swift");
  });

  it("returns php for Language = 'PHP'", () => {
    expect(detectLanguage("PHP", null)).toBe("php");
  });

  it("returns ruby for Language = 'Ruby'", () => {
    expect(detectLanguage("Ruby", null)).toBe("ruby");
  });

  it("returns scala for Language = 'Scala'", () => {
    expect(detectLanguage("Scala", null)).toBe("scala");
  });

  it("returns cpp for Language = 'Cpp'", () => {
    expect(detectLanguage("Cpp", null)).toBe("cpp");
  });

  // --- EXT_MAP-based detection (fallback) ---

  it("falls back to extension when language is null", () => {
    expect(detectLanguage(null, "src/main.ts")).toBe("typescript");
  });

  it("detects tsx files as typescript", () => {
    expect(detectLanguage(null, "component.tsx")).toBe("typescript");
  });

  it("detects js files as javascript", () => {
    expect(detectLanguage(null, "app.js")).toBe("javascript");
  });

  it("detects jsx files as javascript", () => {
    expect(detectLanguage(null, "component.jsx")).toBe("javascript");
  });

  it("detects py files as python", () => {
    expect(detectLanguage(null, "script.py")).toBe("python");
  });

  it("detects rs files as rust", () => {
    expect(detectLanguage(null, "lib.rs")).toBe("rust");
  });

  it("detects go files", () => {
    expect(detectLanguage(null, "main.go")).toBe("go");
  });

  it("detects java files", () => {
    expect(detectLanguage(null, "Main.java")).toBe("java");
  });

  it("detects kotlin files", () => {
    expect(detectLanguage(null, "App.kt")).toBe("kotlin");
  });

  it("detects csharp files", () => {
    expect(detectLanguage(null, "Program.cs")).toBe("csharp");
  });

  it("detects swift files", () => {
    expect(detectLanguage(null, "ViewController.swift")).toBe("swift");
  });

  it("detects php files", () => {
    expect(detectLanguage(null, "index.php")).toBe("php");
  });

  it("detects ruby files", () => {
    expect(detectLanguage(null, "app.rb")).toBe("ruby");
  });

  it("detects scala files", () => {
    expect(detectLanguage(null, "Main.scala")).toBe("scala");
  });

  it("detects cpp files from .cpp extension", () => {
    expect(detectLanguage(null, "main.cpp")).toBe("cpp");
  });

  it("detects cpp files from .cc extension", () => {
    expect(detectLanguage(null, "main.cc")).toBe("cpp");
  });

  it("detects cpp files from .cxx extension", () => {
    expect(detectLanguage(null, "main.cxx")).toBe("cpp");
  });

  it("detects c files from .c extension", () => {
    expect(detectLanguage(null, "main.c")).toBe("c");
  });

  it("detects c files from .h extension", () => {
    expect(detectLanguage(null, "header.h")).toBe("c");
  });

  it("detects hpp files as cpp", () => {
    expect(detectLanguage(null, "header.hpp")).toBe("cpp");
  });

  it("detects json files", () => {
    expect(detectLanguage(null, "package.json")).toBe("json");
  });

  it("detects yaml files from .yaml extension", () => {
    expect(detectLanguage(null, "config.yaml")).toBe("yaml");
  });

  it("detects yaml files from .yml extension", () => {
    expect(detectLanguage(null, "config.yml")).toBe("yaml");
  });

  it("detects markdown files", () => {
    expect(detectLanguage(null, "README.md")).toBe("markdown");
  });

  it("detects html files", () => {
    expect(detectLanguage(null, "index.html")).toBe("html");
  });

  it("detects css files", () => {
    expect(detectLanguage(null, "styles.css")).toBe("css");
  });

  it("detects scss files", () => {
    expect(detectLanguage(null, "styles.scss")).toBe("scss");
  });

  it("detects toml files as ini", () => {
    expect(detectLanguage(null, "Cargo.toml")).toBe("ini");
  });

  it("detects sql files", () => {
    expect(detectLanguage(null, "schema.sql")).toBe("sql");
  });

  it("detects shell files from .sh extension", () => {
    expect(detectLanguage(null, "build.sh")).toBe("shell");
  });

  it("detects shell files from .bash extension", () => {
    expect(detectLanguage(null, "build.bash")).toBe("shell");
  });

  it("detects shell files from .zsh extension", () => {
    expect(detectLanguage(null, "config.zsh")).toBe("shell");
  });

  // --- Fallback to plaintext ---

  it("returns plaintext when both language and file are null", () => {
    expect(detectLanguage(null, null)).toBe("plaintext");
  });

  it("returns plaintext for unknown language string", () => {
    expect(detectLanguage("Haskell", null)).toBe("plaintext");
  });

  it("returns plaintext for unknown file extension", () => {
    expect(detectLanguage(null, "data.xyz")).toBe("plaintext");
  });

  it("returns plaintext for file with no extension", () => {
    expect(detectLanguage(null, "Makefile")).toBe("plaintext");
  });

  // --- Priority: LANGUAGE_MAP overrides EXT_MAP ---

  it("prefers LANGUAGE_MAP over file extension", () => {
    // If both are provided, language takes priority
    expect(detectLanguage("Python", "main.ts")).toBe("python");
  });

  // --- Edge cases ---

  it("handles empty language string (falls back to ext)", () => {
    expect(detectLanguage("", "main.py")).toBe("python");
  });

  it("handles file path with dots in directory names", () => {
    // Should extract extension from the last segment
    expect(detectLanguage(null, "src/v2.0/main.ts")).toBe("typescript");
  });

  it("handles uppercase extension via toLowerCase", () => {
    expect(detectLanguage(null, "FILE.PY")).toBe("python");
  });

  it("handles mixed-case extension via toLowerCase", () => {
    expect(detectLanguage(null, "app.Ts")).toBe("typescript");
  });
});
