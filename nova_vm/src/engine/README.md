# Engine

This folder contains engine-specific details such as the bytecode representation
and interpreter. The Nova VM is a fairly simple stack-based bytecode
interpreter. The interpreter loop revolves around the
[`struct
Vm`](./bytecode/vm.rs:86:12).

## The `Vm` struct

The `Vm` struct is made up of the following parts:

1. Instruction pointer: this tracks the progress of the interpreter in the
   referenced bytecode buffer.

1. Result register: a single `Option<Value>` that holds the last expression or
   statement result, or `None`. When the previous result needs to be recalled
   later, it is pushed onto the Value stack.

1. Reference register: a single `Option` of a [Reference Record] that holds the
   last "place expression" (an expression `a` that can appear on the left side
   of an assignment operation `a = b`) to be evaluated, or `None`. When a
   reference needs to be recalled later, it is pushed onto the Reference stack.

1. Value stack: a `Vec<Value>` that is used to store unnamed variables as
   needed. The stack is also used for storing non-escaping named variables as an
   optimisation.

1. Reference stack: a `Vec` of [Reference Records][Reference Record] that is
   used by the interpreter to store references for later use.

1. Exception handler stack: a `Vec` of installed exception handlers. These are
   produced by try-catch blocks and async iterators, and are removed upon
   exiting the associated block.

1. Iterator stack: a `Vec` of iterators currently being processed. These are
   produced by `for-of` and `for-in` iterators and are removed upon exiting the
   loop.

Contrary to the common (and the most obvious and efficient) way of building a
VM, Nova does not have a single `Vm` struct per `Agent` (engine instance) that
gets reused between calls. Instead, the engine creates a new `Vm` on the native
stack on every function call. This is not an architectural decision per se, but
rather a historical one that ought to be fixed at some point.

Because the `Vm` structs are stored on the stack, they must be explicitly rooted
when calling into methods that might trigger garbage collection (methods that
take `GcScope`). A helper function `with_vm_gc` is provided for this purpose.

In addition to the `Vm` structs, ECMAScript execution per specification is
defined by the [Execution Contexts][Execution Context]. These are stored in a
stack in the `Agent`, and are currently considered entirely separate from the
`Vm` structs despite being very closely related in actual fact.

### Running ECMAScript code in a `Vm`

ECMAScript code must be compiled into bytecode to be executable; such a compiled
bytecode is stored on the `Agent` heap separately and is referenced using the
handle type `Executable`. An `Executable` can then be executed by calling the
`Vm::execute` static method, and will return one of four result variants:

1. `Return`: the code executed successfully and returned a result. This is
   produced by both the `return` statement and an executable reaching the end of
   the bytecode buffer (implicit `return`).

1. `Throw`: the code executed unsuccessfully and threw an error. This is
   produced by the `throw` statement.

1. `Await`: the code could not finish execution and has to await a `Promise`
   before continuing. The variant includes both the `Promise` to await and a
   suspended variant of the `Vm` struct. This is produced by the `await`
   expression and async iterators.

1. `Yield`: the code requested to yield to the caller. The variant includes both
   the `Value` to yield and a suspended variant of the `Vm` struct. This is
   produced by the `yield` expression.

The `struct SuspendedVm` returned by `Await` and `Yield` variants must be stored
on the `Agent` heap and resumed at a later point.

### Compiling ECMAScript code for executing

ECMAScript code can be compiled using one of the four `compile` APIs on the
`Executable` handle type:

1. `compile_script`: for compiling [`Script`s][Script].
1. `compile_module` for compiling ECMAScript [Modules][Module].
1. `compile_function_body` for compiling ECMAScript [Functions][Function].
1. `compile_eval_body` for compiling [`eval()` scripts][eval].

Once compiled, the `Executable` handle is used to refer to the bytecode stored
on the `Agent` heap. If the `Executable` is itself not stored on the heap (such
as stored in an `ECMAScriptFunction`'s heap data) then the bytecode will
eventually be garbage collected.

It is worth noting that at present closures are compiled anew on every use as
the bytecode gets stored in the `ECMAScriptFunction` instance and is not shared
between multiple function instances even though they originate from the same
code:

```typescript
array.filter((v) => v > 10); // one function instance, compiled once

for (let i = 0; i < N; i++) {
  array.filter((v) => v > 10); // one function instance per loop, compiled N times
}
```

This is again not an architectural choice, but simply a historical happenstance
that ought to be fixed.

## Bytecode format

Nova VM's bytecode is a variable-width, high-level bytecode. It does not offer
any instructions for direct heap memory manipulation, but offers a set of
base-level instructions for manipulating the state of the `Vm` struct, and
beyond that provides many ECMAScript specific instructions that perform more
complicated actions that would not be possible to implement on just the `Vm`
struct.

Each bytecode instruction is always made up of a single instruction byte which
is then followed by 2 or 4 bytes of data, depending on the instruction. The data
bytes are interpreted as either `bool`s, `u16` values, or a single `u32` value
depending on the instruction. Note that the data is not aligned and cannot be
directly reinterpreted as a `u16` or `u32`.

## `Vm` dispatch loop

The `Vm` dispatch loop is found in the `Vm::inner_execute` method and consists
of a `while let Some()` loop "consuming" bytecode instructions (by moving the
`Vm`'s instruction pointer forward) from the `Executable`'s bytecode buffer.
This loop also contains the _only_ point in the engine where garbage collection
may be triggered. There is no particular reason why this should be the only
point where GC is checked and triggered, but currently it happens to be so.

[eval]: https://tc39.es/ecma262/#sec-eval-x
[Execution Context]: https://tc39.es/ecma262/#sec-execution-contexts
[Function]: https://tc39.es/ecma262/#sec-ecmascript-function-objects
[Module]: https://tc39.es/ecma262/#sec-source-text-module-records
[Reference Record]: https://tc39.es/ecma262/#sec-reference-record-specification-type
[Script]: https://tc39.es/ecma262/#sec-scripts
