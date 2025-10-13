# Contributing

Hello, welcome and thank you for showing interest in contributing to Nova!
Contributions are absolutely welcome, wanted, and dearly desired. That being
said, before you being, you'll want to know a few things.

1. @aapoalas will likely leave a lot of PR comments. He's not doing it to be
   evil or anything, he's just an idiot.
2. Nova's code follows the ECMAScript specification. When in doubt, read, copy,
   implement the specification.
3. Testing is mainly based upon the test262 conformance suite. Updating
   conformance results needs to be done.

More information is found below.

## Pull Request Code of Conduct

The following ground rules should be followed:

1. Be courteous.
1. Be respectful.
1. Be your best self, and assume the best of everyone.

Feel free to also assume Rust Code of Conduct.

### Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)

The initial commit of a PR branch and the PR title should follow Conventional
Commits. Consider also using the same logic in branch names. The commonly used
prefixes here are `fix`, `feat`, and `chore`. Other recommended ones like
`build`, `perf`, and `refactor` are of course good as well.

Scoping is also recommended, but is not currently clearly defined. Some examples
are:

1. `feat(ecmascript)`: This is an added feature to the spec-completeness of the
   engine, eg. a new abstract operation, heap data object or such.
1. `fix(heap)`: This fixes something in the heap implementation, eg. maybe the
   heap garbage collection.
1. `feat(vm)`: This adds to the interpreter.
1. `chore(cli)`: This might bump a dependency in the `nova_cli` crate.

### Use [Conventional Comments](https://conventionalcomments.org/)

When reviewing PRs, use conventional comments to plainly describe your
intention. Prefixing a comment with `issue:` means that you do not think the PR
can be merged as-is and needs fixing. `issue (nonblocking):` means that you
think there is a problem in the code, but it can (possibly should) be fixed as a
followup or at a later date, and does not block merging. `nitpick:` is your
personal hobby-horse, `question:` is pure curiosity or not understanding the
code, etc.

Also: Whenever possible, give `praise:`! Praising the code and hard work of
others makes you feel good, and probably makes them feel good as well. Even if
it's a minor thing, like someone drive-by fixing that typo that bugged you the
other day, or cleaning up a weird construct into something a bit nicer, or
whatever: Even if it's not directly related to the focal point of the PR, praise
the work others do.

By all this it goes to say: When someone gives you `praise:`, they mean it. When
someone marks down an `issue:` they do not mean that your code is bad, they just
mean that there's something there to improve.

### Align with the [ECMAScript specification](https://tc39.es/ecma262/)

Nova's code and folder structure follows the ECMAScript specification as much as
possible. This means that when you need to implement a new abstract operation
from the specification, your best course of action is generally to copy the
specification text as a comment into the file inside the
`nova_vm/src/ecmascript` folder that is the closest match to the specification's
header structure for that abstract operation. If a matching file clearly does
not exist, you may create one. When in doubt, ask in Discord or open an issue on
GitHub.

Once you've copied the specification text in as a comment, you can start turning
the commented out abstract operation into a function. Start by changing the
header and comment part of the abstract operation into a doc comment and make
the header a level three header with a link to the original specification.
Preferably, wrap the comment part into multiple lines (generally 80 is used as
the line width point). Finally, remove the repetitive and redundant "It performs
the following steps when called:" part.

As an example, this:

```rs
// 7.1.2 ToBoolean ( argument )
//
// The abstract operation ToBoolean takes argument argument (an ECMAScript language value) and returns a Boolean. It converts argument to a value of type Boolean. It performs the following steps when called:
//
// 1. If argument is a Boolean, return argument.
// 2. If argument is one of undefined, null, +0𝔽, -0𝔽, NaN, 0ℤ, or the empty String, return false.
// 3. NOTE: This step is replaced in section B.3.6.1.
// 4. Return true.
```

turns into

