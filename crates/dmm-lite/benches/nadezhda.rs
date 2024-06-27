use criterion::{criterion_group, criterion_main, Criterion};
use dmm_lite::{
    block::multithreaded_parse_map_locations, prefabs::multithreaded_parse_map_prefabs,
};

fn criterion_benchmark(c: &mut Criterion) {
    let nadezhda_dmm = std::fs::read_to_string("./tests/maps/nadezhda.dmm")
        .expect("Failed to load nadezhda-dmm into memory");
    let nadezhda_tgm = std::fs::read_to_string("./tests/maps/nadezhda-tgm.dmm")
        .expect("Failed to load nadezhda-tgm into memory");

    let mut group = c.benchmark_group("nadezhda");

    group.bench_function("dmm prefabs", |b| {
        b.iter_with_large_drop(|| multithreaded_parse_map_prefabs(nadezhda_dmm.as_str()))
    });
    group.bench_function("tgm prefabs", |b| {
        b.iter_with_large_drop(|| multithreaded_parse_map_prefabs(nadezhda_tgm.as_str()))
    });
    group.bench_function("dmm blocks", |b| {
        b.iter_with_large_drop(|| multithreaded_parse_map_locations(nadezhda_dmm.as_str()))
    });
    group.bench_function("tgm blocks", |b| {
        b.iter_with_large_drop(|| multithreaded_parse_map_locations(nadezhda_tgm.as_str()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
