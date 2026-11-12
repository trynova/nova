// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

function assertEquals(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message}: expected ${expected}, got ${actual}`);
  }
}

// A private getter/setter pair shares one Private Name, so static private
// fields that come after it must not be offset by two.
class A {
  static #a = 1;
  get #b() {
    return 2;
  }
  set #b(v) {}
  static getA() {
    return A.#a;
  }
  getB() {
    return this.#b;
  }
}
assertEquals(A.getA(), 1, "static field after accessor pair");
assertEquals(new A().getB(), 2, "instance accessor pair");

class B {
  #f = 1;
  get #b() {
    return 10;
  }
  set #b(v) {
    this.#f = v;
  }
  static #s = 3;
  static #t = 4;
  static sum() {
    return B.#s + B.#t;
  }
  bump() {
    this.#b = 5;
    return this.#f;
  }
}
assertEquals(B.sum(), 7, "two static fields after accessor pair");
assertEquals(new B().bump(), 5, "setter writes instance field");

class C {
  static get #p() {
    return 1;
  }
  static set #p(v) {}
  static #q = 6;
  static q() {
    return C.#q + C.#p;
  }
}
assertEquals(C.q(), 7, "static accessor pair");
