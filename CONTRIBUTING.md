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

## Developer documentation

See [ARCHITECTURE.md](./ARCHITECTURE.md),
[GARBAGE_COLLECTOR.md](./GARBAGE_COLLECTOR.md),
[ecmascript/README.md](./nova_vm/src/ecmascript/README.md),
[engine/README.md](./nova_vm/src/engine/README.md), and
[heap/README.md](./nova_vm/src/heap/README.md) for various details.

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

Scoping is also recommended and corresponds to the main areas of the engine.
These are:

1. `cli`: the testing CLI, located in `nova_cli`.
1. `lint`: the custom lints, located in `nova_lint`.
1. `ecmascript`: the ECMAScript specification implementation, located in
   `nova_vm/src/ecmascript`.
1. `engine`: the bytecode compiler and interpreter, located in
   `nova_vm/src/engine`.
1. `heap`: the heap structure, located in `nova_vm/src/heap`.
1. `gc`: the garbage collector logic, located in `nova_vm/src/heap` as well.
1. `test262`: the Test262 runner and git submodule, located in `tests`.
1. `docs`: documentation.
1. `deps`: dependencies, listed in `Cargo.toml`.

Some commit/PR title examples are below:

1. `feat(ecmascript)`: This is an added feature to the spec-completeness of the
   engine, eg. a new abstract operation, new object type, ...
1. `fix(heap)`: This fixes something in the heap implementation.
1. `feat(engine)`: This adds to the interpreter.
1. `chore(deps)`: Bump a dependency.
1. `feat(test262)`: This adds a new feature to the Test262 runner.
1. `chore(test262)`: This might update the git submodule or the Test262 results.

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

### Tests in PRs

Nova mainly tests itself using the official
[ECMAScript test262 conformance suite](https://github.com/tc39/test262). The
expected results are saved into the `tests/expectations.json` file: All PR
branches are tested to check that the test results match the expectations.

When making changes, you'll thus often need to update or at least check the
expectations. First make sure you have the git submodule checked out:

```sh
git submodule update --init
```

Then running the tests can be done using the following command:

```sh
cargo build --profile dev-fast && cargo run --bin test262 --profile dev-fast -- -u
```

This will build you a "dev-fast" version of Nova, and run the Test262
conformance suite using that executable. At the end of the run, it will record
the results in `expectations.json` and `metrics.json`.

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
you are **not** allowed to reorder the second check to happen together with the
first, as that would be an observable change.

### Performance considerations

The engine is not at the point where performance is a big consideration. That
being said, we do not want to write slow-by-construction code. Heap data clones
should be kept to a minimum where reasonable, and fast-paths for the most common
cases are highly recommended in abstract operations.

That being said, we do not have performance metrics at present. Therefore, it is
also not a supremely important or reasonable thing to require code to be
supremely optimal since we cannot prove it one way or the other.

## What are all the `bind(gc.nogc())`, `unbind()`, `scope(agent, gc.nogc())` and other those calls?

Those are part of the garbage collector. See the
[contributor's guide to the garbage collector](https://github.com/trynova/nova/blob/main/GARBAGE_COLLECTOR.md)
for details.

## List of active development ideas

Here are some good ideas on what you can contribute to.

### Temporal API

The Temporal API is being worked on by students of Bergen University, but it is
a big effort and more hands are absolutely welcome.

### Single VM architecture

Currently, a new `struct Vm` is created on every function call. That is an
unnecessary amount of wastage, and we would do better by avoiding that. We
should have a single persistent `Vm` held by the `Agent`, with
`struct
ExecutionContext`s holding the data needed to "pop" an execution
context's data from the `Vm`.

### Realm-specific heaps

Currently non-ordinary objects are "Realm-agnostic" in that they do not know
which Realm they belong to and will freely drift to whichever Realm we're
currently executing code in. This is not correct, and the fix for this would be
to either make exotic objects Realm-aware or make heaps Realm-specific. I prefer
the latter option.

This would entail making `Agent` contain multiple `Heap`s, and introducing a new
`CrossRealmProxy` object type whose purpose is to "transfer" objects between
Realm heaps. An object `{}` created in Realm A would appear in Realm B's heap as
a `CrossRealmProxy(_)` whose data indicates that the real object is found in
Realm A, and gives some stable `OutRealmReference` index in the Realm A heap
that contains the actual object reference within it.

So: in Realm A we have

1. Object `{}` which does not have a stable identity due to our garbage
   collector performing compaction.
2. `struct OutRealmReferenceRecord { rc: usize; object: Object }` which has a
   stable identity (no compacting of these), reference count, and references the
   actual object.

and in Realm B we have

1. A `struct CrossRealmProxyRecord { realm: Realm; ref: OutRealmReference }`
   which references the `OutRealmReferenceRecord` in Realm A.
2. `struct CrossRealmProxy` handles that transparently trace to the original
   object using all of the indirection in between.

With this we make cross-realm objects very slow, but we keep in-realm exotic
objects from having to become Realm-aware. The cost of Realm-awareness is
on-average 4 bytes per object.

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

### Other things

This list serves as a "this is where you were" for returning developers as well
as a potential easy jumping-into point for newcomers.

Some more long-term prospects and/or wild ideas:

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
