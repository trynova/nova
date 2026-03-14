# Nova JavaScript engine

Nova is a [JavaScript] engine focused on being lightweight, modular, and easy to
embed. The engine's architecture is built close to the ECMAScript specification
in structure with the implementation relying on idiomatic Rust and data-oriented
design over traditional JavaScript engine building strategies. Interpreter
performance is also a goal, but not yet a high priority.

The engine is exposed as a library with an API for implementation in Rust
projects which themselves must serve as a runtime for JavaScript code. The
execution model is greatly inspired by [Kiesel] and [LibJS].

The project's website can be found at [trynova.dev], where we blog about the
project's progress and where we track our Test262 pass rate. The development
discussion is in the progress of moving to [Zulip] but our old [Discord server]
is also still available.

## Lightweight

The engine's heap is set up to keep heap allocations minimal, sacrifing speed of
uncommon structures for a smaller memory footprint in the common cases. The
intention is for modern JavaScript written with strict TypeScript types to run
light and fast, while any TypeScript lines requiring `as any` or `as unknown`
around objects is likely to be much slower and take more memory than expected.

## Easy to embed

The engine has very little bells or whistles and is very easy to set up for
one-off script runs or simple call-and-return instances. The engine uses the
[WTF-8] encoding internally for [`String`] storage, making interfacing with
JavaScript look and act similar to normal Rust code.

```rust
use nova_vm::{ecmascript::{DefaultHostHooks, GcAgent}, engine::GcScope};
let mut agent = GcAgent::new(Default::default(), &DefaultHostHooks);
let realm = agent.create_default_realm();
let _ = agent.run_in_realm(&realm, |_agent, _gc| {
  // do work here
});
agent.gc();
```

## [Architecture]

The engine's public API relies on idiomatic Rust over traditional JavaScript
engine building wisdom. This is most apparent in the [`Value`] type and its
subvariants such as [`Object`]: instead of using NaN-boxing, NuN-boxing, or
other traditional and known efficient strategies for building a dynamically
typed language, Nova uses normal Rust enums carrying either on-stack data or a
32-bit handle to heap-allocated data. The only pointer that gets consistently
passed through call stacks is the [`Agent`] reference, and handles are merely
ways to access heap-allocated JavaScript data held inside the `Agent`.

Internally, the architecture and structure of the engine follows the ECMAScript
specification but uses data-oriented design for the actual implementation. Data
on the heap is allocated in homogenous (containing data of only one type) arenas
with hot data split apart from cold data, and optional data stored behind keyed
indirections using the arena's associated 32-bit handle as the key, thus using
no memory to store the default null case. The arenas are additionally compacted
during garbage collection, trading some extra collection time for better runtime
cache locality for hot data.

## Shortcomings and unexpected edge cases

Nova JavaScript engine is not perfect and has many shortcomings.

1. The engine performance is acceptable, but it is not fast by any means.
1. The [`Array`] implementation does not support sparse storage internally.
   Calling `new Array(10 ** 9)` will request an allocation for 1 billion
   JavaScript [`Value`]s.
1. The [`RegExp`] implementation does not support lookaheads, lookbehinds, or
   backreferences. It is always in UTF-8 / Unicode sets mode, does not support
   RegExp patterns containing unpaired surrogates, and its groups are slightly
   different from what the ECMAScript specification defines. In short: it is not
   compliant.
1. [`Promise`] subclassing is currently not supported.
1. The engine does not support [WebAssembly] execution.

## Talks

### [Out the cave, off the cliff — data-oriented design in Nova JavaScript engine]

