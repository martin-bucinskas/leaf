use std::{env, fs};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use clap::{Parser as ClapParser};
use log::{error, info};

mod ast;
mod assembler;
mod parser;

#[derive(ClapParser)]
#[command(author, version, about, long_about = None)]
struct Cli {
  /// Input assembly file
  #[arg(short, long)]
  input: String,

  /// Output bytecode file
  output: String,

  /// Increase logging verbosity (-v, -vv, etc.)
  #[arg(short, long, action = clap::ArgAction::Count)]
  verbose: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();

  let log_level = match cli.verbose {
    0 => "info",
    1 => "debug",
    _ => "trace",
  };

  unsafe {
    std::env::set_var("RUST_LOG", log_level);
  }

  env_logger::init();

  info!("Assembling {} into {}", cli.input, cli.output);

  let source = match fs::read_to_string(&cli.input) {
    Ok(s) => s,
    Err(e) => {
      error!("Failed to read input file: {}", e);
      return Err(e.into());
    }
  };

  let bytecode = match assembler::assemble_with_header(&source) {
    Ok(b) => b,
    Err(e) => {
      error!("Assembly error: {}", e);
      return Err(e.into());
    }
  };

  let mut file = match std::fs::File::create(Path::new(&cli.output)) {
    Ok(f) => f,
    Err(e) => {
      error!("Failed to create output file: {}", e);
      return Err(e.into());
    }
  };

  if let Err(e) = file.write_all(&bytecode) {
    error!("Failed to write bytecode: {}", e);
    return Err(e.into());
  }

  let instruction_count = source.lines()
    .filter(|l| !l.trim().is_empty()).count();

  info!("Successfully assembled {} instructions into {} bytes", instruction_count, bytecode.len());

  Ok(())
}
