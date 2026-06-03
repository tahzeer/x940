use criterion::{black_box, Criterion};
use std::fs;
use x940rs::{parse_mt940, to_json, DecoderChain};

fn bench_parse_swift(c: &mut Criterion) {
    let raw = fs::read_to_string("tests/data/swift/swift_payload_1.sta").unwrap();
    let chain = DecoderChain::auto();

    c.benchmark_group("parse").bench_function("swift_3txns", |b| {
        b.iter(|| {
            let stmts = parse_mt940(black_box(&raw), &chain).unwrap();
            black_box(stmts);
        });
    });
}

fn bench_parse_gvc(c: &mut Criterion) {
    let raw = fs::read_to_string("tests/data/gvc/gvc_payload_2.sta").unwrap();
    let chain = DecoderChain::auto();

    c.benchmark_group("parse").bench_function("gvc_2txns", |b| {
        b.iter(|| {
            let stmts = parse_mt940(black_box(&raw), &chain).unwrap();
            black_box(stmts);
        });
    });
}

fn bench_decode_swift(c: &mut Criterion) {
    let chain = DecoderChain::auto();
    let input =
        "/EREF/INV-2026-991/REMI/MONTHLY RETAINER FEES/NAME/ALPHA DIGITAL CORP/BIC/ALPHDEFFXXX";

    c.benchmark_group("decode").bench_function("swift", |b| {
        b.iter(|| {
            let result = chain.decode(black_box(input));
            black_box(result);
        });
    });
}

fn bench_decode_gvc(c: &mut Criterion) {
    let chain = DecoderChain::auto();
    let input =
        "166?00REMITTANCE?20INV-9924?21KREATOR ABSCHNITT 1?3010020030?3188776655?32ACME CORP GMBH";

    c.benchmark_group("decode").bench_function("gvc", |b| {
        b.iter(|| {
            let result = chain.decode(black_box(input));
            black_box(result);
        });
    });
}

fn bench_serialize_json(c: &mut Criterion) {
    let raw = fs::read_to_string("tests/data/swift/swift_payload_1.sta").unwrap();
    let chain = DecoderChain::auto();
    let stmts = parse_mt940(&raw, &chain).unwrap();

    c.benchmark_group("serialize").bench_function("json", |b| {
        b.iter(|| {
            let json = to_json(black_box(&stmts)).unwrap();
            black_box(json);
        });
    });
}

criterion::criterion_group!(
    benches,
    bench_parse_swift,
    bench_parse_gvc,
    bench_decode_swift,
    bench_decode_gvc,
    bench_serialize_json,
);
criterion::criterion_main!(benches);
