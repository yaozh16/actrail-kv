//! 本文件管理进程级当前 locale，并提供不可变快照创建入口。

use std::sync::{OnceLock, RwLock};

use anyhow::{bail, Result};

use super::{namespace, Locale, LocaleModule, LocaleSnapshot};

static CURRENT_LOCALE: OnceLock<RwLock<Locale>> = OnceLock::new();

pub struct LocaleFactory;

impl LocaleFactory {
    /// 初始化全局 locale；未初始化时默认使用 zh-CN。
    pub fn init(locale: Locale) -> Result<()> {
        super::registry::validate_catalogs()?;
        Self::set_locale(locale)
    }

    /// 设置后续快照使用的 locale，不改变已创建快照。
    pub fn set_locale(locale: Locale) -> Result<()> {
        let current = CURRENT_LOCALE.get_or_init(|| RwLock::new(Locale::ZhCn));
        let mut guard = current
            .write()
            .map_err(|_| anyhow::anyhow!("locale registry lock is poisoned"))?;
        *guard = locale;
        Ok(())
    }

    pub fn current() -> Result<Locale> {
        let current = CURRENT_LOCALE.get_or_init(|| RwLock::new(Locale::ZhCn));
        current
            .read()
            .map(|guard| *guard)
            .map_err(|_| anyhow::anyhow!("locale registry lock is poisoned"))
    }

    pub fn snapshot() -> Result<LocaleSnapshot> {
        Ok(LocaleSnapshot::new(Self::current()?))
    }

    pub fn create(namespace: &str) -> Result<LocaleModule> {
        Self::snapshot()?.create(namespace)
    }

    pub fn init_from_code(code: &str) -> Result<Locale> {
        let Some(locale) = Locale::parse(code) else {
            let message = Self::create(namespace::CLI_REPORT)?.format(
                "unsupported_locale",
                &[
                    ("locale", code.to_string()),
                    ("supported", Locale::supported_codes().join(", ")),
                ],
            )?;
            bail!("{message}");
        };
        Self::init(locale)?;
        Ok(locale)
    }
}

#[cfg(test)]
mod tests {
    use super::LocaleFactory;
    use crate::locale::{Locale, LOCALE_TEST_LOCK};

    #[test]
    fn locale_change_only_affects_new_snapshots() {
        let _guard = LOCALE_TEST_LOCK.lock().expect("locale test lock");
        LocaleFactory::set_locale(Locale::ZhCn).expect("zh");
        let zh = LocaleFactory::snapshot().expect("zh snapshot");
        let zh_common = LocaleFactory::create("report.common").expect("zh module");
        LocaleFactory::set_locale(Locale::EnUs).expect("en");
        let en = LocaleFactory::snapshot().expect("en snapshot");

        assert_eq!(zh.locale(), Locale::ZhCn);
        assert_eq!(en.locale(), Locale::EnUs);
        assert_eq!(zh.locale(), Locale::ZhCn);
        assert_eq!(
            zh_common.get("document_title").expect("zh title"),
            "LLM KV 缓存结构诊断"
        );
        assert_ne!(
            zh_common.get("document_title").expect("zh title"),
            LocaleFactory::create("report.common")
                .expect("en module")
                .get("document_title")
                .expect("en title")
        );
        LocaleFactory::set_locale(Locale::ZhCn).expect("restore");
    }

    #[test]
    fn unsupported_locale_uses_registered_cli_message() {
        let _guard = LOCALE_TEST_LOCK.lock().expect("locale test lock");
        LocaleFactory::set_locale(Locale::ZhCn).expect("zh");

        let error = LocaleFactory::init_from_code("fr-FR").expect_err("unsupported locale");
        let message = error.to_string();

        assert!(message.contains("不支持 locale fr-FR"));
        assert!(message.contains("zh-CN, en-US"));
    }
}
