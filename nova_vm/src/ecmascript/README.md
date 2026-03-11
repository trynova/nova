# ECMAScript

This folder contains the code for things mentioned directly in the
[ECMAScript language specification](https://tc39.es/ecma262/). As much as is
reasonable, the structure within this folder should be similar to the
specification text and code should reuse terms from the specification directly.

This is also conceptually the main entry point into the `nova_vm` library for
embedders.

## Values

The main ECMAScript `Value` type of Nova is found in
[`types/language/value.rs`](./types/language/value.rs) and is a public Rust
`enum`. The Nova `Value` API is thus fully open about its internal
representation, and the usage of pattern matching on `Value` in embedders is
explicitly encouraged.

Nova's `Value` variants always hold either an in-line stored payload directly,
or ["handle"](https://en.wikipedia.org/wiki/Handle_(computing)) to data stored
on the engine heap.

### Primitives

Nova's primitive types are implemented as follows.

1. `undefined` and `null` as payload-less variants on `Value`.

1. `true` and `false` as a `bool` carrying variant on `Value`.

1. Strings are encoded as [WTF-8] and have two variants on `Value`: short
   strings stored in-line, and longer strings heap-allocated and referenced
   though a handle.

1. Symbols have a single variant on `Value`, carrying a handle to heap-allocated
   data.

1. Numbers have three variants on `Value`: safe integers stored in-line,
   double-precision floating point values with 8 trailing zeroes stored in-line,
   and other numbers heap-allocated and referenced through a handle.

1. BigInts have two variants on `Value`: 56-bit signed values stored in-line,
   and larger values heap-allocated and referenced through a handle.

### Objects

All non-primitive ECMAScript values are objects. Object data is always
heap-allocated and referenced through a handle. Unlike most JavaScript engines,
Nova does not use pointers to refer to heap-allocated data and instead chooses
to use a combination of the handle's type and an index contained in the handle.
This means that all object data is allocated on the heap in dedicated typed
arenas, and accessing the data is done by offsetting based on the handle's
contained index.

## Crossreferencing

### 6. ECMAScript Data Types and Values

Found in the [`types`](./types/) folder.

### 7. Abstract Operations

Found in the [`abstract_operations`](./abstract_operation) folder.

### 8. Syntax-Directed Operations

This is more about the parsing so I am not sure if this needs to be in the
engine at all.

If this ends up being needed then it will be in a [`syntax`](./syntax/) folder.

### 9. Executable Code and Execution Contexts

Found in the [`execution`](./execution/) folder.

### 10. Ordinary and Exotic Objects Behaviours

Found in the [`builtins`](./builtins) folder.

### 11-15. ECMAScript Language, and 17. Error Handling and Language Extensions

For the parts concerning evaluation of the language syntax, these are mainly
found in the adjacent [`engine`](../engine) folder.

### 16. ECMAScript Language: Scripts and Modules

Found in the [`scripts_and_modules`](./scripts_and_modules/) folder.

### 18.-28. ECMAScript Standard Built-in Objects

Found in the [`builtins`](./builtins) folder.
