# Architecture

The architecture of Nova engine is built around data-oriented design. This means
that most "data" like things are found in the heap in a vector with like-minded
individuals.

## Concurrency

Currently the heap is not thread-safe at all but the plan is to make it
thread-safe enough for concurrent marking to become possible in the future. To
this end, here's how I (@aapoalas) envision the engine to look like in the
future:

```rs
struct Agent<'agent, 'generation>(Box<AgentInner<'agent>>, AgentGuard<'generation>);
```

The `Agent` struct here is just a RAII wrapper around the inner Heap-allocated
Agent data. The first important thing is the `'agent` lifetime: This is a brand.
It is valid for as long as the Nova engine instance lives, and its only purpose
is to make sure that (type-wise) uses cannot mistakenly or otherwise mix and
match Values from different engine instances.

The second lifetime, `'generation`, is the garbage collection generation. Here
what I want to achieve is a separation between "gc" and "nogc" scopes. But
before we dive into that, here's what I imagine a `Value` looking like:

```rs
enum Value<'generation> {
    Undefined,
    String(StringIndex<'generation>>),
    SmallString(SmallString),
    // ...
}
```

The `Value` enum carries the `'generation` lifetime: As long as we can guarantee
that no garbage collection happens, we can safely keep `Value<'gen>` on the
stack or even temporarily on the heap.

If we call a method that may trigger GC, then all `Value<'gen>` items are
invalidated. If we want to keep values alive through eg. JavaScript function
calls, we must use:

```rs
struct ShadowStackValue<'agent>(u32, PhantomPinned);
```

This just moves the `Value` onto an Agent-controlled "shadow stack" that the
`u32` points into. Due to the `PhantomPinned` the shadow stack is mostly just
push-pop as any stack should be, and thus relatively quick. But it is also on
the heap and thus garbage collection can update any references on the shadow
stack.

Note that this is essentially equivalent to:

```rs
struct GlobalValue<'agent>(u32);
```

but "global values" are not push-pop, likely will have generational indexes,
possibly will have reference counting and so on and so forth.
