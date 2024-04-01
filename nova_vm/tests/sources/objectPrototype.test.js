const nullObject = Object.create(null);

if (!nullObject) {
    throw new Error("'Object.create(null)' produced nullish output");
}
if (Object.getPrototypeOf(nullObject) !== null) {
    throw new Error("'Object.create(null)' did not produce object with null prototype");
}