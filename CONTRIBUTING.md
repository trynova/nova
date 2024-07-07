# Contributing

First be forewarned: Contributions are wanted and dearly desired, but @aapoalas
is a pain to deal with. He'll nitpick your PR to the bone if he happens to be in
the mood to do so. He's not doing it to be evil or anything, he's just an idiot.
Please forgive him, if you can.

## Pull Request Code of Conduct

The following ground rules should be followed:

1. Be courteous.
1. Be respectful.
1. Be your best self, and assume the best of everyone.

Then more specific ones. Feel free to also assume Rust Code of Conduct, it's
probably a good one.

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
can be merged as-is and needs fixing. `needs (nonblocking):` means that you
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

Whenever adding a feature, it is always a good idea to add a test. That being
said, our test harness is currently mostly nonexistential. As such, features can
be merged without tests **within reason**. The expectation then is that either
the feature will be tested well by test262 or tests will be added later.

### Performance considerations

The engine is not at the point where performance would really be a big
consideration. That being said, if you can show great performance or conversely
show that performance hasn't gotten worse, then that's great! No need to worry
about it all too much, though.

## List of active development ideas

Here are some good ideas on what you can contribute to.

### Technical points

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

These and other technical work items can be found from the GitHub issues with
the
[`technical` label](https://github.com/trynova/nova/issues?q=is%3Aopen+is%3Aissue+label%3Atechnical+).

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

### Other things

This list serves as a "this is where you were" for returning developers as well
as a potential easy jumping-into point for newcompers.

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
