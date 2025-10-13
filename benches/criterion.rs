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

bench_harness!(
    "simple/array.js",
    "simple/binary-trees.js",
    "simple/count.js",
    "simple/fibonacci-slow.js",
    "simple/fibonacci-fast.js",
);

criterion_group!(benches, bench_execution);
criterion_main!(benches);
