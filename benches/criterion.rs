use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

mod runner;

macro_rules! bench_harness {
    ($($name:literal,)*) => {
        fn bench_execution(c: &mut Criterion) {
            $(
                {
                    static CODE: &str = include_str!(concat!("scripts/", $name));

                    c.bench_function(concat!($name, " (Execution)"), move |b| {
                        b.iter_batched(
                            || -> runner::ParsedScript { runner::ParsedScript::new(CODE, true) },
                            |script| { script.run(); },
                            BatchSize::PerIteration,
                        )
                    });
                }
            )*
        }
    };
}

// The commented out boa tests are not compatible with strict mode
bench_harness!(
    "boa/arithmetic_operations.js",
    "boa/array_access.js",
    "boa/array_create.js",
    "boa/array_pop.js",
    "boa/boolean_object_access.js",
    //"boa/clean_js.js",
    "boa/fibonacci.js",
    "boa/for_loop.js",
    //"boa/mini_js.js",
    "boa/number_object_access.js",
    "boa/object_creation.js",
    "boa/object_prop_access_const.js",
    "boa/object_prop_access_dyn.js",
    "boa/regexp.js",
    "boa/regexp_creation.js",
    "boa/regexp_literal.js",
    "boa/regexp_literal_creation.js",
    "boa/string_compare.js",
    "boa/string_concat.js",
    "boa/string_copy.js",
    "boa/string_object_access.js",
    "boa/symbol_creation.js",
    "simple/array.js",
    "simple/binary-trees.js",
    "simple/count.js",
    "simple/fibonacci-slow.js",
    "simple/fibonacci-fast.js",
);

criterion_group!(benches, bench_execution);
criterion_main!(benches);
