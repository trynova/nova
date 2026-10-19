// Regression test for https://github.com/trynova/nova/issues/948

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
// Nested classes must not displace the deferred field-initializer slot.
class P extends A {
  p = 'p-field'
  constructor() {
    class InnerWithDefault {
      inner = 7
    }
    if (new InnerWithDefault().inner !== 7) {
      throw new Error('expected inner === 7')
    }
    super()
  }
}
if (new P().p !== 'p-field') {
  throw new Error('expected p === "p-field"')
}

// Variant 2: nested derived class with its own user constructor and
// fields (its deferred initializer lives at slot 0 of its own body
// executable, one level down).
class Q extends A {
  q = 'q-field'
  constructor() {
    const Nested = class extends A {
      nested = 'nested-field'
      constructor() { super() }
    }
    const inner = new Nested()
    if (inner.nested !== 'nested-field') {
      throw new Error('expected nested === "nested-field"')
    }
    super()
    if (this.q !== 'q-field') {
      throw new Error('expected this.q === "q-field" inside constructor')
    }
  }
}
const q = new Q()
if (q.q !== 'q-field') {
  throw new Error('expected q.q === "q-field"')
}
