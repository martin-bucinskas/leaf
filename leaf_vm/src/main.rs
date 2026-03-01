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

  let args: Vec<String> = std::env::args().collect();
  let exe_path = if args.len() > 1 {
    &args[1]
  } else {
    // "C:\\Users\\bucin\\RustroverProjects\\leaf\\leaf_asm\\fixtures\\out\\exe\\fibonacci.leafexe"
    "C:\\Users\\bucin\\RustroverProjects\\leaf\\leaf_asm\\new_fixtures\\09_complex_syscalls.leafexe"
  };

  let mut vm = VM::new(0x10000);
  let x = LeafAsmFile::read_from_path(exe_path)
    .expect("Failed to read ELF file");
  vm.load_program(&x);
  vm.run();
}
