// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

const nullObject = Object.create(null);

if (!nullObject) {
  throw new Error("'Object.create(null)' produced nullish output");
}
if (Object.getPrototypeOf(nullObject) !== null) {
  throw new Error(
    "'Object.create(null)' did not produce object with null prototype",
  );
}
const objectOfNullObject = Object.create(nullObject);
if (!objectOfNullObject) {
  throw new Error("'Object.create(nullObject)' produced nullish output");
}
if (Object.getPrototypeOf(objectOfNullObject) !== nullObject) {
  throw new Error(
    "'Object.create(nullObject)' did not produce object with nullObject prototype",
  );
}
