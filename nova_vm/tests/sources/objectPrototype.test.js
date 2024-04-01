const nullObject = Object.create(null);

if (!nullObject) {
    throw "wrong";
}
if (Object.getPrototypeOf(nullObject) !== null) {
    throw "wrong";
}