pub trait WriteableResource {
  fn write_to(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()>;

  fn write_to_path<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
      let mut file = std::fs::File::create(path)?;
      self.write_to(&mut file)
  }
}

pub trait ReadableResource {
  fn read_from(reader: &mut dyn std::io::Read) -> std::io::Result<Self>
  where
    Self: Sized;

  fn read_from_path<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<Self>
  where
    Self: Sized,
  {
      let mut file = std::fs::File::open(path)?;
      Self::read_from(&mut file)
  }
}
