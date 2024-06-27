use criterion::{criterion_group, criterion_main, Criterion};
use dmm_lite::prefabs::multithreaded_parse_map_prefabs;

fn criterion_benchmark(c: &mut Criterion) {
    let meta_dmm = std::fs::read_to_string("./tests/maps/nadezhda.dmm")
        .expect("Failed to load nadezhda-dmm into memory");
    let meta_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm")
        .expect("Failed to load nadezhda-tgm into memory");

    let mut group = c.benchmark_group("nadezhda");

    group.bench_function("dmm prefabs", |b| {
        b.iter_with_large_drop(|| multithreaded_parse_map_prefabs(meta_dmm.as_str()))
    });
    group.bench_function("tgm prefabs", |b| {
        b.iter_with_large_drop(|| multithreaded_parse_map_prefabs(meta_tgm.as_str()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
