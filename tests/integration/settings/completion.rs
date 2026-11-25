//! 本文件验证统一配置的并发补全、原值保留及失败保护。

use std::{
    fs,
    sync::{Arc, Barrier},
    thread,
};

use actrail_kv_settings::Settings;
use serde_json::{json, Value};

#[test]
fn concurrent_commands_merge_defaults_without_losing_other_sections() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("new").join("settings.json");
    let ready = Arc::new(Barrier::new(2));
    let workers: Vec<_> = [
        ("receiver", json!({"listen": "127.0.0.1:8080"})),
        ("report", json!({"locale": "zh-CN"})),
    ]
    .into_iter()
    .map(|(name, defaults)| {
        let path = path.clone();
        let ready = ready.clone();
        thread::spawn(move || {
            let mut settings = Settings::load(Some(&path), false).unwrap();
            assert_eq!(settings.section(name, defaults.clone()).unwrap(), defaults);
            ready.wait();
            settings.finish().unwrap();
        })
    })
    .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let completed: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(
        completed,
        json!({"receiver": {"listen": "127.0.0.1:8080"}, "report": {"locale": "zh-CN"}})
    );
}

#[test]
fn completes_only_missing_fields_and_leaves_invalid_settings_intact() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("settings.json");
    fs::write(&path, br#"{"analyzer":{"output":"custom.json","options":{"top_k":12}},"report":{"locale":"en-US"}}"#).unwrap();
    let mut settings = Settings::load(Some(&path), false).unwrap();
    let resolved = settings
        .section(
            "analyzer",
            json!({"output":"analysis.json","options":{"top_k":20,"max_records":100}}),
        )
        .unwrap();
    assert_eq!(
        resolved,
        json!({"output":"custom.json","options":{"top_k":12,"max_records":100}})
    );
    assert_eq!(
        settings.configured_path("analyzer", "output").unwrap(),
        Some(temporary.path().canonicalize().unwrap().join("custom.json"))
    );
    settings.finish().unwrap();
    let completed: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(completed["analyzer"], resolved);
    assert_eq!(completed["report"], json!({"locale": "en-US"}));

    let invalid = b"{invalid";
    fs::write(&path, invalid).unwrap();
    assert!(Settings::load(Some(&path), false).is_err());
    assert_eq!(fs::read(path).unwrap(), invalid);
}
