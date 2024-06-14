# Nova - Your favorite javascript and wasm engine

## :warning: This project is a Work In Progress, and is very far from being suitable for use :warning:

Nova is a [JavaScript](https://tc39.es/ecma262) and
[WebAssembly](https://webassembly.org) engine written in Rust.

The engine is exposed as a library with an API for implementation in Rust
projects which themselves must serve as a runtime for JavaScript code. The
execution model is currently greatly inspired by
[Kiesel](https://codeberg.org/kiesel-js/kiesel) and
[SerenityOS's LibJS](https://github.com/SerenityOS/serenity). See the code for
more details.

The core of our team is on our [Discord server](https://discord.gg/RTrgJzXKUM).

## Talks

### [Nova Engine - Building a DOD JS Engine in Rust @ Finland Rust-lang meetup 1/2024](https://www.youtube.com/watch?v=WKGo1k47eYQ)

Slides:
[Google Drive](https://docs.google.com/presentation/d/1PRinuW2Zbw9c-FGArON3YHiCUP22qIeTpYvDRNbP5vc/edit?usp=drive_link)

Presented at the Finland Rust-lang group's January meetup, 2024. Focus on how
JavaScript engines work in general, and what sort of design choices Nova makes
in this context.

### [Nova JavaScript Engine - Exploring a Data-Oriented Engine Design @ Web Engines Hackfest 2024](https://www.youtube.com/live/r4tPJDj7nm0?si=OFOVaLkfM_gliuyY&t=11946)

Slides:
[Google Drive](https://docs.google.com/presentation/d/1YlHr67ZYCyMp_6uMMvCWOJNOUhleUtxOPlC0Gz8Bg7o/edit?usp=drive_link)

Presented at the Web Engines Hackfest, 2024. Focus on the details of why a
data-oriented engine design is interesting, what sort of benefits it gives and
what sort of costs it has. Explores the engine at a slightly deeper level.

The talk was revisited at the TC39 June meeting, 2024. No video is available,
but the slightly modified slides are.

TC39 slides:
[Google Drive](https://docs.google.com/presentation/d/1Pv6Yn2sUWFIvlLwX9ViCjuyflsVdpEPQBbVlLJnFubM/edit?usp=drive_link)

## [Architecture](./ARCHITECTURE.md)

The architecture of the engine follows the ECMAScript specification in spirit,
but uses data-oriented design for the actual implementation. Records that are
present in the specification are likely present in the Nova engine as well and
they're likely found in an "equivalent" file / folder path as the specification
defines them in.

Where the engine differs from the specification is that most ECMAScript types
and specification Record types are defined "twice": They have one "heap data"
definition, and another "index" definition. The heap data definition generally
corresponds to the specification's definition, in some degree at least. The
index definition is either a wrapper around `u32` or a `NonZeroU32`. Most spec
defined methods are defined on the index definitions (this avoids issues with
borrowing).

The only case when direct "Record type A contains Record type B" ownership is
used is when there can be only one referrer to the Record type B.

### Heap structure - Data-oriented design

Reading the above, you might be wondering why the double-definitions and all
that. The ultimate reason is two-fold:

1. It is an interesting design.
2. It helps the computer make frequently used things fast while allowing the
   infrequently used things to become slow.

Data-oriented design is all the rage on the Internet because of its
cache-friendliness. This engine is one more attempt at seeing what sort of
real-world benefits one might gain with this sort of architecture.

If you find yourself interested in where the idea spawns from and why, take a
look at [the Heap README.md](./nova_vm/src/heap/README.md). It gives a more
thorough walkthrough of the Heap structure and what the idea there is.

## [Contributing](./CONTRIBUTING.md)

So you wish to contribute, eh? You're very welcome to do so! Please take a look
at [the CONTRIBUTING.md](./CONTRIBUTING.md).
