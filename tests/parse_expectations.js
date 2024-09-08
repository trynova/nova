const exepctationResultKeys = Object.keys(
  JSON.parse(readTextFile("./tests/expectations.json")),
);

const baseMap = new Map();
const baseKeys = [];

for (const key of exepctationResultKeys) {
  const lastIndex = key.lastIndexOf("/");
  const baseKey = key.substring(0, lastIndex);
  const entry = baseMap.get(baseKey);
  if (!entry) {
    baseMap.set(baseKey, 1);
    baseKeys.push(baseKey);
  } else {
    baseMap.set(baseKey, entry + 1);
  }
}

for (const key of baseKeys) {
  print(`'${key}': ${baseMap.get(key)}`);
}
