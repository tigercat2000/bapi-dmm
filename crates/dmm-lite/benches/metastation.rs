use criterion::{criterion_group, criterion_main, Criterion};
use dmm_lite::{
    block::multithreaded_parse_map_locations, parse_map_multithreaded,
    prefabs::multithreaded_parse_map_prefabs,
};
use winnow::Located;

fn criterion_benchmark(c: &mut Criterion) {
    let meta_dmm = std::fs::read_to_string("./tests/maps/MetaStation.dmm")
        .expect("Failed to load MetaStation-dmm into memory");
    let meta_tgm = std::fs::read_to_string("./tests/maps/MetaStation-tgm.dmm")
        .expect("Failed to load MetaStation-tgm into memory");

    let mut group = c.benchmark_group("Metastation");

    group.bench_function("dmm prefabs", |b| {
        b.iter_with_large_drop(|| multithreaded_parse_map_prefabs(Located::new(meta_dmm.as_str())))
    });
    group.bench_function("tgm prefabs", |b| {
        b.iter_with_large_drop(|| multithreaded_parse_map_prefabs(Located::new(meta_tgm.as_str())))
    });
    group.bench_function("dmm blocks", |b| {
        b.iter_with_large_drop(|| {
            multithreaded_parse_map_locations(Located::new(meta_dmm.as_str()))
        })
    });
    group.bench_function("tgm blocks", |b| {
        b.iter_with_large_drop(|| {
            multithreaded_parse_map_locations(Located::new(meta_tgm.as_str()))
        })
    });
    group.bench_function("dmm full", |b| {
        b.iter_with_large_drop(|| parse_map_multithreaded("Meta".to_owned(), meta_dmm.as_str()))
    });
    group.bench_function("tgm full", |b| {
        b.iter_with_large_drop(|| parse_map_multithreaded("Meta".to_owned(), meta_tgm.as_str()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
