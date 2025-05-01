use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use device_detection_51deg::{
    evidence::{self, Evidence},
    hash_engine,
};

pub fn criterion_benchmark(c: &mut Criterion) {
    let test_file =
        PathBuf::from_str("device-detection-cxx/device-detection-data/20000 Evidence Records.yml")
            .unwrap();

    let test_data = fs::read_to_string(&test_file).unwrap();

    let mut cases: Vec<HashMap<String, String>> = Vec::default();

    for part in test_data.split("---") {
        if part.is_empty() {
            continue;
        }
        cases.push(serde_yaml::from_str(part).unwrap());
    }
    let cases_evidence: Vec<Evidence> = cases
        .iter()
        .map(|record| {
            let mut evidence = Evidence::default();
            for (key, value) in record {
                let field = key
                    .strip_prefix("header.")
                    .expect("all hints should be headers");
                evidence = evidence.add(evidence::EvidenceKind::HeaderString, field, value)
            }
            evidence
        })
        .collect();

    let hash_engine = hash_engine::HashEngineBuilder::new(
        &PathBuf::from_str("device-detection-cxx/device-detection-data/51Degrees-LiteV4.1.hash")
            .unwrap(),
    )
    .hash_config(hash_engine::HashConfig::HighPerformance)
    .init()
    .inspect_err(|e| {
        dbg!(format!("{}", e));
    })
    .expect("building the engine should work");

    let mut group = c.benchmark_group("client_hint_cases");
    for case in [1usize, 100, 1000, 10_000, 20_000]
        .into_iter()
        .map(|count| {
            cases_evidence
                .iter()
                .take(count)
                .collect::<Vec<&Evidence>>()
        })
    {
        group.throughput(criterion::Throughput::Elements(case.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(case.len()), &case, |b, case| {
            b.iter(|| {
                for evidence in case.iter() {
                    let _result = hash_engine
                        .process(&evidence)
                        .expect("processing evidence to work");
                }
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
