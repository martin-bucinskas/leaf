pub mod ast;
pub mod parser;
pub mod codegen;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use pest::Parser as PestParser;
use pest_derive::Parser;
use crate::ast::Program;

#[derive(Parser)]
#[grammar = "grammar/leaf.pest"]
pub struct LeafParser;

pub fn compile_file(path: &Path, visited: &mut HashSet<PathBuf>) -> Program {
    let absolute_path = fs::canonicalize(path).expect(&format!("Failed to canonicalize path: {:?}", path));
    if visited.contains(&absolute_path) {
        return Program {
            includes: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
        };
    }
    visited.insert(absolute_path.clone());

    let content = fs::read_to_string(&absolute_path).expect(&format!("Failed to read file: {:?}", absolute_path));
    let pair = LeafParser::parse(Rule::program, &content)
        .expect(&format!("Failed to parse file: {:?}", absolute_path))
        .next()
        .unwrap();

    let mut program = crate::parser::parse_program(pair);
    let mut all_globals = Vec::new();
    let mut all_functions = Vec::new();

    let parent_dir = absolute_path.parent().unwrap();

    for include in &program.includes {
        let include_path = if include.starts_with("@std/") {
            let std_name = &include[5..];
            // Look for std in leaf_compiler/resources/std
            let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            p.push("resources");
            p.push("std");
            p.push(format!("{}.leafc", std_name));
            p
        } else {
            parent_dir.join(include)
        };

        let included_program = compile_file(&include_path, visited);
        all_globals.extend(included_program.globals);
        all_functions.extend(included_program.functions);
    }

    // Merge current program's elements
    all_globals.extend(program.globals);
    all_functions.extend(program.functions);

    program.globals = all_globals;
    program.functions = all_functions;

    program
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    #[test]
    fn test_parse_simple_program() {
        let input = r#"
            fn main() -> int {
                int x = 10;
                print(x);
                return 0;
            }
        "#;
        let result = LeafParser::parse(Rule::program, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_factorial() {
        let input = r#"
            fn factorial(int n) -> int {
                if (n <= 1) {
                    return 1;
                }
                return n * factorial(n - 1);
            }

            fn main() -> int {
                int result = factorial(5);
                print(result);
                return 0;
            }
        "#;
        let result = LeafParser::parse(Rule::program, input);
        assert!(result.is_ok(), "Failed to parse factorial: {:?}", result.err());
    }

    #[test]
    fn test_ast_conversion() {
        let input = r#"
            fn factorial(int n) -> int {
                if (n <= 1) {
                    return 1;
                }
                return n * factorial(n - 1);
            }

            fn main() -> int {
                int result = factorial(5);
                print(result);
                return 0;
            }
        "#;
        let pair = LeafParser::parse(Rule::program, input).unwrap().next().unwrap();
        let program = crate::parser::parse_program(pair);
        assert_eq!(program.functions.len(), 2);
        assert_eq!(program.functions[0].name, "factorial");
        assert_eq!(program.functions[1].name, "main");
    }

    #[test]
    fn test_codegen() {
        let input = r#"
            fn main() -> int {
                int x = 5;
                int y = 10;
                print(x + y);
                return 0;
            }
        "#;
        let pair = LeafParser::parse(Rule::program, input).unwrap().next().unwrap();
        let program = crate::parser::parse_program(pair);
        let mut codegen = crate::codegen::CodeGenerator::new();
        let asm = codegen.generate(&program);
        assert!(asm.contains("main:"));
        assert!(asm.contains("SYSCALL"));
        println!("{}", asm);
    }
}
