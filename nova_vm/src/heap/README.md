In this document, at present, "I" refers to @aapoalas.

# Heap design

The foremost design principle of the Nova engine heap is (what to me passes as)
Data Oriented Design. DOD emphasises knowing your problem space and your data,
common actions on it, and choosing your data structures based on that. I am not
the greatest expert but I am hoping I know my JavaScript well enough both from a
language, programmer, and to some degree an engine point of view that I have a
good idea of the data and common actions on it.

So what are common actions that a JavaScript runtime does?

- There's all the boring stuff like arithemtic operations, boolean logic, etc.
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
- Accessing or assinging non-element (indexed) properties on arrays.
- Checking the length of a function.
- Accessing or assigning propertes on functions. This does happen, but it is
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