Slides:
[Google Drive](https://docs.google.com/presentation/d/1_N5uLxkR0G4HSYtGuI68eXaj51c7FVCngDg7lxiRytM/edit?usp=sharing)

Presented originally at Turku University JavaScript Day, then at Sydney Rust
Meetup, and finally at [JSConf.jp] in slightly differing and evolving forms, the
talk presents the "today" of major JavaScript engines and the "future" of what
Nova is doing, and why it is both a good and a bad idea.

### [Abusing reborrowing for fun, profit, and a safepoint garbage collector @ FOSDEM 2025]

Slides:
[PDF](https://fosdem.org/2025/events/attachments/fosdem-2025-4394-abusing-reborrowing-for-fun-profit-and-a-safepoint-garbage-collector/slides/237982/Abusing_r_4Y4h70i.pdf)

Repository: [GitHub](https://github.com/aapoalas/abusing-reborrowing)

Presented at FOSDEM, 2025. Focuses on the technical challenges and solutions
that lead to Nova's safepoint garbage collector design. The final design mildly
abuses Rust's "reborrowing" functionality to make the borrow checker not only
understand Nova's garbage collector but cooperate with making sure it is used in
the correct way.

### [Nova Engine - Building a DOD JS Engine in Rust @ Finland Rust-lang meetup 1/2024]

Slides:
[Google Drive](https://docs.google.com/presentation/d/1PRinuW2Zbw9c-FGArON3YHiCUP22qIeTpYvDRNbP5vc/edit?usp=drive_link)

Presented at the Finland Rust-lang group's January meetup, 2024. Focus on how
JavaScript engines work in general, and what sort of design choices Nova makes
in this context.

### [Nova JavaScript Engine - Exploring a Data-Oriented Engine Design @ Web Engines Hackfest 2024]

Slides:
[Google Drive](https://docs.google.com/presentation/d/1YlHr67ZYCyMp_6uMMvCWOJNOUhleUtxOPlC0Gz8Bg7o/edit?usp=drive_link)

Presented at the Web Engines Hackfest, 2024. Focus on the details of why a
data-oriented engine design is interesting, what sort of benefits it gives and
what sort of costs it has. Explores the engine at a slightly deeper level.

The talk was revisited at the TC39 June meeting, 2024. No video is available,
but the slightly modified slides are.

TC39 slides:
[Google Drive](https://docs.google.com/presentation/d/1Pv6Yn2sUWFIvlLwX9ViCjuyflsVdpEPQBbVlLJnFubM/edit?usp=drive_link)

## [Contributing]

So you wish to contribute, eh? You're very welcome to do so! Please take a look
at [the CONTRIBUTING.md][Contributing].

[`Agent`]: crate::ecmascript::Agent
[`Array`]: crate::ecmascript::Array
[`RegExp`]: crate::ecmascript::RegExp
[`Promise`]: crate::ecmascript::Promise
[`Object`]: crate::ecmascript::Object
[`String`]: crate::ecmascript::String
[`Value`]: crate::ecmascript::Value
[WebAssembly]: https://webassembly.org
[WTF-8]: https://wtf-8.codeberg.page/
[JavaScript]: https://tc39.es/ecma262
[Kiesel]: https://codeberg.org/kiesel-js/kiesel
[LibJS]: https://github.com/LadybirdBrowser/ladybird/tree/master/Libraries/LibJS
[Architecture]: https://github.com/trynova/nova/blob/main/ARCHITECTURE.md
[Contributing]: https://github.com/trynova/nova/blob/main/CONTRIBUTING.md
[trynova.dev]: https://trynova.dev/
[Out the cave, off the cliff — data-oriented design in Nova JavaScript engine]: https://www.youtube.com/watch?v=QuJRKhySp-0
[Nova JavaScript Engine - Exploring a Data-Oriented Engine Design @ Web Engines Hackfest 2024]: https://www.youtube.com/watch?v=5olgPdqKZ84
[Nova Engine - Building a DOD JS Engine in Rust @ Finland Rust-lang meetup 1/2024]: https://www.youtube.com/watch?v=WKGo1k47eYQ
[Abusing reborrowing for fun, profit, and a safepoint garbage collector @ FOSDEM 2025]: https://fosdem.org/2025/schedule/event/fosdem-2025-4394-abusing-reborrowing-for-fun-profit-and-a-safepoint-garbage-collector/
[Discord server]: https://discord.gg/bwY4TRB8J7
[Zulip]: https://trynova.zulipchat.com/
[JSConf.jp]: https://jsconf.jp/2025/en