```rs
/// ### [7.1.2 ToBoolean ( argument )](https://tc39.es/ecma262/#sec-toboolean)
///
/// The abstract operation ToBoolean takes argument argument (an ECMAScript
/// language value) and returns a Boolean. It converts argument to a value of
/// type Boolean.
//
// 1. If argument is a Boolean, return argument.
// 2. If argument is one of undefined, null, +0𝔽, -0𝔽, NaN, 0ℤ, or the empty String, return false.
// 3. NOTE: This step is replaced in section B.3.6.1.
// 4. Return true.
```

Now after the doc comment part, add a function of the same name (using Rust's
preferred casing) and move the abstract operation steps inside the function's
body. Consider adding a `todo!()` at the end of the function to avoid Rust
yelling at you a whole lot about your return value. Usually `pub(crate)` is the
preferred publicity level, but go down to private if you can.

The first argument to Nova's functions is usually a `&mut Agent` or `&Agent`
reference. A `&mut Agent` call can mutate the JavaScript heap, which also means
it can call JavaScript. A `&Agent` call cannot mutate the heap and thus cannot
call JavaScript, but can access data from the heap. A call that takes neither
operates entirely on stack-local data. Other arguments should be named as the
specification suggests.

In our example, from an overabundance of caution, we assume we'll need access to
the heap and thus take an `&Agent` reference.

```rs
/// ### [7.1.2 ToBoolean ( argument )](https://tc39.es/ecma262/#sec-toboolean)
///
/// The abstract operation ToBoolean takes argument argument (an ECMAScript
/// language value) and returns a Boolean. It converts argument to a value of
/// type Boolean.
pub(crate) fn to_boolean(agent: &Agent, argument: Value) -> bool {
    // 1. If argument is a Boolean, return argument.
    // 2. If argument is one of undefined, null, +0𝔽, -0𝔽, NaN, 0ℤ, or the empty String, return false.
    // 3. NOTE: This step is replaced in section B.3.6.1.
    // 4. Return true.
    todo!()
}
```

After this, go step by step implementing the abstract operation. Note that the
specification text is still not the bible, and you are allowed to take certain
liberties with it. Specifically, you may do any and all of the following with
some conditions:

1. Add fast-paths for common cases.
2. Move steps around.
3. Skip steps that are performed elsewhere or guaranteed to always do the same
   thing.

The condition for doing any of these is that the change must not be observable
to a user. For example, if the specification states to initially set the values
of a struct to all undefined, and then define them one at a time without calling
any JavaScript (press the `u` key on the ECMAScript specification website to see
where JavaScript may get called) then you may instead define the entire struct
all-at-once and potentially avoid supporting undefined fields (`Option<T>`)
entirely.

If the specification tells you to assert that a particular parameter is of a
specification type `T` and Nova's implementation already guarantees that type
through the function's type signature, you may ignore the assertion entirely.
Consider adding a comment to explain that the type signature guarantees the
assertion.

If the specification tells you to first get the value of an immutable type
(string, boolean, number, symbol), or to prepare a value's data for creation
(such as creating a new String), and then performs checks to see if all
parameters are correct (throwing errors if incorrect), then you may freely
reorder the value getter and/or the data creation to happen after the checks.

If the specification tells you to first check one parameter value (and throws an
error if incorrect), then conditionally performs some JavaScript function call,
and then checks another parameter value and throws an error if incorrect then
you are not allowed to reorder the second check to happen together with the
first, as that would be an observable change.

### Tests in PRs

