# Architecture

The architecture of Nova engine is built around data-oriented design. This means
that most "data" like things are found in the heap in a vector with like-minded
individuals. Data-oriented design is all the rage on the Internet because of its
cache-friendliness. This engine is one more attempt at seeing what sort of
real-world benefits one might gain with this sort of architecture.

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
equivalent to normal `Vec`s, all referenced by index using the public API's
handle types. The eventual aim is to store everything in Structs of Arrays, with
a smattering of keyed side-tables (hash maps or b-trees) on the side to hold
optional data.

The intention here is to make it fast for the computer to access frequently used
things while allowing infrequently used things to stay out of the hot path, and
enabling rarely used optional parts of structures to take little or no memory at
all to store at the cost of access performance. For details on the heap
architecture, see the [heap/README.md].

[heap/README.md]: ./nova_vm/src/heap/README.md
