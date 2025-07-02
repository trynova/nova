// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Data structure describing the shape of an object.
///
/// ## What is a shape?
///
/// Object shapes describe the "shape", ie. the keys of an object and their
/// order. For shape-finding purposes, they also describe any descendants that
/// the shape may have, eg. the object shape `{ x, y }` is a descendant of the
/// shape `{ x }`, as it is created by adding `y` to the latter.
///
/// ### Why have shapes?
///
/// Shapes are a fundamental and important mechanism of JavaScript engines in
/// general. They are a requirement for a few critically important
/// optimisations without which a JavaScript engine is woefully inadequate as a
/// modern general-purpose programming tool.
///
/// The first optimisation they enable is deduplication of object keys; two
/// objects both containing `{ x, y }` do not need to store a list of keys
/// each, with both lists containing `x` and `y`. Instead, they both refer to
/// an object shape that contains the list of keys. This cuts object memory
/// usage roughly in half or more, as an object only needs to store its
/// property values without the keys.
///
/// The second optimisation they enable is inline caching of property lookups:
/// when JavaScript code performs a property lookup, eg. `obj.x`, it can store
/// the object shape and offset where it found the property in an "inline
/// cache" (the name stems from the cache data often being stored in the
/// bytecode or machine code data directly, "in line"). When the lookup gets
/// repeated, the code can check if the object shape matches and skip the
/// property search entirely if a match is found.
pub struct ObjectShape {}