Nova mainly tests itself using the official
[ECMAScript test262 conformance suite](https://github.com/tc39/test262). The
expected results are saved into the `tests/expectations.json` file: All PR
branches are tested to check that the test results match the expectations.

When making changes, you'll thus often need to update or at least check the
expectations. You can do this with the following command:

```sh
cargo build --profile dev-fast && cargo run --bin test262 --profile dev-fast -- -u
```

This will build you a "dev-fast" version of Nova, and run the test262
conformance using that executable. At the end of the run, it will record the
results `expectations.json`.

You can run an individual test262 test case or a set of tests using

```sh
cargo build && cargo run --bin test262 eval-test internal/path/to/tests
```

Here the "internal/path/to/tests" matches a path or a subpath in the
`expectations.json`. As an example:

```sh
cargo build && cargo run --bin test262 eval-test built-ins/Array/from/from-string.js
```

We also have some unit and integration test around using cargo's test harnesses.
Adding to these is absolutely welcome, as they enable more Miri testing etc.
These are also run on all PRs.

#### Custom quick-shot tests

Keep your own `test.js` in the `nova` folder and run it with

```sh
cargo run eval test.js
```

This is great for quick, simple things you want to test out in isolation.

### Performance considerations

The engine is not at the point where performance is a big consideration. That
being said, we do not want to write slow-by-construction code. Heap data clones
should be kept to a minimum where reasonable, and fast-paths for the most common
cases are highly recommended in abstract operations.

We have a very minimal performance benchmark in the `benches` directory. It can
be run with `cargo bench` or `cargo criterion` (the latter requires
[`cargo-criterion`] to be installed). Only execution time is currently measured.
Memory usage and parsing time are not. For now, take the benchmarks with grain
of salt.

## What are all the `bind(gc.nogc())`, `unbind()`, `scope(agent, gc.nogc())` and other those calls?

Those are part of the garbage collector. See the
[contributor's guide to the garbage collector](https://github.com/trynova/nova/blob/main/GARBAGE_COLLECTOR.md) for
details.

## List of active development ideas

Here are some good ideas on what you can contribute to.

### Internal methods of exotic objects

ECMAScript spec has a ton of exotic objects. Most of these just have some extra
internal slots while others change how they interact with user actions like
get-by-identifier or get-by-value etc.

You can easily find exotic objects' internal methods by searching for
`"fn internal_get_prototype_of("` in the code base. Many of these matches will
be in files that contain a lot of `todo!()` points. As an example,
[proxy.rs](./nova_vm/src/ecmascript/builtins/proxy.rs) is currently entirely
unimplemented. The internal methods of Proxies can be found
[here](https://tc39.es/ecma262/#sec-proxy-object-internal-methods-and-internal-slots):
These abstract internal methods would need to be translated into Nova Rust code
in the `proxy.rs` file.

[This PR](https://github.com/trynova/nova/pull/174) can perhaps also serve as a
good guide into how internal methods are implemented: Especially check the first
and third commits. One important thing for internal method implementations is
that whenever a special implementation exists in the spec, our internal method
should link to it. Another thing is that if you cannot figure out what you
should be calling in the method or the method you should be calling doesn't
exist yet and you think implementing it would be too much work, it is perfectly
fine to simply add a `todo!()` call to punt on the issue.

### Builtin functions

Even more than internal methods, the ECMAScript spec defines builtin functions.
The Nova engine already includes bindings for nearly all of them (only some
Annex B functions should be missing) but the bindings are mostly just `todo!()`
calls.

Implementing missing builtin functions, or at least the easy and commonly used
parts of them, is a massive and important effort. You can find a mostly
exhaustive list of these (by constructor or prototype, or combined)
[in the GitHub issue tracker](https://github.com/trynova/nova/issues?q=is%3Aopen+is%3Aissue+label%3A%22builtin+function%22).

### Heap evolution

The heap needs much more work before it can be considered complete. Technical
work items like the heap evolution works can be found from the GitHub issues
with the
[`technical` label](https://github.com/trynova/nova/issues?q=is%3Aopen+is%3Aissue+label%3Atechnical+).

#### Parallel, single-mutator-multiple-reader vectors

The heap will need to be concurrently marked at some point. Additionally, we'll
want to split some heap data structures into two or more parts; only the
commonly used parts should be loaded into L1 cache during common engine
operations.

For this purpose we'll need our own `Vec`, `Vec2`, `Vec3` and possibly other
vector types. The first order of business is to get the length and capacity to
be stored as a `u32`. The second will be enabling the splitting of heap data
structures; this sbould work in a way similar to `ParallelVec` so that the size
of `Vec2` and `Vec3` stays equal to `Vec`.

Then finally, at some point we'll also want to make the whole heap thread-safe.
Heap vectors (`Vec`, `Vec2`, ...) will become RCU-based, so when they expand (on
push) they will return a `None` or `Some(droppable_vec)` which can either be
dropped immediately (if concurrent heap marking is not currently ongoing) or
pushed into a "graveyard" `UnsafeCell<Vec<(*mut (), fn(*mut ()))>>` that gets
dropped at the end of a mark-and-sweep iteration.

#### Interleaved garbage collection

Currently Nova's garbage collection can only happen when no JavaScript is
running. This means that for instance a dumb live loop like this will eventually
exhaust all memory:

```ts
while (true) {
  ({});
}
```

We want to interleave garbage collection together with running JavaScript, but
this is not a trivial piece of work. Right now a function in the engine might
look like this:

```rs
fn call<'gc>(agent: &mut Agent, obj: Value, mut gc: GcScope<'gc, '_>) -> JsResult<'gc, Value<'gc>> {
    if !obj.is_object() {
        return Err(agent.throw_error(agent, "Not object", gc));
    }
    let is_extensible = is_extensible(agent, obj, gc)?;
    if is_extensible {
        set(agent, obj, "foo".into(), "bar".into(), gc)?;
    }
    get(agent, obj, "length", gc)
}
```

If `obj` is a Proxy, then it the all three of the internal calls
(`is_extensible`, `set`, and `get`) can call user code. Even for non-Proxies,
the `set` and `get` methods may call user code through getters and setters. Now,
if that user code performs a lot of allocation then we'll eventually need to
perform garbage collection. The question is then "are we sure that `obj` is
still valid to use"?

We obviously must make sure that somehow we can keep using `obj`, otherwise
interleaved garbage collection cannot be done. There are two ways to do this:

1. We make all `Value`s safe to keep on stack. In our case, this means that
   `Value` must point to an in-heap reference that points to the `Value`'s
   actual heap data (the things that contains properties etc.). The in-heap
   reference is dropped when the scope is dropped, so the `Value` is safe to
   keep within call scope.
2. Alternatively, `Value` cannot be used after a potential garbage collection
   point. A separate type is added that can be used to move the `Value` onto the
   heap and point to it. That type is safe to keep within call scope.

Some additional thoughts on the two approaches is found below.

##### Make `Value` safe to keep on stack

Any problem can always be fixed by adding an extra level of indirection. In this
case the problem of "where did you put that Value, is it still needed, and can I
mutate it during garbage collection?" can be solved by adding a level of
indirection. In V8 this would be the `HandleScope`. The garbage collector would
be given access to the `HandleScope`'s memory so that it can trace items "on the
stack" and fix them to point to the proper items after garbage collection.

This would be the easiest solution, as this could optionally even be made to
work in terms of actual pointers to `Value`s. The big downside is that this is
an extra indirection which is often honestly unnecessary.

If the `Value` is not pointer based, then another downside is that we cannot
drop them automatically once they're no longer needed using `impl Drop` because
we'd need access to the `HandleScope` inside the `Drop`. Something called linear
types could fix this issue.

##### `Value` lifetime bound to garbage collection safepoints

Any problem can always be fixed by adding an extra lifetime. In this case the
problem of "you're not allowed to keep that Value on stack, I would need to
mutate it during garbage collection" can be solved by using a lifetime to make
sure that Values are never on the stack when garbage collection might happen.
This isn't too hard, really, it just means calls change to be:

```rs
fn call(agent: &'a mut Agent, value: Value<'a>) -> Value<'a> {
    // ...
}
```

This works perfectly well, except for the fact that it cannot be called. Why?
Because the `Value<'a>` borrows the exclusively owned `&'a mut Agent` lifetime;
this is called a reborrow and it's fine within a function but it cannot be done
intra-procedurally. What we could do is this:

```rs
fn call(agent: &mut Agent,  value: Value, mut gc: Gc) -> Value {
    // SAFETY: We've not called any methods that take `&mut Agent` before this.
    // `Value` is thus still a valid reference.
    let value = unsafe { value.bind(agent) };
    // ...
    result.into_register()
}
```

Now we can at least call the function, and lifetimes would protect us from
keeping `Value<'a>` on the stack unsafely. They would _not_ help us with making
sure that `Register<Value<'a>>` is used properly and even if it did, the whole
`Register<Value<'a>>` system is fairly painful to use as each function call
would need to start with this `unsafe {}` song and dance.

But what about when we call some mutable function and need to keep a reference
to a stack value past that call? This is how that would look:

```rs
fn call<'gc>(agent: &mut Agent, value: Value, mut gc: GcScope<'gc, '_>) -> JsResult<'gc, Value<'gc>> {
    let value = unsafe { value.bind(agent) };
    let kept_value: Global<Value> = value.make_global(value);
    other_call(agent, gc.reborrow(), value.into_register())?;
    let value = kept_value.take(agent);
    // ...
}
```

We'd need to make the Value temporarily a Global (which introduces an extra
level of indirection), and then "unwrap" that Global after the call. Globals do
currently exist in Nova, but they are "leaky" in that dropping them on the stack
does not clear their memory on the heap, and is effectively a heap memory leak.
In this case we can see that if `other_call` returns early with an error, then
we accidentally leak `kept_value`'s data. This is again not good.

So we'd need a `Local<'a, Value<'_>>` type of indirection in this case as well.
Whether or not the whole `Value` system makes any sense with that added in is
then very much up for debate.

### Other things

This list serves as a "this is where you were" for returning developers as well
as a potential easy jumping-into point for newcomers.

- Write implementations of more abstract operations
  - See `nova_vm/src/ecmascript/abstract_operations`
  - Specifically eg. `operations_on_objects.rs` is missing multiple operations,
    even stubs.

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
- Add a `Local<'a, Value>` enum that acts as our GC-safe, indirected Value
  storage. See above for more discussion on this under "Heap evolution".
  - A plain `Value` won't be safe to keep on the stack when calling into the
    engine if the engine is free to perform garbage collection at (effectively)
    any (safe)point within the engine code. The `Value`'s internal number might
    need to be readjusted due to GC, which then breaks the `Value`'s identity in
    a sense.
  - A `Local<'a, Value>`s would not point directly to the heap but would instead
    point to an intermediate storage (this is also exactly how V8 does it) where
    identities never change. A nice benefit here is that if we make `Local`
    itself an equivalent enum to `Value`, just with a different index type
    inside, then we can have the intermediate storage store only heap value
    references with 5 bytes each:
    `struct Storage<const N: usize> { types: [u8; N]; values: [u32; N]; }`. We
    cannot drop the types array as it is needed for marking and sweeping the
    storage.
- Add `DISCRIMINANT + 0x80` variants that work as thrown values of type
  `DISCRIMINANT`
  - As a result, eg. a thrown String would be just a String with the top bit set
    true. This would stop Result usage which is a darn shame (can be fixed with
    Nightly features). But it would mean that returning a (sort of)
    `Result<Value>` would fit in a register.
- Consider a register based VM instead of going stack based

# Running the test262 suite

1. Clone this repository with submodules:

   `git clone --recurse-submodules git@github.com:trynova/nova.git`

2. Execute the test262 runner:

   `cargo build -p nova_cli && cargo run --bin test262`

   **Important:** The test runner executes the compiled `nova_cli` directly. If
   you have made changes to CLI or VM, ensure that the `nova_cli` target is
   rebuilt before executing the test runner.

[`cargo-criterion`]: https://crates.io/crates/cargo-criterion
