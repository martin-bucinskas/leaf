use leaf_common::leaf_file::LeafAsmFile;
use leaf_common::ReadableResource;
use crate::vm::VM;

mod vm;

fn main() {
  // Set up logging level
  let log_level = "info";
  unsafe {
    std::env::set_var("RUST_LOG", log_level);
  }
  env_logger::init();

  // let mut vm = VM::new(0x10000);
  let mut vm = VM::new(0x100);
  // let x = LeafAsmFile::read_from_path("C:\\Users\\bucin\\RustroverProjects\\leaf\\leaf_asm\\fixtures\\out\\exe\\fibonacci.leafexe")
  let x = LeafAsmFile::read_from_path("C:\\Users\\bucin\\RustroverProjects\\leaf\\leaf_asm\\fixtures\\out\\exe\\all.leafexe")
    .expect("Failed to read ELF file");
  vm.load_program(&x);
  vm.run();
}
