const nullObject = Object.create(null);

if (!nullObject) {
    throw new Error("'Object.create(null)' produced nullish output");
}
if (Object.getPrototypeOf(nullObject) !== null) {
    throw new Error("'Object.create(null)' did not produce object with null prototype");
}
const objectOfNullObject = Object.create(nullObject);
if (!objectOfNullObject) {
    throw new Error("'Object.create(nullObject)' produced nullish output");
}
// TODO: Object.create handling is mistaken
if (Object.getPrototypeOf(objectOfNullObject) !== nullObject) {
    throw new Error("'Object.create(nullObject)' did not produce object with nullObject prototype");
}