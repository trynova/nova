In this document, at present, "I" refers to @aapoalas.

# Heap design

The foremost design principle of the Nova engine heap is (what to me passes as)
Data Oriented Design. DOD emphasises knowing your problem space and your data,
common actions on it, and choosing your data structures based on that. I am not
the greatest expert but I am hoping I know my JavaScript well enough both from a
language, programmer, and to some degree an engine point of view that I have a
good idea of the data and common actions on it.

So what are common actions that a JavaScript runtime does?

- There's all the boring stuff like arithmetic operations, boolean logic, etc.
- Creating objects and arrays is very common.
- Checking for properties in an object is very common.
- Object access is very common.
- Creating and calling functions is very common.
- Creating other builtin objects is quite common. (That is: Date, ArrayBuffer,
  RegExp, ...)
- Object property manipulation is common as well.
- Adding new object properties is fairly common.
- Checking the length of an array.
- Iterating arrays and to some degree even objects is very common as well.

What are some uncommon actions?

- Deleting of object properties. It is done, yes, but rarely do you see objects
  used as hashmaps anymore.
- Accessing or defining property descriptors.
- Accessing or assigning non-element (indexed) properties on arrays.
- Calling getter or setter functions. This happens but again is not the most
  common path.
- Checking the length of a function.
- Accessing or assigning properties on functions. This does happen, but it is
  infrequent.
- Accessing or assigning properties on ArrayBuffers, Uint8Arrays, DataViews,
  Dates, RegExps, ... Most builtin objects are used for their named purpose only
  and beyond that are left alone.
- Checking the prototype of an object. This is done indirectly with
  hashmap-objects and with method calls but most engines try to minimize these
  as much as possible with various tricks. The best object is often the one
  where you never miss a key access.
- Changing the prototype of an object.

So what we can gather from this is that

1. Objects mostly need good access to their keys and values. Their prototype is
   somewhat secondary, and with hidden classes / shapes the keys actually become
   somewhat secondary as well.
2. Property descriptors are not very important.
3. Function calls are very important.
4. Quick iteration over array elements, object keys, and object values are quite
   important.

From an engine standpoint the only common action that is done outside of
JavaScript is the garbage collection. The garbage collection mostly cares about
quick access to JavaScript heap values and, if possible, efficient iteration
over them.

Thus our heap design starts to form. To help the garbage collection along we
want to place like elements one after the other. As much as possible, we want to
give the CPU an easy time in guessing where we'll be accessing data next. This
then means both that vectors are our friend but also we want to avoid vectors of
`dyn Trait` and instead want vectors of concrete structs even if we end up with
more vectors.

To avoid unnecessary memory usage for eg. arrays, ArrayBuffers, RegExps etc.
we'd want to avoid creating their "object" side if its not going to be used.
This then means that the "object" side of these must be separate from their
"business" side. An ArrayBuffer will need to have a pointer to a raw memory
buffer somewhere and that is its most common usage so it makes sense to put that
in its "business" side but we do not need to keep its key-value pairs close at
hand.

So we put object base heap data one after the other somewhere. We could refer to
these by pointer but that takes 8 bytes of memory and Rust doesn't particularly
like raw pointers, nor does it like `&'static` references. So direct references
are not a good idea. If we use heap vectors we can instead use indexes into
these vectors! A vector is `usize` indexed but a JS engine will never run into 4
million objects or numbers or strings. A `u32` index is thus sufficient.

It then makes sense to put other heap data in vectors as well. So we have
indexes to the vector of ArrayBuffers, indexes to the vector of objects, etc.
The question then becomes where are these vectors kept, and that is then the
heap struct. The heap becomes a collection of vectors that are, at least for
now, simply allocated by the default allocator and thus can be anywhere in
memory. This means that objects created one after another will find themselves
one after another in the vector of object heap data, and their property values
are also likely to reside one after the other. We (hope to) get good cache
locality!

## Comparing

How do other engines structure their heap? Here I am going to compare our
vectored heap to V8 engine and the other Rust-based JavaScript engine, Boa.

First V8: I do not know how exactly V8 structures its heap. I know a few things
though:

