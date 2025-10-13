use gungraun::{library_benchmark, library_benchmark_group, main};

mod runner;

use runner::ParsedScript;

fn setup(source_str: &str) -> ParsedScript {
    ParsedScript::new(source_str, true, true)
}

macro_rules! bench_harness {
    ($($ID:ident : $name:literal,)*) => {
        $(
            mod $ID {
                pub(super) static CODE: &str = include_str!(concat!("scripts/", $name));
            }
        )*

        #[library_benchmark]
        $(#[bench::$ID($ID::CODE)])*
        fn bench_parse(script: &str) {
            setup(script);
        }

        #[library_benchmark(setup=setup)]
        $(#[bench::$ID($ID::CODE)])*
        fn bench_exec(script: ParsedScript) {
            script.run();
        }
    };
}

// The commented out boa tests are not compatible with strict mode
bench_harness!(
    boa_arith : "boa/arithmetic_operations.js",
    boa_array_access : "boa/array_access.js",
    boa_array_create : "boa/array_create.js",
    boa_array_pop : "boa/array_pop.js",
    boa_bool_obj_access : "boa/boolean_object_access.js",
    // boa_clean : "boa/clean_js.js",
    boa_fib : "boa/fibonacci.js",
    boa_for : "boa/for_loop.js",
    // boa_min : "boa/mini_js.js",
    boa_number_obj_access : "boa/number_object_access.js",
    boa_obj_create : "boa/object_creation.js",
    boa_obj_prop_access_const : "boa/object_prop_access_const.js",
    boa_obj_prop_access_dyn : "boa/object_prop_access_dyn.js",
    boa_regexp : "boa/regexp.js",
    boa_regexp_create : "boa/regexp_creation.js",
    boa_regexp_lit : "boa/regexp_literal.js",
    boa_regexp_lit_create : "boa/regexp_literal_creation.js",
    boa_str_cmp : "boa/string_compare.js",
    boa_str_concat : "boa/string_concat.js",
    boa_str_cp : "boa/string_copy.js",
    boa_str_obj_access : "boa/string_object_access.js",
    boa_symb_create : "boa/symbol_creation.js",
    simple_array : "simple/array.js",
    simple_binary_trees : "simple/binary-trees.js",
    simple_count : "simple/count.js",
    simple_fib_slow : "simple/fibonacci-slow.js",
    simple_fib_fast : "simple/fibonacci-fast.js",
);

library_benchmark_group!(
   name = bench_parse_group;
   benchmarks = bench_parse
);

library_benchmark_group!(
   name = bench_exec_group;
   benchmarks = bench_exec
);

main!(library_benchmark_groups = bench_exec_group);
