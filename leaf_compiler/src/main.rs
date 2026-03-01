use std::fs;
use std::path::PathBuf;
use clap::Parser;
use leaf_compiler::{LeafParser, Rule};
use pest::Parser as PestParser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input Leaf source file
    #[arg(short, long)]
    input: PathBuf,

    /// Output assembly file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    let mut visited = std::collections::HashSet::new();
    let program = leaf_compiler::compile_file(&args.input, &mut visited);
    
    let mut codegen = leaf_compiler::codegen::CodeGenerator::new();
    let asm = codegen.generate(&program);

    if let Some(output_path) = args.output {
        fs::write(output_path, asm).expect("Failed to write output file");
    } else {
        println!("{}", asm);
    }
}
