# Nova - Your favorite javascript and wasm engine

## :warning: This project is a Work In Progress, and is very far from being suitable for use :warning:

Nova is a [JavaScript](https://tc39.es/ecma262) (and eventually
[WebAssembly](https://webassembly.org)) engine written in Rust.

The engine is exposed as a library with an API for implementation in Rust
projects which themselves must serve as a runtime for JavaScript code. The
execution model is currently greatly inspired by
[Kiesel](https://codeberg.org/kiesel-js/kiesel) and
[SerenityOS's LibJS](https://github.com/SerenityOS/serenity). See the code for
more details.

The project's website can be found at [trynova.dev](https://trynova.dev/), where
we blog about the project's progress, and where we track our Test262 pass rate.
The core of our team is on our [Discord server](https://discord.gg/bwY4TRB8J7).

## Talks

### [Out the cave, off the cliff — data-oriented design in Nova JavaScript engine](https://www.youtube.com/watch?v=QuJRKhySp-0)

Slides:
[Google Drive](https://docs.google.com/presentation/d/1_N5uLxkR0G4HSYtGuI68eXaj51c7FVCngDg7lxiRytM/edit?usp=sharing)

Presented originally at Turku University JavaScript Day, then at Sydney Rust
Meetup, and finally at [JSConf.jp](https://jsconf.jp/2025/en) in slightly
differing and evolving forms, the talk presents the "today" of major JavaScript
engines and the "future" of what Nova is doing, and why it is both a good and a
bad idea.

### [Abusing reborrowing for fun, profit, and a safepoint garbage collector @ FOSDEM 2025](https://fosdem.org/2025/schedule/event/fosdem-2025-4394-abusing-reborrowing-for-fun-profit-and-a-safepoint-garbage-collector/)

Slides:
[PDF](https://fosdem.org/2025/events/attachments/fosdem-2025-4394-abusing-reborrowing-for-fun-profit-and-a-safepoint-garbage-collector/slides/237982/Abusing_r_4Y4h70i.pdf)

Repository: [GitHub](https://github.com/aapoalas/abusing-reborrowing)

Presented at FOSDEM, 2025. Focuses on the technical challenges and solutions
that lead to Nova's safepoint garbage collector design. The final design mildly
abuses Rust's "reborrowing" functionality to make the borrow checker not only
understand Nova's garbage collector but cooperate with making sure it is used in
the correct way.

### [Nova Engine - Building a DOD JS Engine in Rust @ Finland Rust-lang meetup 1/2024](https://www.youtube.com/watch?v=WKGo1k47eYQ)

Slides:
[Google Drive](https://docs.google.com/presentation/d/1PRinuW2Zbw9c-FGArON3YHiCUP22qIeTpYvDRNbP5vc/edit?usp=drive_link)

Presented at the Finland Rust-lang group's January meetup, 2024. Focus on how
JavaScript engines work in general, and what sort of design choices Nova makes
in this context.

### [Nova JavaScript Engine - Exploring a Data-Oriented Engine Design @ Web Engines Hackfest 2024](https://www.youtube.com/watch?v=5olgPdqKZ84)

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

The architecture and structure of the engine follows the ECMAScript
specification in spirit, but uses data-oriented design for the actual
implementation. Types that are present in the specification, and are often
called "Something Records", are generally found as a `struct` in Nova in an
"equivalent" file / folder path as the specification defines them in. But
instead of referring to these records by pointer or reference, the engine
usually calls these structs the "SomethingRecord" or "SomethingHeapData", and
defines a separate "handle" type which takes the plain "Something" type name and
only contains a 32-bit unsigned integer. The record struct is stored inside the
engine heap in a vector, and the handle type stores the correct vector index for
the value. Polymorphic index types, such as the main JavaScript Value, are
represented as tagged enums over the index types.

In general, all specification abstract operations are then written to operate on
the index types instead of operating on references to the heap structs
themselves. This avoids issues with re-entrancy, pointer aliasing, and others.

### Heap structure - Data-oriented design

Reading the above, you might be wondering why the split into handle and heap
data structs is done. The ultimate reason is two-fold:

1. It is an interesting design.

1. It helps the computer make frequently used things fast while allowing the
   infrequently used things to take less (or no) memory at the cost of access
   performance.

Data-oriented design is all the rage on the Internet because of its
cache-friendliness. This engine is one more attempt at seeing what sort of
real-world benefits one might gain with this sort of architecture.

If you find yourself interested in where the idea spawns from and why, take a
look at [the Heap README.md](./nova_vm/src/heap/README.md). It gives a more
thorough walkthrough of the Heap structure and what the idea there is.

## [Contributing](./CONTRIBUTING.md)

So you wish to contribute, eh? You're very welcome to do so! Please take a look
at [the CONTRIBUTING.md](./CONTRIBUTING.md).
