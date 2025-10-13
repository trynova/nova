// Loosly based on The Computer Language Benchmarks Game binary-trees task, but simplified

class Tree {
  constructor(left, right) {
    this.left = left;
    this.right = right;
  }
}

function item_check(tree) {
  if (tree.left === null) {
    return 1;
  } else {
    return 1 + item_check(tree.left) + item_check(tree.right);
  }
}

function bottom_up_tree(depth) {
  if (depth == 0) {
    return new Tree(null, null);
  } else {
    depth--;
    return new Tree(bottom_up_tree(depth), bottom_up_tree(depth));
  }
}

let tree0 = bottom_up_tree(8);

for (let i = 0; i < 50; i++) {
  let tree = bottom_up_tree(5);
}

item_check(tree0);
