use std::{fs::File, io::{BufReader, BufWriter, Read, Write}, path::Path};
use clap::{Parser as ClapParser, Subcommand};
use log::{info, error};
use crate::assembler::assemble::Assembler;
use crate::assembler::{LeafAsmFile, LeafAsmObjectHeader};
use crate::common::{ReadableResource, WriteableResource};
use crate::linker::linker::link;

mod ast;
mod parser;
mod linker;
mod assembler;
mod common;


/// Generate a header for a new object file
fn make_header() -> LeafAsmObjectHeader {
  LeafAsmObjectHeader {
    magic: *b"LAF\0",
    version: 1,
    reserved: 0,
    checksum: 0, // filled in during write_to
  }
}

#[derive(ClapParser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  /// Increase verbosity (-v, -vv, -vvv)
  #[arg(short, long, action = clap::ArgAction::Count)]
  verbose: u8,

  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// Assemble one or more .leaf files into .leafobj
  Assemble {
    /// Input file(s) to assemble
    #[arg(short, long, required = true)]
    inputs: Vec<String>,

    /// Output files (optional, same count as input)
    #[arg(short, long, required = false)]
    outputs: Option<Vec<String>>,
  },

  /// Link one or more .leafobj files into a single executable
  Link {
    /// Input object files to link
    #[arg(required = true)]
    inputs: Vec<String>,

    /// Output file for the linked executable
    #[arg(short, long, required = true)]
    output: String,

    /// Entry point for the executable
    #[arg(short, long, required = false)]
    entry: Option<String>,
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();

  // Set up logging level
  let log_level = match cli.verbose {
    0 => "info",
    1 => "debug",
    _ => "trace",
  };
  unsafe {
    std::env::set_var("RUST_LOG", log_level);
  }
  env_logger::init();

  match &cli.command {
    Command::Assemble { inputs, outputs } => {
      // Output file logic
      let output_files: Vec<String> = if let Some(out) = outputs {
        if out.len() != inputs.len() {
          error!("Number of outputs must match inputs");
          std::process::exit(1);
        }
        out.clone()
      } else {
        // Default: replace extension .leaf with .leafobj, or append .leafobj
        inputs.iter()
          .map(|f| {
            if let Some(stem) = Path::new(f).file_stem() {
              format!("{}.leafobj", stem.to_string_lossy())
            } else {
              format!("{}.leafobj", f)
            }
          })
          .collect()
      };

      for (input_path, output_path) in inputs.iter().zip(output_files.iter()) {
        // Read source
        let src = match std::fs::read_to_string(input_path) {
          Ok(s) => s,
          Err(e) => {
            error!("Failed to read {}: {}", input_path, e);
            continue;
          }
        };
        // Parse and assemble
        let program = match parser::parse_program(&src) {
          Ok(lines) => lines,
          Err(e) => {
            error!("Failed to parse {}: {}", input_path, e);
            continue;
          }
        };
        // Entry point: pick "main" if it exists, else None
        let entry_point = program.iter().filter_map(|l| match l {
          ast::Line::LabelOnly(l) => Some(l),
          _ => None,
        }).find(|l| l.as_str() == "main").map(|_| "main".to_string());
        let object = Assembler::assemble(&program, entry_point);

        let file = LeafAsmFile {
          header: make_header(),
          object,
        };
        let mut output_file = BufWriter::new(File::create(output_path)?);
        if let Err(e) = file.write_to(&mut output_file) {
          error!("Failed to write {}: {}", output_path, e);
        } else {
          info!("Assembled {} -> {}", input_path, output_path);
        }
      }
    }
    Command::Link { inputs, output, entry } => {
      // Read all input object files
      let mut objects = Vec::new();
      for in_path in inputs {
        let mut file = BufReader::new(File::open(in_path)?);
        let asm_file = match LeafAsmFile::read_from(&mut file) {
          Ok(obj) => obj,
          Err(e) => {
            error!("Failed to read {}: {}", in_path, e);
            std::process::exit(1);
          }
        };
        objects.push(asm_file.object);
      }
      let entry_name = entry.clone().unwrap_or_else(|| "main".to_string());
      let linked = match link(&objects, &entry_name) {
        Ok(obj) => obj,
        Err(e) => {
          error!("Linking failed: {}", e);
          std::process::exit(1);
        }
      };
      let file = LeafAsmFile {
        header: make_header(),
        object: linked,
      };
      let mut out_file = BufWriter::new(File::create(output)?);
      if let Err(e) = file.write_to(&mut out_file) {
        error!("Failed to write output file: {}", e);
        std::process::exit(1);
      } else {
        info!("Linked {} object(s) into {}", inputs.len(), output);
      }
    }
  }
  Ok(())
}
