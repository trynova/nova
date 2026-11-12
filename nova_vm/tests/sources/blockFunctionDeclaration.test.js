// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

function assertEquals(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${expected}, got ${actual}`);
  }
}

// A function declared in a block can see lexical declarations that come
// later in the same block.
{
  function h() {
    return c;
  }
  const c = 2;
  assertEquals(h(), 2, "const after function");
}
{
  function h2() {
    return d + e;
  }
  let d = 3;
  class K {}
  const e = 4;
  assertEquals(h2(), 7, "let and const after function");
}
assertEquals(
  (() => {
    {
      function f() {
        return x;
      }
      let x = 5;
      return f();
    }
  })(),
  5,
  "block in function",
);
switch (1) {
  case 1:
    function sw() {
      return w;
    }
    let w = 8;
    assertEquals(sw(), 8, "case block");
}

// Calling the function before the binding is initialised is a TDZ error.
{
  function early() {
    return y;
  }
  let threw = false;
  try {
    early();
  } catch (err) {
    threw = err instanceof ReferenceError;
  }
  assertEquals(threw, true, "TDZ before initialisation");
  let y = 1;
}
