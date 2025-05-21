const a = new Set(
  Object.keys(JSON.parse(readTextFile("./tests/expectations.json"))),
);

print(`Size: ${a.size}`);

const fn = (k) => !a.has(k);
const novaTests0 = JSON.parse(
  readTextFile("/home/aapoalas/nova_tests_0.json"),
).filter(fn);
const novaTests1 = JSON.parse(
  readTextFile("/home/aapoalas/nova_tests_1.json"),
).filter(fn);

print(novaTests0.length);
print(novaTests1.length);
