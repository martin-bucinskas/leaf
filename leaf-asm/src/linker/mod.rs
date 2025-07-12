pub mod linker;

use std::fs;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LinkerFile {
  pub input_files: Vec<String>,
  pub output_file: String,
  pub entry_point: Option<String>,
}

pub fn parse_linker_file<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<LinkerFile> {
  let content = fs::read_to_string(path)
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
  toml::from_str(&content)
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
