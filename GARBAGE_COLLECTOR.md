# Contributor's guide to working with Nova's interleaved garbage collector

> Alternative title: Why is the borrow checker angry at me?

Nova's garbage collector can be run in any function that takes the marker
`GcScope` as a parameter. Nova's handles (JavaScript values) are "unrooted"
handles meaning that when the garbage collector runs, they are invalidated.
Taken together, this means that when writing code in Nova you have to be very,
very careful! ... or would have to be if we didn't make use of the borrow
checker to help us.

Instead of being very, very careful you just have to remember and abide by a few
rules. Follow along!

## What Nova's garbage collector does

Nova' garbage collector is, at its core, a fairly simple thing. It starts from a
set of "roots" (heap allocated values) and
[traces](https://en.wikipedia.org/wiki/Tracing_garbage_collection) them to find
all heap allocated values that are reachable from them. It then removes all
other heap allocated values, and finally compacts the heap to only contain the
values deemed reachable. Compacting means that most `Value`s that remain in the
heap move during garbage collection, and any references that pointed to their
old location must be fixed to point to the correct post-compaction location.
This means that the garbage collector must be able to reach and mutate all
`Value`s that are going to be used after garbage collection.

The "set of roots" in Nova's case is a list of global JavaScript `Value`s, a
list of "scoped" JavaScript `Value`s, a list of `Vm` structs that are currently
waiting for an inner function call to return, and maybe some others. The
important thing for a developer here is the list of "scoped" `Value`s. When our
Nova engine code is working with `Value`s, they are **not** automatically or
magically reachable by the garbage collector. Thus, if interleaved garbage
collection happens, the `Value` will not be traced nor will its pointed-to
location be fixed post-compaction. Effectively, the `Value` will likely point to
either out of bounds data or to data that is different than what it was before
the garbage collection.

To avoid this, we want to add the `Value` to the list of "scoped" `Value`s, and
refer to the index in that list: The garbage collector does not move `Value`s
within the list, so our index will still point to the same conceptual `Value`
after garbage collection and its pointed-to location will be fixed
post-compaction. Adding `Value`s to the list is done using the `Scoped::scope`
trait method.

Fortunately, we can explain the interaction between `Value`s and the garbage
collector to Rust's borrow checker and have it ensure that we call
`Scoped::scope` before the garbage collector is (possibly) run.
**Unfortunately** explaining the interaction isn't entirely trivial and means we
have to jump through quite a few hoops.

## Simple example

Let's take a silly and mostly trivial example: A method that takes a single
JavaScript object, deletes a property from it and returns the object. Here's the
naive way to write the function:

```rs
fn method<'gc>(agent: &mut Agent, obj: Object, gc: GcScope<'gc, '_>) -> JsResult<'gc, Object<'gc>> {
    // WARNING: the next line is erroneous!
    delete(agent, obj, "key".into(), gc)?;
    Ok(obj)
}
```

Because the `delete` method takes a `GcScope`, it might trigger garbage
collection. If that happens, then the `obj` would no longer point to the same
object that we took as a parameter: this is use-after-free. Here, we can use
scoping to solve the problem:

```rs
fn method<'gc>(agent: &mut Agent, obj: Object, gc: GcScope<'gc, '_>) -> JsResult<'gc, Object<'gc>> {
    let scoped_obj = obj.scope(agent, gc.nogc());
    delete(agent, obj, "key".into(), gc.reborrow())?;
    Ok(scoped_obj.get(agent))
}
```

What we've done here is that before the `delete` call we push the `obj` onto the
list of "scoped" `Value`s found inside the heap, and only then do we call
`delete`. After the `delete` call is done, we read the `obj` value back from the
list of "scoped" `Value`s, thus ensuring that we still point to the correct
object heap data.

The issue here is that we have to know to call the `scope` method ourselves, and
without help this will be impossible to keep track of. Above you already see the
`GcScope::nogc` and `GcScope::reborrow` methods: We use these to make Rust's
borrow checker track the GC safety for us.

The `GcScope::nogc` method performs a shared borrow on the current `GcScope` and
returns a `NoGcScope` that is bound to the lifetime of that shared borrow.
Effectively you can think of it as saying "for as long as this `NoGcScope` or a
`Value` derived from it exists, garbage collection cannot be performed". This
method is used for scoping, explicitly binding `Value`s to the `GcScope`'s
lifetime, and calling methods that are guaranteed to not call JavaScript or
perform garbage collection otherwise.

> Note 1: "Scoped" `Value`s do not restrict garbage collection from being
> performed. They have a different type, `Scoped<Value>`, and are thus not
> `Value`s in the sense mentioned in the previous paragraph.

> Note 2: Currently, Nova makes no difference between methods that can call into
> JavaScript and methods that can perform garbage collection. All JavaScript
> execution is required to be capable of performing garbage collection so
> calling into JavaScript always requires the `GcScope`. A method that cannot
> call into JavaScript but may trigger garbage collection is theoretically
> possible and would likewise require a `GcScope` but there would be no way to
> say that it never calls JavaScript.

The `GcScope::reborrow` method performs an exclusive borrow on the current
`GcScope` and returns a new `GcScope` that is bound to the lifetime of that
exclusive borrow. Effectively, it says "for as long as this `GcScope` or a
`Value` derived from it exists, no other `GcScope` or `Value` derived from them
can be used". This method is used when calling into methods that may perform
garbage collection.

With the `GcScope::nogc`, we can explicitly "bind" a `Value` to the `GcScope`
like this:

```rs
fn method(agent: &mut Agent, obj: Object, gc: GcScope) -> JsResult<Object> {
    let obj = obj.bind(gc.nogc());
    let scoped_obj = obj.scope(agent, gc.nogc());
    delete(agent, obj.unbind(), "key".into(), gc.reborrow()).unbind()?;
    Ok(scoped_obj.get(agent))
}
```

If we were to write out all the lifetime changes here a bit more explicitly, it
would look something like this:

```rs
fn method(agent: &'agent mut Agent, obj: Object<'obj>, gc: GcScope<'gc, 'scope>) -> JsResult<'gc, Object<'gc>> {
    let nogc: NoGcScope<'nogc, 'scope> = gc.nogc(); // [1]
    let obj: Object<'nogc> = obj.bind(gc.nogc());
    let scoped_obj: Scoped<'scope, Object<'static>> = obj.scope(agent, gc.nogc()); // [2]
    {
        let obj_unbind: Object<'static> = obj.unbind(); // [3]
        let gc_reborrow: GcScope<'gcrb, 'scope> = gc.reborrow(); // [4]
        let result: JsResult<'gcrb, bool> = delete(agent, obj_unbind, "key".into(), gc_reborrow); // [5]
        let unbound_result: JsResult<'static, bool> = result.unbind();
        unbound_result?;
    }
    let scoped_obj_get: Object<'static> = scoped_obj.get(agent); // [6]
    Ok(scoped_obj_get)
}
```

Taking the steps in order:

- 1: `'gc: 'nogc`, ie. `'nogc` is shorter than and derives/reborrows from `'gc`.

- 2: `Scoped` does not bind to the `'nogc` lifetime of
  `NoGcScope<'nogc, 'scope>` but instead to the `'scope` lifetime. This is
  purposeful and is what enables `Scoped` to be used after the `delete` call
  without angering the borrow checker.

- 3: The `Object` needs to be "unbound" from the `GcScope` when used as a
  parameter for a method that may perform garbage collection. The reason for
  this is that, effectively, performing `gc.reborrow()` or passing `gc` as a
  parameter to a call invalidates all existing `Value`s "bound" to the
  `GcScope`, including the `obj` that we wanted to pass as a parameter.

- 4: `'gc: 'gcrb`, ie. `'gcrb` is shorter than and derives/reborrows from `'gc`.

- 5: The `delete` method returns a `JsResult<bool>` that carries the `'gcrb`
  lifetime. As this is a shorter lifetime than `'gc`, we cannot rethrow the
  `JsResult::Err` variant as-is (it does not live long enough to be returned
  from the `fn method` function) and therefore have to `.unbind()` the result
  before rethrowing.

- 6: The `Value` (or in this case `Object`) returned from a `Scoped::get` is
  unbound from the `GcScope`. This isn't great and we'd prefer to have `get`
  take in a `NoGcScope` and return `Value<'nogc>` instead but I've not yet
  figured out if that's possible (it requires expert level trait magics).
  Because here we return the `Value` we got immediately, this lifetime is not a
  problem but in general we should perform call `value.bind(gc.nogc())`
  immediately after the `get`.

With these steps, the borrow checker will now ensure that `obj` is not used
after the `delete` call, giving us the help we want and desperately need.

## Rules of thumb for methods that take `GcScope`

Here's a helpful set of things to remember about scoping of `Value`s in calls
and the different APIs related to the `GcScope`.

### At the beginning of a function, bind all parameters

Example:

```rs
fn method(agent: &mut Agent, a: Object, b: Value, c: PropertyKey, d: ArgumentsList, gc: GcScope) {
    let nogc = gc.nogc(); // Perfectly okay to avoid repeating `gc.nogc()` in each call.
    let a = a.bind(nogc);
    let b = b.bind(nogc);
    let c = c.bind(nogc);
    let arg0 = arguments.get(0).bind(nogc);
    let arg1 = arguments.get(1).bind(nogc);
    let arg2 = arguments.get(2).bind(nogc);
    // ... continue to actual work ...
}
```

Yes, this is annoying, I understand. You **must** still do it, or bugs will seep
through! You can also bind `d: ArgumentsList` directly as well to reduce the
work a little.

> Note: The `nogc` local value cannot be used after the first `gc.reborrow()`
> call. You'll need to re-do `let nogc = gc.nogc();` or `nogc = gc.nogc()` if
> you want the convenience again.

### Unbind all parameters only at the call-site, never before

Example:

```rs
method(
    agent,
    a.unbind(),
    b.unbind(),
    c.unbind(),
    ArgumentsList(&[arg0.unbind(), arg1.unbind(), arg2.unbind()]),
    gc.reborrow()
);
```

Yes, this is also annoying. If you don't do it, the borrow checker will yell at
you. Exception: If you call a method with a scoped `Value` directly then you
don't need to explicitly unbind the result from as it is already
`Value<'static>`, ie. unbound.

Example:

```rs
method(agent, scoped_a.get(agent), gc.reborrow());
```

### Immediately "rebind" return values from methods that take `GcScope`

Example:

```rs
let result = method(agent, a.unbind(), gc.reborrow())
    .unbind()
    .bind(gc.nogc());
```

The reason to do this is that the `result` as returned from `method` extends the
lifetime of the `gc.reborrow()` exclusive borrow on the `GcScope`. In effect,
the `result` says that as long as it lives, the `gc: GcScope` cannot be used nor
can any other `Value`s exist. The exact reason for why it works like this is
some fairly obscure Rust lifetime trivia having to do with internal mutability.

In our case, a quick `.unbind().bind(gc.nogc())` allows us to drop the exclusive
borrow on the `GcScope` and replace it with, effectively, a shared borrow on the
`GcScope`. This gives us the `GcScope` binding we wanted.

Exception: This does not need to be done if you're simply returning the result
immediately (this does require passing the entire `gc` by-value instead of by
calling `gc.reborrow()`):

```rs
fn call<'a>(agent: &mut Agent, a: Value, gc: GcScope<'a, '_>) -> JsResult<'a, Value<'a>> {
    let a = a.bind(gc.nogc());
    method(agent, a.unbind(), gc) // No need to rebind the result
}
```

### Always immediately bind `Scoped<Value>::get` results

Example:

```rs
let a = scoped_a.get(agent).bind(gc.nogc());
```

This ensures that no odd bugs occur.

Exception: If the result is immediately used without assigning to a variable,
binding can be skipped.

```rs
scoped_a.get(agent).internal_delete(agent, scoped_b.get(agent), gc.reborrow());
```

Here it is perfectly okay to skip the binding for both `scoped_a` and `scoped_b`
as the borrow checker would force you to again unbind both `Value`s immediately.

### When in doubt, scope!

Example:

```rs
let a = a.bind(gc.nogc());
let result = method(agent, a.unbind(), gc.reborrow())
    .unbind()
    .bind(gc.nogc());
a.internal_set_prototype(agent, result.unbind(), gc.reborrow()); // Error! `gc` is immutably borrowed here but mutably borrowed above
```

If you cannot figure out a way around the borrow checker error (it is absolutely
correct in erroring here), then scope the offending `Value`:

```rs
let a = a.bind(gc.nogc());
let scoped_a = a.scope(agent, gc.nogc());
let result = method(agent, a.unbind(), gc.reborrow())
    .unbind().bind(gc.nogc());
scoped_a.get(agent).internal_set_prototype(agent, result.unbind(), gc.reborrow());
```

### NEVER unbind a `Value` into a local variable

**Bad example:**

```rs
let a = a.unbind();
```

This makes the borrow checker consider this `Value` valid to use for the entire
lifetime of the program. This is very likely _not_ correct and will lead to
bugs.

Exception: If you need to temporarily unbind the `Value` from a lifetime to use
the `into_nogc` method, this can be done:

```rs
let a = a.unbind();
// No garbage collection or JS call can happen after this point. We no longer
// need the GcScope.
let gc = gc.into_nogc();
// With this we're back to being bound; temporary unbinding like this is okay.
let a = a.bind(gc);
```

**Bad example:**

"Temporary unbinding" like above must not contain any `GcScope` usage in
between:

```rs
let a = a.unbind();
// GC can trigger here!
method(agent, b.unbind(), gc.reborrow());
let gc = gc.into_nogc();
// We're back to being bound but GC might have triggered while we were unbound!
let a = a.bind(gc);
```

This is absolutely incorrect and will one day lead to a weird bug.

### Do not scope the same value multiple times

**Bad example:**

```rs
let a = a.scope(agent, gc.nogc());
call(agent, gc.reborrow());
let a = a.get(agent).bind(gc.nogc());
no_gc_method(agent, a, gc.nogc());
let a = a.scope(agent, gc.nogc());
other_Call(agent, gc.reborrow());
let a = a.get(agent).bind(gc.nogc());
no_gc_method(agent, a, gc.nogc());
let a = a.scope(agent, gc.nogc());
// etc...
```

A `Scoped<Value>` is valid for the entire call (at least) and are trivially
cloneable (they're currently not `Copy` but there is no real reason they
couldn't be). Creating one from a `Value` is however a non-trivial operation
that always includes allocating new heap space (though this is amortized).

One might think that the above code ends up with the `a: Value` moved onto the
heap once and the two other scopings just deduplicating to that same `Value`,
but in reality no deduplication is done. This code would store the same `Value`
on the heap thrice.

### Only call `gc.reborrow()` at the call site, never before

Example:

```rs
method(agent, gc.reborrow());
```

**Bad example:**

```rs
let gc_reborrow = gc.reborrow();
method(agent, gc_reborrow);
```

This just doesn't really serve any purpose. Don't do it.