1. The V8 heap is built around a slab of allocated memory around a base pointer
   [[1](https://v8.dev/blog/pointer-compression)].
1. Objects in V8's heap are always prepared to accept properties.
1. Objects in V8 can have varying sizes to accommodate inline property values.
1. Objects refer to other objects in the V8 heap using pointers or, with pointer
   compression, 32-bit offsets around the base pointer.
1. Objects in V8's heap are not necessarily allocated side-by-side.
1. V8's garbage collection is based on a tracing mark-and-sweep algorithm.

Then Boa: Boa uses a modified version of the [gc](https://crates.io/crates/gc)
crate which is built around reference counting and created using basic Rust
structs. Thus it follows that:

1. The Boa heap is not located in a defined area. Each heap object is allocated
   separately according to the whims of the allocator. I believe Boa does use
   jemalloc which means that allocations are reused and allocated elements are
   generally placed in the same area, though.
1. Objects in Boa's heap are always statically sized and seem to rely on traits
   and/or static vtable references to implement most if not all of their inner
   workings.
1. Objects refer to other objects using pointers.
1. Boa's garbage collection is reference counting.

### Advantages and disadvantages of Boa

There are some advantages to how Boa does things.

1. It is much simpler to write. Letting the allocator take care of placing your
   objects means you do not need to worry about it.
1. Implementing garbage collection is left to a 3rd party crate and you mostly
   do not need to worry about it.

Disadvantages are also of course present.

1. Objects may be allocated all over the place with no regards to cache
   locality.
1. Any `dyn Trait` accessing and vtable usage incurs an indirection that the CPU
   has to spend time resolving.
1. All intra-heap references are 8 bytes in size.
1. Reference counting cannot collect cycles.

### Advantages and disadvantages of V8

V8's first and foremost advantage is of course being backed by Google and having
had tons and tons of work poured into it. But let's talk about the technical
advantages.

1. Varying object sizes means that oft-encountered object shapes can become
   essentially plain `struct`s with minimal indirection from the object to its
   values.
1. Pointer compression enables V8 to save its internal heap references in 4
   bytes of memory.

Still, V8 does have some disadvantages as well.

1. Varying object sizes mean that objects cannot live side-by-side in harmony
   and must instead be allocated at least some distance from one another in the
   general case.
1. Since each object is prepared to accept properties, it means that the object
   struct size is relatively large for what it's doing. A single `Uint8Array` is
   72 bytes in size, when its most important usage is being a boxed slice of up
   to 2^32 elements: That can be done in 16 bytes with padding.

### Advantages and disadvantages of Nova

So, what can we expect from Nova's heap? First the advantages:

1. All objects are of the same size and can thus be placed in a vector,
   benefiting from cache locality and amortisation of allocations.
1. All references to and within the heap data can be done with a 32 bit integer
   index and a type: The type tells which array to access from and the index
   gives the offset.
1. Placing heap data "by type and use" in a DOD fashion allows eg. `Uint8Array`s
   to not include any of the data that an object has
1. Garbage collection must be done on the heap level which quite logically lends
   itself to a tracing mark-and-sweep algorithm.
1. With careful work it should be possible to enable partially concurrent
   marking of the heap.

There are disadvantages as well.

1. Either no objects carry inline properties and thus any property access must
   always take an extra pointer indirection, or all objects carry inline
   properties even if they're not used. This is may be somewhat offset by key
   checks requiring object shape access by pointer anyhow and the two reads are
   not 100% dependent on one another (conditional on the number of elements).
1. Heap vectors need reallocation when growing. This may prove to be such a
   performance demerit that it requires changing from heap vectors into vectors
   of heap chunks, trading reallocation need for worse cache locality.
1. Garbage collection must be done on the heap level and implemented manually!

## Ownership, Rust and the borrow checker

We've established that our heap will consist of vectors that contain heap data
that refer to one another using a type + index pair (an enum, essentially). The
question one might then ask is, what does the Rust borrow checker think of this?

There are two immediate answers:

1. The borrow checker does not like this at all.
1. The borrow checker does not care about this at all.

These two seem to be in direct confrontation with one another so let's explain a
bit more. First: JavaScript's ownership model is not directly compatible with
Rust's ownership model. When a JavaScript object contains another object, we
cannot represent that as a Rust borrow such as `struct A { key: &'b B }`.

From Rust's point of view the most direct way to represent these contains
relations would be something like `Rc<RefCell<A>>`. This is then similar to the
`gc` crate that Boa uses.

We do not want to do reference counting, so that way is barred to us. But we
cannot do references either, the borrow checker will not allow such a thing. So
it seems like the borrow checker does not like what we're doing at all.

But remember, we're not doing references (ie. pointers), we're doing indexes. So
instead of the above `struct A` with an internal reference we have
`struct A { key: (u8, u32) }` where the `u8` is the type information and `u32`
the index. These imply no referential ownership and thus the borrow check does
not care at all.

So, did we just turn off the borrow checker? Kind of yes. Is this supposed to be
safe? Yes and no. From the borrow checker point of view, and from the engine
point of view, the ownership of each object is clear: The heap owns them plain
and simple. Only the heap allows accessing those objects, and only the heap
allows changing those objects. So we did not turn off the borrow checker, we
just explained to it the ownership as it pertains to the engine instead of the
JavaScript code running inside the engine.

But this does also mean that we're now in charge of tracking the JavaScript-wise
ownership of objects ourselves: The borrow checker will not give us a helping
hand with that. This means that it's possible that we create bugs that from
JavaScript's point of view sense are use-after-free or similar memory safety
related errors. The borrow checker will just make sure that we're not causing
actual memory corruption with this, even if we do cause heap corruption.

That is exactly how we want it to be as well: As said, JavaScript's ownership
model cannot be represented in a way that would satisfy the Rust borrow checker
and thus it would be a waste of time to even try.

So summing up: Our heap model does not try to present the JavaScript ownership
model to Rust's borrow checker in any way or form. Instead we represent to the
borrow checker the engine's ownership model: The heap owns all of the data it
contains and manages that data's lifetime according to its own whims.

## Garbage collection

Above I mentioned that our garbage collection is a tracing collection algorithm.
It's also intended to be a compacting GC. Not that it properly exists or works
yet.

Here're the broad strokes of it:

1. Starting with roots (global values as well as local values currently held by
   the mutator thread, ie. the JavaScript thread), mark the heap data entries
   corresponding to those roots. Marks are entered in a separate vector of mark
   bytes (or possibly bits).
2. For each marked entry, trace all its referred heap data entries. Recurse.
3. Once no more work is left to be done, walk through the vector of mark bytes
   and note each unmarked slot in the heap vectors. Gather up a list of
   compactions (eg. Starting at index 30 in object heap vector, shift down 2,
   starting at index 45 shift down 3, ...) and the number of marked elements
   based on this walk. Then walk through each heap vector based on these
   compaction lists and shift elements down (copy within) in the vector. Once
   all shifts are done, set the length of the heap vector to the number of
   marked elements.
4. While walking each heap vector, every internal reference in the heap must
   have its index potentially shifted according to the list of compactions as
   well. (eg. If an array contains an object with its heap data at index 32,
   then according to the example given above its index must be shifted down by
   two, becoming 30.)

This compacting algorithm is where the largest risks for use-after-free and
similar errors lie. An off-by-one error or a missed shift of a reference index
will cause the JavaScript heap to become corrupted. At best this may lead to a
crash of the engine from eg. trying to access an element beyond a heap vector's
length. At worst it will lead to simply inexplicable runtime behaviour in
JavaScript.

A further complication will be concurrent tracing of the heap. It would be very
beneficial if possibly multiple garbage collector threads could run concurrently
with the mutator thread (JavaScript runtime), performing marking of reachable
heap elements. The mutator thread must consequently mark as dirty any
already-marked elements when it mutates them, and before a sweep is started all
dirty elements must be re-traced from while the mutator thread is paused (stop
the world).

The compaction of the heap can also be done in a parallel manner: Every heap
vector's unmarked slots can be gathered up into a list of compactions
parallelly. These compaction lists must be combined into a complete whole after
which each heap vector can then be compacted parallelly.

The complicated part is making sure that concurrent tracing of the heap is
actually safe. Rust will help us making sure of this but there are things that
need to be done manually as well. The most important thing is the heap vectors:
At any time the mutator thread can end up needing to grow a vector which means a
reallocation. This can be done safely with an RCU synchronization mechanism but
does mean that it needs to be done.
