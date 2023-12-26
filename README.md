# Nova - Your favorite javascript and wasm engine

## :warning: This project is a Work In Progress, and is very far from being suitable for use :warning:

Nova is a [JavaScript](https://tc39.es/ecma262) and
[WebAssembly](https://webassembly.org) engine written in Rust.

The engine is exposed as a library with an API for implementation in Rust
projects which themselves must serve as a runtime for JavaScript code.

The core of our team is on our [Discord server](https://discord.gg/RTrgJzXKUM).

## List of active development ideas

This list serves as a "this is where you were" for returning developers as well
as a potential easy jumping-into point for newcompers.

- Write implementations of more abstract operations
  - See `nova_vm/src/ecmascript/abstract_operations`
  - Specifically eg. `operations_on_objects.rs` is missing multiple operations,
    even stubs.
- Write implementations of class abstract operations
  - String, Number, ...
- Start `nova_vm/src/syntax` folder for
  [8 Syntax-Directed Operations](https://tc39.es/ecma262/#sec-syntax-directed-operations)
  - This will serve as the bridge between oxc AST, Bytecode, and Bytecode
    interpretation

Some more long-term prospects and/or wild ideas:

- Reintroduce lifetimes to Heap if possible
  - `Value<'gen>` lifetime that is "controlled" by a Heap generation number:
    Heap Values are valid while we can guarantee that the Heap generation number
    isn't mutably borrowed. This is basically completely equal to a scope based
    `Local<'a, Value>` lifetime but the intended benefit is that the
    `Value<'gen>` lifetimes can also be used during Heap compaction: When Heap
    GC and compaction occurs it can be written as a transformation from
    `Heap<'old>` to `Heap<'new>` and the borrow checker would then help to make
    sure that any and all `T<'new>` structs within the heap are properly
    transformed to `T<'new>`.
- Add a `Reference` variant to `Value` (or create a `ValueOrReference` enum that
  is the true root enum)
  - ReferenceRecords would (maybe?) move to Heap directly. This might make some
    syntax-directed operations simpler to implement.
- Add `DISCRIMINANT + 0x80` variants that work as thrown values of type
  `DISCRIMINANT`
  - As a result, eg. a thrown String would be just a String with the top bit set
    true. This would stop Result usage which is a darn shame (can be fixed with
    Nightly features). But it would mean that returning a (sort of)
    `Result<Value>` would fit in a register.
- Consider a register based VM instead of going stack based
