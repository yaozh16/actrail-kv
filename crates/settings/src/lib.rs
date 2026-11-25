//! 本 crate 按需读取统一设置，并以原子更新补齐缺失默认值。

use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{self, Path, PathBuf},
};

use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Map, Value};

const SECTIONS: [&str; 5] = ["receiver", "storage", "analyzer", "export", "report"];

pub struct Settings {
    path: PathBuf,
    sections: Map<String, Value>,
    pending: Map<String, Value>,
    legacy: bool,
}

impl Settings {
    pub fn load(explicit: Option<&Path>, allow_legacy: bool) -> Result<Self> {
        let requested = match explicit {
            Some(path) => path::absolute(path)?,
            None => std::env::home_dir()
                .context("could not locate home directory; pass --config <path>")?
                .join(".actrail/actrail-kv/settings.json"),
        };
        let path = match requested.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == ErrorKind::NotFound => requested,
            Err(error) => return Err(error).context("failed to resolve settings path"),
        };
        anyhow::ensure!(
            path.file_name().is_some(),
            "settings path needs a file name"
        );
        let sections = read_settings(&path)?;
        let legacy = explicit.is_some()
            && allow_legacy
            && !sections.is_empty()
            && !sections.keys().any(|key| SECTIONS.contains(&key.as_str()));
        if !legacy {
            validate_sections(&sections)
                .with_context(|| format!("invalid settings {}", path.display()))?;
        }
        Ok(Self {
            path,
            sections,
            pending: Map::new(),
            legacy,
        })
    }

    pub fn section<T: Serialize + DeserializeOwned>(
        &mut self,
        name: &str,
        defaults: T,
    ) -> Result<T> {
        if self.legacy {
            return Ok(defaults);
        }
        let defaults = serde_json::to_value(defaults)?;
        let changed = fill_section(&mut self.sections, name, &defaults);
        let value = serde_json::from_value(self.sections[name].clone())
            .with_context(|| format!("invalid settings.{name} in {}", self.path.display()))?;
        if changed {
            self.pending.insert(name.to_owned(), defaults);
        }
        Ok(value)
    }

    pub fn legacy<T: DeserializeOwned>(&self) -> Result<Option<T>> {
        self.legacy
            .then(|| {
                serde_json::from_value(Value::Object(self.sections.clone()))
                    .with_context(|| format!("invalid legacy config {}", self.path.display()))
            })
            .transpose()
    }

    pub fn configured_path(&self, section: &str, field: &str) -> Result<Option<PathBuf>> {
        let value = self
            .sections
            .get(section)
            .and_then(|value| value.get(field));
        let path = match value {
            Some(value) => serde_json::from_value::<Option<PathBuf>>(value.clone())
                .with_context(|| format!("invalid settings.{section}.{field}"))?,
            None => None,
        };
        Ok(path.map(|path| self.resolve(path)))
    }

    pub fn directory(&self) -> &Path {
        self.path
            .parent()
            .expect("absolute settings file path has a parent")
    }

    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        self.directory().join(path)
    }

    pub fn check_output(&self, path: &Path) -> Result<()> {
        let output = path::absolute(path)?;
        let same_file = match same_file::is_same_file(&self.path, &output) {
            Ok(same) => same,
            Err(error) if error.kind() == ErrorKind::NotFound => false,
            Err(error) => return Err(error).context("failed to check settings and output paths"),
        };
        let canonical_entry = match (output.parent(), output.file_name()) {
            (Some(parent), Some(name)) => {
                parent.canonicalize().ok().map(|parent| parent.join(name))
            }
            _ => None,
        };
        anyhow::ensure!(
            output != self.path && !same_file && canonical_entry.as_ref() != Some(&self.path),
            "output must not overwrite settings file {}",
            self.path.display()
        );
        Ok(())
    }

    pub fn finish(self) -> Result<PathBuf> {
        if self.pending.is_empty() {
            return Ok(self.path);
        }

        let directory = self.directory();
        fs::create_dir_all(directory)
            .with_context(|| format!("failed to create {}", directory.display()))?;
        let mut lock_name = OsString::from(".");
        lock_name.push(self.path.file_name().expect("validated settings path"));
        lock_name.push(".lock");
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options
            .open(directory.join(lock_name))
            .context("failed to open settings lock")?;
        FileExt::lock_exclusive(&lock).context("failed to lock settings")?;

        // Re-read after taking the lock so concurrent commands contribute their own defaults.
        let mut current = read_settings(&self.path)?;
        validate_sections(&current)
            .with_context(|| format!("invalid settings {}", self.path.display()))?;
        let mut changed = false;
        for (name, defaults) in &self.pending {
            changed |= fill_section(&mut current, name, defaults);
        }
        if changed {
            let mut output = tempfile::NamedTempFile::new_in(directory)?;
            serde_json::to_writer_pretty(&mut output, &current)?;
            output.write_all(b"\n")?;
            output.as_file().sync_all()?;
            output
                .persist(&self.path)
                .with_context(|| format!("failed to save settings {}", self.path.display()))?;
        }
        Ok(self.path)
    }
}

fn read_settings(path: &Path) -> Result<Map<String, Value>> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Map::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read settings {}", path.display()))
        }
    };
    serde_json::from_slice(&contents)
        .with_context(|| format!("invalid settings {}", path.display()))
}

fn validate_sections(sections: &Map<String, Value>) -> Result<()> {
    if let Some(key) = sections
        .keys()
        .find(|key| !SECTIONS.contains(&key.as_str()))
    {
        anyhow::bail!("unknown top-level settings field {key:?}");
    }
    Ok(())
}

fn fill_section(sections: &mut Map<String, Value>, name: &str, defaults: &Value) -> bool {
    match sections.get_mut(name) {
        Some(section) => match (section, defaults) {
            (Value::Object(section), Value::Object(defaults)) => {
                let mut changed = false;
                for (key, value) in defaults {
                    changed |= fill_section(section, key, value);
                }
                changed
            }
            _ => false,
        },
        None => {
            sections.insert(name.to_owned(), defaults.clone());
            true
        }
    }
}
