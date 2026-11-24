//! 本文件验证分析器的原子文件边界不会覆盖输入或跟随硬链接误写语料。

use std::fs;

use actrail_kv_analyzer::run::{analyze_file, AnalysisOptions};

#[test]
fn refuses_same_input_and_output_without_modifying_corpus() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("requests.ndjson");
    let original = br#"{"payload":{"model":"m","messages":[]}}
"#;
    fs::write(&path, original).expect("fixture");

    assert!(analyze_file(&path, &path, AnalysisOptions::default()).is_err());
    assert_eq!(fs::read(&path).expect("preserved corpus"), original);
}

#[cfg(unix)]
#[test]
fn refuses_output_that_is_a_hard_link_to_input() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("requests.ndjson");
    let output = directory.path().join("alias.json");
    let original = br#"{"payload":{"model":"m","messages":[]}}
"#;
    fs::write(&input, original).expect("fixture");
    fs::hard_link(&input, &output).expect("hard link");

    assert!(analyze_file(&input, &output, AnalysisOptions::default()).is_err());
    assert_eq!(fs::read(&input).expect("preserved corpus"), original);
}

#[test]
fn rejects_a_corpus_with_no_analyzable_requests() {
    let directory = tempfile::tempdir().expect("tempdir");
    let input = directory.path().join("requests.ndjson");
    let output = directory.path().join("analysis.json");
    fs::write(&input, b"not-json\n").expect("fixture");

    assert!(analyze_file(&input, &output, AnalysisOptions::default()).is_err());
    assert!(!output.exists());
}
