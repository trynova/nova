// Regression test for https://github.com/trynova/nova/issues/948
// "class field initializers are broken in subclasses"
//
// A user-written derived class constructor used to throw
// `ReferenceError: Uninitialized this binding` because the instance field
// initializer prelude was emitted at the start of the constructor body,
// before `super()` had bound `this`. The fix defers the field initializer
// to a separate executable that runs after `super()` returns.

class A {}
class B extends A {
  b = 2
  constructor() { super() }
}

const b = new B()
if (b.b !== 2) {
  throw new Error('expected b.b === 2, got ' + b.b)
}

// Field visible to the constructor body after super() returns.
class C extends A {
  c = 3
  constructor() {
    super()
    if (this.c !== 3) {
      throw new Error('expected this.c === 3 inside constructor')
    }
  }
}
new C()

// Multiple instance fields.
class E extends A {
  e1 = 1
  e2 = 2
  constructor() { super() }
}
const e = new E()
if (e.e1 !== 1 || e.e2 !== 2) {
  throw new Error('expected e.e1 === 1 and e.e2 === 2')
}

// Grand-child still inherits fields from both levels.
class I extends B {
  i = 'i-field'
  constructor() { super() }
}
const i = new I()
if (i.b !== 2 || i.i !== 'i-field') {
  throw new Error('expected i.b === 2 and i.i === "i-field"')
}

// Base class with fields must remain unchanged (no regression).
class G {
  g = 42
  constructor() {}
}
if (new G().g !== 42) {
  throw new Error('expected new G().g === 42')
}