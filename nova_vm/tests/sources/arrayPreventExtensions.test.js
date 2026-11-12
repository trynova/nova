// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

function assertEquals(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${expected}, got ${actual}`);
  }
}

function assertThrowsTypeError(fn, message) {
  try {
    fn();
  } catch (err) {
    if (err instanceof TypeError) {
      return;
    }
    throw new Error(`${message}: expected TypeError, got ${err}`);
  }
  throw new Error(`${message}: expected TypeError, got no error`);
}

// Object.preventExtensions leaves "length" writable.
{
  const a = Object.preventExtensions([1, 2]);
  const desc = Object.getOwnPropertyDescriptor(a, "length");
  assertEquals(desc.writable, true, "length stays writable");
  assertEquals(Object.isExtensible(a), false, "array is not extensible");
}

// Removing elements works.
{
  const a = Object.preventExtensions([1, 2]);
  assertEquals(a.pop(), 2, "pop result");
  assertEquals(a.length, 1, "length after pop");
}
{
  const a = Object.preventExtensions([1, 2, 3]);
  assertEquals(a.shift(), 1, "shift result");
  assertEquals(a.join(), "2,3", "array after shift");
}
{
  const a = Object.preventExtensions([1, 2, 3]);
  assertEquals(a.splice(0, 1).join(), "1", "splice result");
  assertEquals(a.join(), "2,3", "array after splice");
}
{
  const a = Object.preventExtensions([]);
  assertEquals(a.pop(), undefined, "pop on empty array");
  assertEquals(a.push(), 0, "push with no items");
  assertEquals(a.unshift(), 0, "unshift with no items");
}
{
  const a = Object.preventExtensions([1, 2, 3]);
  a.length = 1;
  assertEquals(a.join(), "1", "shrink length");
  a.length = 3;
  assertEquals(a.length, 3, "grow length");
  assertEquals(1 in a, false, "growing length adds holes");
}

// Adding elements still fails.
assertThrowsTypeError(() => {
  Object.preventExtensions([1]).push(2);
}, "push");
assertThrowsTypeError(() => {
  Object.preventExtensions([1]).unshift(0);
}, "unshift");
assertThrowsTypeError(() => {
  "use strict";
  const a = Object.preventExtensions([1]);
  a[1] = 2;
}, "assign past the end");
assertThrowsTypeError(() => {
  "use strict";
  const a = Object.preventExtensions([1, , 3]);
  a[1] = 2;
}, "assign to a hole");
assertEquals(
  Reflect.defineProperty(Object.preventExtensions([1]), 5, { value: 1 }),
  false,
  "define past the end",
);
{
  "use strict";
  const a = Object.preventExtensions([1, 2]);
  a[0] = 9;
  assertEquals(a[0], 9, "overwrite an element");
}

// A non-writable length does not stop filling a hole.
{
  "use strict";
  const a = [1, , 3];
  Object.defineProperty(a, "length", { writable: false });
  a[1] = 2;
  assertEquals(a.join(), "1,2,3", "fill a hole with read-only length");
}
