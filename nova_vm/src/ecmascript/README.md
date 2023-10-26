## ECMAScript

This folder contains the code for things mentioned directly in the
[ECMAScript language specification](https://tc39.es/ecma262/). As much as is
reasonable, the structure within this folder should be similar to the
specification text and code should reuse terms from the specification directly.

### Crossreferencing

#### 6. ECMAScript Data Types and Values

Found in the [`types`](./types/) folder.

#### 7. Abstract operations

Currently mostly found as methods on `Value`.

Maybe move to [`abstract_operations`](./abstract_operation)?

#### 8. Syntax-Directed Operations

This is more about the parsing so I am not sure if this needs to be in the
engine at all.

If this ends up being needed then it will be in a [`syntax`](./syntax/) folder.

#### 9. Executable Code and Execution Contexts

Found in the [`execution`](./execution/) folder.

#### 10. Ordinary and Exotic Objects Behaviours

Currently mostly found in `builtins` but maybe move to
[`behaviours`](./behaviours)?

On the other hand, this part of the spec also contains the subsection 10.3
Built-in Function Objects and various other built-in related things so it might
be okay to keep this in `builtins` in an inline sort of way.

#### 11-15. ECMAScript Language, and 18. Error Handling and Language Extensions

This is all syntax (and then some) and will not be found in the engine.

#### 16. ECMAScript Language: Scripts and Modules

Found in the [`scripts_and_modules`](./scripts_and_modules/) folder.

#### 18. ECMAScript Standard Built-in Objects, and 19-28.

Should be found in the [`builtins`](./builtins/) folder.
