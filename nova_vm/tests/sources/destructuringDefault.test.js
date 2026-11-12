// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

function assertEquals(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${expected}, got ${actual}`);
  }
}

function assertThrowsReferenceError(fn, message) {
  try {
    fn();
  } catch (err) {
    if (err instanceof ReferenceError) {
      return;
    }
    throw new Error(`${message}: expected ReferenceError, got ${err}`);
  }
  throw new Error(`${message}: expected ReferenceError, got no error`);
}

// A default value can read a binding that was destructured before it.
assertEquals(
  (() => {
    const { a, b = a } = { a: 1 };
    return b;
  })(),
  1,
  "const object pattern",
);
assertEquals(
  (() => {
    let [a, b = a] = [2];
    return b;
  })(),
  2,
  "let array pattern",
);
assertEquals(
  (() => {
    var { a, b = a } = { a: 3 };
    return b;
  })(),
  3,
  "var object pattern",
);
assertEquals(
  (({ width, height = width }) => height)({ width: 4 }),
  4,
  "parameter pattern",
);
assertEquals(((a, b = a) => b)(5), 5, "parameter default");
assertEquals(
  (() => {
    const { a, b = a + 1, c = b * 2 } = { a: 1 };
    return c;
  })(),
  4,
  "chained defaults",
);
assertEquals(
  (() => {
    const [[a, b = a] = [7]] = [];
    return b;
  })(),
  7,
  "nested pattern with outer default",
);
assertEquals(
  (() => {
    const { a, c = (() => a)() } = { a: 9 };
    return c;
  })(),
  9,
  "closure in default",
);

// Reading a binding before it is destructured is still a TDZ error.
assertThrowsReferenceError(() => {
  const { b = a, a } = {};
}, "default before binding");
assertThrowsReferenceError(() => {
  const { a = a } = {};
}, "own default");
assertThrowsReferenceError(() => {
  const [a] = [a];
}, "initializer");
assertThrowsReferenceError(() => {
  const [{ a } = { a: a }] = [];
}, "default of enclosing pattern");
assertThrowsReferenceError(() => {
  const { [a]: a } = {};
}, "computed key");
assertThrowsReferenceError(() => {
  ((x = x) => x)();
}, "own parameter default");
assertThrowsReferenceError(() => {
  (({ x } = { x: x }) => x)();
}, "parameter pattern default");
assertThrowsReferenceError(() => {
  const x = x;
}, "self-referencing initializer");
