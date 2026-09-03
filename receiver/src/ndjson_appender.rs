//! 本文件负责在单一临界区内完成 NDJSON 整行写入和刷新。

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::Mutex,
};

use actrail_kv_artifacts::CapturedRequest;
use fs2::FileExt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppendError {
    #[error("failed to serialize captured request: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("captured request output lock is poisoned")]
    LockPoisoned,
    #[error("captured request output is locked by another receiver: {0}")]
    AlreadyLocked(io::Error),
    #[error("captured request output is unavailable after a previous write failure")]
    Failed,
    #[error("failed to append captured request: {write}; rollback also failed: {rollback}")]
    WriteAndRollback {
        write: io::Error,
        rollback: io::Error,
    },
    #[error("failed to append captured request: {0}")]
    Io(#[from] io::Error),
}

pub struct NdjsonAppender {
    state: Mutex<AppendState>,
}

struct AppendState {
    output: Box<dyn AppendFile>,
    failed: bool,
}

trait AppendFile: Write + Send {
    fn len(&self) -> io::Result<u64>;
    fn truncate(&mut self, length: u64) -> io::Result<()>;
}

impl AppendFile for File {
    fn len(&self) -> io::Result<u64> {
        Ok(self.metadata()?.len())
    }

    fn truncate(&mut self, length: u64) -> io::Result<()> {
        self.set_len(length)
    }
}

impl NdjsonAppender {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppendError> {
        let output = open_private_append_file(path.as_ref())?;
        FileExt::try_lock_exclusive(&output).map_err(AppendError::AlreadyLocked)?;
        Ok(Self::from_output(Box::new(output)))
    }

    pub fn append(&self, record: &CapturedRequest) -> Result<(), AppendError> {
        let mut line = serde_json::to_vec(record)?;
        line.push(b'\n');

        let mut state = self.state.lock().map_err(|_| AppendError::LockPoisoned)?;
        if state.failed {
            return Err(AppendError::Failed);
        }
        let original_length = match state.output.len() {
            Ok(length) => length,
            Err(error) => {
                state.failed = true;
                return Err(AppendError::Io(error));
            }
        };
        let result = state
            .output
            .write_all(&line)
            .and_then(|()| state.output.flush());
        if let Err(write) = result {
            let rollback = state.output.truncate(original_length);
            state.failed = true;
            return match rollback {
                Ok(()) => Err(AppendError::Io(write)),
                Err(rollback) => Err(AppendError::WriteAndRollback { write, rollback }),
            };
        }
        Ok(())
    }

    fn from_output(output: Box<dyn AppendFile>) -> Self {
        Self {
            state: Mutex::new(AppendState {
                output,
                failed: false,
            }),
        }
    }
}

#[cfg(unix)]
fn open_private_append_file(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut options = OpenOptions::new();
    options.create(true).append(true).mode(0o600);
    let output = options.open(path)?;
    output.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    Ok(output)
}

#[cfg(not(unix))]
fn open_private_append_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    options.open(path)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{self, Write},
        sync::{Arc, Mutex},
        thread,
    };

    use actrail_kv_artifacts::{CapturedRequest, ComparisonMetadata};
    use serde_json::json;
    use tempfile::tempdir;

    use super::{AppendError, AppendFile, NdjsonAppender};

    #[test]
    fn concurrent_appends_always_produce_complete_parseable_lines() {
        const WRITERS: usize = 24;
        const RECORDS_PER_WRITER: usize = 80;

        let directory = tempdir().expect("create temporary directory");
        let output = directory.path().join("requests.ndjson");
        let appender = Arc::new(NdjsonAppender::open(&output).expect("open appender"));

        let handles: Vec<_> = (0..WRITERS)
            .map(|writer| {
                let appender = Arc::clone(&appender);
                thread::spawn(move || {
                    for record in 0..RECORDS_PER_WRITER {
                        appender
                            .append(&CapturedRequest {
                                captured_at: "2026-09-02T00:00:00Z".to_owned(),
                                session_id: None,
                                source: Some(format!("writer-{writer}")),
                                comparison: comparison(),
                                payload: json!({
                                    "writer": writer,
                                    "record": record,
                                    "content": "包含换行标记而不是物理换行\n尾部"
                                }),
                            })
                            .expect("append complete record");
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("writer thread succeeds");
        }
        drop(appender);

        let contents = fs::read_to_string(output).expect("read output");
        let lines: Vec<_> = contents.lines().collect();
        assert_eq!(lines.len(), WRITERS * RECORDS_PER_WRITER);

        let mut seen = vec![vec![false; RECORDS_PER_WRITER]; WRITERS];
        for line in lines {
            let parsed: CapturedRequest = serde_json::from_str(line).expect("line is intact JSON");
            let writer = parsed.payload["writer"].as_u64().expect("writer") as usize;
            let record = parsed.payload["record"].as_u64().expect("record") as usize;
            seen[writer][record] = true;
        }
        assert!(seen.into_iter().flatten().all(|present| present));
    }

    #[test]
    fn second_appender_cannot_open_the_same_output() {
        let directory = tempdir().expect("create temporary directory");
        let output = directory.path().join("requests.ndjson");
        let _first = NdjsonAppender::open(&output).expect("open first appender");

        assert!(matches!(
            NdjsonAppender::open(&output),
            Err(AppendError::AlreadyLocked(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn creates_and_tightens_existing_output_to_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempdir().expect("create temporary directory");
        let created = directory.path().join("created.ndjson");
        let created_appender = NdjsonAppender::open(&created).expect("create private output");
        assert_eq!(
            fs::metadata(&created)
                .expect("created metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        drop(created_appender);

        let existing = directory.path().join("existing.ndjson");
        fs::write(&existing, b"existing\n").expect("create existing output");
        fs::set_permissions(&existing, fs::Permissions::from_mode(0o666))
            .expect("make existing output permissive");
        let _existing_appender = NdjsonAppender::open(&existing).expect("open existing output");
        assert_eq!(
            fs::metadata(existing)
                .expect("existing metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn partial_write_is_rolled_back_and_permanently_fails_appender() {
        let shared = Arc::new(Mutex::new(FailingFileState {
            bytes: b"existing\n".to_vec(),
            fail_after: 5,
            truncate_fails: false,
        }));
        let appender = NdjsonAppender::from_output(Box::new(FailingFile {
            shared: Arc::clone(&shared),
            written: 0,
        }));
        let record = captured_record();

        assert!(matches!(appender.append(&record), Err(AppendError::Io(_))));
        assert_eq!(shared.lock().expect("state").bytes, b"existing\n");
        assert!(matches!(appender.append(&record), Err(AppendError::Failed)));
    }

    #[test]
    fn rollback_failure_is_reported_and_permanently_fails_appender() {
        let shared = Arc::new(Mutex::new(FailingFileState {
            bytes: Vec::new(),
            fail_after: 3,
            truncate_fails: true,
        }));
        let appender = NdjsonAppender::from_output(Box::new(FailingFile { shared, written: 0 }));
        let record = captured_record();

        assert!(matches!(
            appender.append(&record),
            Err(AppendError::WriteAndRollback { .. })
        ));
        assert!(matches!(appender.append(&record), Err(AppendError::Failed)));
    }

    fn captured_record() -> CapturedRequest {
        CapturedRequest {
            captured_at: "2026-09-02T00:00:00Z".to_owned(),
            session_id: None,
            source: None,
            comparison: comparison(),
            payload: json!({"model": "example"}),
        }
    }

    fn comparison() -> ComparisonMetadata {
        ComparisonMetadata {
            endpoint_key: "llm-primary".to_owned(),
            agent_key: None,
            model_deployment_key: None,
            kv_namespace: None,
        }
    }

    struct FailingFile {
        shared: Arc<Mutex<FailingFileState>>,
        written: usize,
    }

    struct FailingFileState {
        bytes: Vec<u8>,
        fail_after: usize,
        truncate_fails: bool,
    }

    impl Write for FailingFile {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let mut state = self.shared.lock().expect("state");
            if self.written >= state.fail_after {
                return Err(io::Error::new(io::ErrorKind::WriteZero, "injected failure"));
            }
            let accepted = bytes.len().min(state.fail_after - self.written);
            state.bytes.extend_from_slice(&bytes[..accepted]);
            self.written += accepted;
            Ok(accepted)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl AppendFile for FailingFile {
        fn len(&self) -> io::Result<u64> {
            Ok(self.shared.lock().expect("state").bytes.len() as u64)
        }

        fn truncate(&mut self, length: u64) -> io::Result<()> {
            let mut state = self.shared.lock().expect("state");
            if state.truncate_fails {
                return Err(io::Error::other("injected rollback failure"));
            }
            state.bytes.truncate(length as usize);
            Ok(())
        }
    }
}
