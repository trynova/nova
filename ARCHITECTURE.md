# Architecture

The architecture of Nova engine is built around data-oriented design. This means
that most "data" like things are found in the heap in a vector with like-minded
individuals.

## ECMAScript implementation

Nova code aims to conform fairly strictly to the
[ECMAScript specification](https://tc39.es/ecma262/) in terms of both code
layout and structure.

For details on the ECMAScript implementation, see the
[ecmascript/README.md](./nova_vm/src/ecmascript/README.md).

## Engine implementation

Nova's VM is a stack-based bytecode interpreter.

For details on the engine, see the
[engine/README.md](./nova_vm/src/engine/README.md).

## Heap implementation

Nova's heap is made up of a mix of normal Rust `Vec`s and custom `SoAVec`
structs that implement a "Struct of Arrays" data structure with an API
equivalent to normal `Vec`s, all referenced by-index.

For details on the heap architecture, see the
[heap/README.md](./nova_vm/src/heap/README.md).
