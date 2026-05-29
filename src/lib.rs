//! Forge Code — Decompose source code into tiles for Plato agents

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTile {
    pub id: Uuid,
    pub kind: CodeKind,
    pub name: String,
    pub body: String,
    pub start_line: usize,
    pub end_line: usize,
    pub language: Language,
    pub meta: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CodeKind {
    Function, Struct, Enum, Impl, Trait, Class, Method, Import, Comment, Module, Block, Test,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    Rust, Python, TypeScript, JavaScript, Go, C, Java, Unknown,
}

pub struct CodeDecomposer { pub language: Language }

impl CodeDecomposer {
    pub fn new(language: Language) -> Self { Self { language } }

    pub fn detect_language(input: &str, filename: Option<&str>) -> Language {
        if let Some(name) = filename {
            match name.rsplit('.').next().unwrap_or("") {
                "rs" => return Language::Rust,
                "py" => return Language::Python,
                "ts" => return Language::TypeScript,
                "js" => return Language::JavaScript,
                "go" => return Language::Go,
                "c" | "h" => return Language::C,
                "java" => return Language::Java,
                _ => {}
            }
        }
        if input.contains("fn ") && input.contains("let mut") { return Language::Rust; }
        if input.contains("def ") && input.contains("import ") { return Language::Python; }
        if input.contains("func ") && input.contains("package ") { return Language::Go; }
        Language::Unknown
    }

    pub fn decompose(&self, input: &str) -> Vec<CodeTile> {
        match self.language {
            Language::Rust => self.decompose_rust(input),
            Language::Python => self.decompose_python(input),
            Language::TypeScript | Language::JavaScript => self.decompose_ts(input),
            Language::Go => self.decompose_go(input),
            Language::C => self.decompose_c(input),
            Language::Java => self.decompose_java(input),
            Language::Unknown => self.decompose_generic(input),
        }
    }

    fn tile(&self, kind: CodeKind, name: &str, body: &str, start: usize, end: usize) -> CodeTile {
        let mut meta = HashMap::new();
        meta.insert("lines".into(), (end.saturating_sub(start) + 1).to_string());
        CodeTile { id: Uuid::new_v4(), kind, name: name.to_string(), body: body.to_string(), start_line: start, end_line: end, language: self.language.clone(), meta }
    }

    fn name_after(line: &str, keyword: &str) -> String {
        if let Some(idx) = line.find(keyword) {
            let after = &line[idx + keyword.len()..];
            after.split(|c: char| c.is_whitespace() || c == '(' || c == '<' || c == '{' || c == ':')
                .next().unwrap_or("unknown").trim().to_string()
        } else { "unknown".into() }
    }

    fn brace_block(lines: &[&str], start: usize) -> (String, usize) {
        let mut depth = 0i32;
        let mut found = false;
        for (i, line) in lines.iter().enumerate().skip(start) {
            for ch in line.chars() {
                match ch {
                    '{' => { depth += 1; found = true; }
                    '}' => { depth -= 1; if depth == 0 && found {
                        let body: String = lines[start..=i].join("\n");
                        return (body, i);
                    }}
                    _ => {}
                }
            }
        }
        (lines[start..].join("\n"), lines.len().saturating_sub(1))
    }

    fn indent_block(lines: &[&str], start: usize) -> (String, usize) {
        let base = lines[start].chars().take_while(|c| *c == ' ' || *c == '\t').count();
        let mut end = start;
        for (i, line) in lines.iter().enumerate().skip(start + 1) {
            if line.trim().is_empty() { end = i; continue; }
            let indent = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
            if indent <= base && !line.trim().is_empty() { break; }
            end = i;
        }
        (lines[start..=end].join("\n"), end)
    }

    pub fn decompose_rust(&self, input: &str) -> Vec<CodeTile> {
        let lines: Vec<&str> = input.lines().collect();
        let mut tiles = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("use ") {
                tiles.push(self.tile(CodeKind::Import, t, t, i+1, i+1));
            } else if t.starts_with("fn ") || t.starts_with("pub fn ") || t.starts_with("async fn ") || t.starts_with("pub async fn ") {
                let name = Self::name_after(t, "fn ");
                let (body, end) = Self::brace_block(&lines, i);
                let kind = if name.starts_with("test_") { CodeKind::Test } else { CodeKind::Function };
                tiles.push(self.tile(kind, &name, &body, i+1, end+1));
            } else if t.starts_with("struct ") || t.starts_with("pub struct ") {
                let name = Self::name_after(t, "struct ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Struct, &name, &body, i+1, end+1));
            } else if t.starts_with("enum ") || t.starts_with("pub enum ") {
                let name = Self::name_after(t, "enum ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Enum, &name, &body, i+1, end+1));
            } else if t.starts_with("impl ") {
                let name = Self::name_after(t, "impl ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Impl, &name, &body, i+1, end+1));
            } else if t.starts_with("trait ") || t.starts_with("pub trait ") {
                let name = Self::name_after(t, "trait ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Trait, &name, &body, i+1, end+1));
            } else if t.starts_with("//") {
                tiles.push(self.tile(CodeKind::Comment, "comment", t, i+1, i+1));
            }
        }
        tiles
    }

    pub fn decompose_python(&self, input: &str) -> Vec<CodeTile> {
        let lines: Vec<&str> = input.lines().collect();
        let mut tiles = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("#") {
                tiles.push(self.tile(CodeKind::Comment, "comment", t, i+1, i+1));
            } else if t.starts_with("import ") || t.starts_with("from ") {
                tiles.push(self.tile(CodeKind::Import, t, t, i+1, i+1));
            } else if t.starts_with("def ") || t.starts_with("async def ") {
                let name = Self::name_after(t, "def ");
                let (body, end) = Self::indent_block(&lines, i);
                let kind = if name.starts_with("test_") { CodeKind::Test } else { CodeKind::Function };
                tiles.push(self.tile(kind, &name, &body, i+1, end+1));
            } else if t.starts_with("class ") {
                let name = Self::name_after(t, "class ");
                let (body, end) = Self::indent_block(&lines, i);
                tiles.push(self.tile(CodeKind::Class, &name, &body, i+1, end+1));
            }
        }
        tiles
    }

    pub fn decompose_ts(&self, input: &str) -> Vec<CodeTile> {
        let lines: Vec<&str> = input.lines().collect();
        let mut tiles = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("import ") {
                tiles.push(self.tile(CodeKind::Import, t, t, i+1, i+1));
            } else if t.starts_with("//") {
                tiles.push(self.tile(CodeKind::Comment, "comment", t, i+1, i+1));
            } else if t.contains("function ") {
                let name = Self::name_after(t, "function ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Function, &name, &body, i+1, end+1));
            } else if t.starts_with("class ") {
                let name = Self::name_after(t, "class ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Class, &name, &body, i+1, end+1));
            } else if t.starts_with("interface ") || t.starts_with("type ") {
                let kw = if t.starts_with("interface ") { "interface " } else { "type " };
                let name = Self::name_after(t, kw);
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Struct, &name, &body, i+1, end+1));
            }
        }
        tiles
    }

    pub fn decompose_go(&self, input: &str) -> Vec<CodeTile> {
        let lines: Vec<&str> = input.lines().collect();
        let mut tiles = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("//") { tiles.push(self.tile(CodeKind::Comment, "comment", t, i+1, i+1)); }
            else if t.starts_with("import ") || t.starts_with("package ") { tiles.push(self.tile(CodeKind::Import, t, t, i+1, i+1)); }
            else if t.starts_with("func ") {
                let name = Self::name_after(t, "func ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Function, &name, &body, i+1, end+1));
            } else if t.starts_with("type ") && t.contains("struct") {
                let name = Self::name_after(t, "type ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Struct, &name, &body, i+1, end+1));
            }
        }
        tiles
    }

    pub fn decompose_c(&self, input: &str) -> Vec<CodeTile> {
        let lines: Vec<&str> = input.lines().collect();
        let mut tiles = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("/*") { tiles.push(self.tile(CodeKind::Comment, "comment", t, i+1, i+1)); }
            else if t.starts_with("#include") { tiles.push(self.tile(CodeKind::Import, t, t, i+1, i+1)); }
            else if t.contains("(") && t.contains(")") && !t.starts_with("if") && !t.starts_with("for") && !t.starts_with("while") && !t.starts_with("return") && t.contains("{") {
                let name = t.split('(').next().unwrap_or("unknown").trim().to_string();
                let (body, end) = Self::brace_block(&lines, i);
                if !body.is_empty() { tiles.push(self.tile(CodeKind::Function, &name, &body, i+1, end+1)); }
            }
        }
        tiles
    }

    pub fn decompose_java(&self, input: &str) -> Vec<CodeTile> {
        let lines: Vec<&str> = input.lines().collect();
        let mut tiles = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let t = line.trim();
            if t.starts_with("//") || t.starts_with("/*") { tiles.push(self.tile(CodeKind::Comment, "comment", t, i+1, i+1)); }
            else if t.starts_with("import ") || t.starts_with("package ") { tiles.push(self.tile(CodeKind::Import, t, t, i+1, i+1)); }
            else if t.contains("class ") && !t.starts_with("if") {
                let name = Self::name_after(t, "class ");
                let (body, end) = Self::brace_block(&lines, i);
                tiles.push(self.tile(CodeKind::Class, &name, &body, i+1, end+1));
            } else if (t.contains("void ") || t.contains("static ")) && t.contains("(") && !t.starts_with("if") && !t.starts_with("for") && !t.starts_with("while") && t.contains("{") {
                let name = t.split('(').next().unwrap_or("unknown").trim().split(' ').last().unwrap_or("unknown").to_string();
                let (body, end) = Self::brace_block(&lines, i);
                if !body.is_empty() { tiles.push(self.tile(CodeKind::Method, &name, &body, i+1, end+1)); }
            }
        }
        tiles
    }

    pub fn decompose_generic(&self, input: &str) -> Vec<CodeTile> {
        let lines: Vec<&str> = input.lines().collect();
        let mut tiles = Vec::new();
        let mut block_start = 0usize;
        let mut block_lines: Vec<&str> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() && !block_lines.is_empty() {
                let body = block_lines.join("\n");
                let name = block_lines[0].trim().split(|c: char| c.is_whitespace() || c == '(').next().unwrap_or("block");
                tiles.push(self.tile(CodeKind::Block, name, &body, block_start + 1, i));
                block_lines.clear();
            } else if !line.trim().is_empty() {
                if block_lines.is_empty() { block_start = i; }
                block_lines.push(*line);
            }
        }
        if !block_lines.is_empty() {
            let body = block_lines.join("\n");
            tiles.push(self.tile(CodeKind::Block, "final", &body, block_start + 1, lines.len()));
        }
        tiles
    }

    pub fn filter_by_kind(tiles: &[CodeTile], kind: &CodeKind) -> Vec<CodeTile> {
        tiles.iter().filter(|t| &t.kind == kind).cloned().collect()
    }

    pub fn stats(tiles: &[CodeTile]) -> CodeStats {
        let mut kind_counts = HashMap::new();
        for t in tiles {
            *kind_counts.entry(format!("{:?}", t.kind)).or_insert(0) += 1;
        }
        CodeStats { tile_count: tiles.len(), total_lines: tiles.iter().map(|t| t.end_line.saturating_sub(t.start_line) + 1).sum(), kind_counts }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStats {
    pub tile_count: usize,
    pub total_lines: usize,
    pub kind_counts: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_rust_functions() {
        let d = CodeDecomposer::new(Language::Rust);
        let tiles = d.decompose("use std::io;\n\nfn hello() {\n    println!(\"hi\");\n}\n\nfn world(x: i32) -> i32 {\n    x + 1\n}\n");
        assert_eq!(tiles.iter().filter(|t| t.kind == CodeKind::Function).count(), 2);
    }
    #[test] fn test_rust_struct() {
        let d = CodeDecomposer::new(Language::Rust);
        let tiles = d.decompose("struct Foo { x: i32, y: i32 }\n");
        assert!(tiles.iter().any(|t| t.kind == CodeKind::Struct && t.name == "Foo"));
    }
    #[test] fn test_rust_enum() {
        let d = CodeDecomposer::new(Language::Rust);
        let tiles = d.decompose("enum Color { Red, Green, Blue }\n");
        assert!(tiles.iter().any(|t| t.kind == CodeKind::Enum && t.name == "Color"));
    }
    #[test] fn test_rust_impl() {
        let d = CodeDecomposer::new(Language::Rust);
        let tiles = d.decompose("impl Foo { fn bar(&self) {} }\n");
        assert!(tiles.iter().any(|t| t.kind == CodeKind::Impl));
    }
    #[test] fn test_python_functions() {
        let d = CodeDecomposer::new(Language::Python);
        let tiles = d.decompose("def hello():\n    print('hi')\n\ndef add(x, y):\n    return x + y\n");
        assert_eq!(tiles.iter().filter(|t| t.kind == CodeKind::Function).count(), 2);
    }
    #[test] fn test_python_class() {
        let d = CodeDecomposer::new(Language::Python);
        let tiles = d.decompose("class Dog:\n    def bark(self):\n        print('woof')\n");
        assert!(tiles.iter().any(|t| t.kind == CodeKind::Class && t.name == "Dog"));
    }
    #[test] fn test_typescript() {
        let d = CodeDecomposer::new(Language::TypeScript);
        let tiles = d.decompose("import { foo } from 'bar';\nfunction greet(name: string): void {\n    console.log(name);\n}\nclass App {\n    run() {}\n}\n");
        assert!(tiles.iter().any(|t| t.kind == CodeKind::Import));
        assert!(tiles.iter().any(|t| t.kind == CodeKind::Function));
    }
    #[test] fn test_go() {
        let d = CodeDecomposer::new(Language::Go);
        let tiles = d.decompose("package main\nimport \"fmt\"\nfunc main() {\n    fmt.Println(\"hi\")\n}\ntype Point struct {\n    X int\n    Y int\n}\n");
        assert!(tiles.iter().any(|t| t.kind == CodeKind::Function && t.name == "main"));
        assert!(tiles.iter().any(|t| t.kind == CodeKind::Struct && t.name == "Point"));
    }
    #[test] fn test_detect_rust() {
        assert_eq!(CodeDecomposer::detect_language("fn main() { let mut x = 1; }", None), Language::Rust);
    }
    #[test] fn test_detect_python() {
        assert_eq!(CodeDecomposer::detect_language("def hello():\n    import os\n", None), Language::Python);
    }
    #[test] fn test_detect_by_filename() {
        assert_eq!(CodeDecomposer::detect_language("", Some("main.rs")), Language::Rust);
        assert_eq!(CodeDecomposer::detect_language("", Some("app.py")), Language::Python);
        assert_eq!(CodeDecomposer::detect_language("", Some("index.ts")), Language::TypeScript);
    }
    #[test] fn test_filter_by_kind() {
        let d = CodeDecomposer::new(Language::Rust);
        let tiles = d.decompose("fn a() {}\nfn b() {}\nstruct C {}\n");
        let fns = CodeDecomposer::filter_by_kind(&tiles, &CodeKind::Function);
        assert!(fns.len() >= 2);
    }
    #[test] fn test_stats() {
        let d = CodeDecomposer::new(Language::Rust);
        let tiles = d.decompose("fn a() {}\nstruct B {}\nuse std::io;\n");
        let stats = CodeDecomposer::stats(&tiles);
        assert!(stats.tile_count >= 3);
    }
    #[test] fn test_generic() {
        let d = CodeDecomposer::new(Language::Unknown);
        let tiles = d.decompose("some random text\n\nmore text here\n\n");
        assert!(!tiles.is_empty());
    }
    #[test] fn test_empty() {
        let d = CodeDecomposer::new(Language::Rust);
        assert!(d.decompose("").is_empty());
    }
}
