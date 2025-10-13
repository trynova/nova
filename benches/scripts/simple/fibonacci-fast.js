function fibonacci(n) {
  let f0 = 0;
  let f1 = 1;
  if (n < 0) {
    throw "n too small";
  } else if (n == 0) {
    return 0;
  } else {
    while (n > 1) {
      n--;
      let f2 = f0 + f1;
      f0 = f1;
      f1 = f2;
    }
    return f1;
  }
}

fibonacci(400);
