#!/usr/bin/env bash

echo "Running tests for leaf-asm..."

cargo run -- assemble \
  --inputs fixtures/simple.leaf fixtures/caller.leaf fixtures/callee.leaf fixtures/jump.leaf fixtures/data_and_rodata.leaf \
  --outputs fixtures/out/simple.leafobj fixtures/out/caller.leafobj fixtures/out/callee.leafobj fixtures/out/jump.leafobj fixtures/out/data_and_rodata.leafobj

cargo run -- assemble --inputs fixtures/simple.leaf fixtures/caller.leaf fixtures/callee.leaf fixtures/jump.leaf fixtures/data_and_rodata.leaf --outputs fixtures/out/simple.leafobj fixtures/out/caller.leafobj fixtures/out/callee.leafobj fixtures/out/jump.leafobj fixtures/out/data_and_rodata.leafobj

cargo run -- assemble --inputs fixtures/simple.leaf --outputs fixtures/out/simple.leafobj
cargo run -- assemble --inputs fixtures/caller.leaf --outputs fixtures/out/caller.leafobj
cargo run -- assemble --inputs fixtures/callee.leaf --outputs fixtures/out/callee.leafobj
cargo run -- assemble --inputs fixtures/jump.leaf --outputs fixtures/out/jump.leafobj
cargo run -- assemble --inputs fixtures/data_and_rodata.leaf --outputs fixtures/out/data_and_rodata.leafobj

cargo run -- link --output fixtures/out/exe/simple.leafexe fixtures/out/simple.leafobj
cargo run -- link --output fixtures/out/exe/caller_callee.leafexe fixtures/out/caller.leafobj fixtures/out/callee.leafobj
cargo run -- link --output fixtures/out/exe/jump.leafexe fixtures/out/jump.leafobj
cargo run -- link --output fixtures/out/exe/data_and_rodata.leafexe fixtures/out/data_and_rodata.leafobj
cargo run -- link --output fixtures/out/exe/all.leafexe fixtures/out/simple.leafobj fixtures/out/caller.leafobj fixtures/out/callee.leafobj fixtures/out/data_and_rodata.leafobj