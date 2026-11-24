//! 本文件提供不可变 locale 快照、namespace 句柄与命名参数格式化。

use std::collections::BTreeMap;

use anyhow::{bail, Result};

use super::{registry, Locale};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocaleSnapshot {
    locale: Locale,
}

impl LocaleSnapshot {
    pub(crate) const fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub const fn locale(self) -> Locale {
        self.locale
    }

    pub fn create(self, namespace: &str) -> Result<LocaleModule> {
        let catalog = registry::namespace_catalog(self.locale, namespace)?;
        Ok(LocaleModule {
            locale: self.locale,
            namespace: catalog.name,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocaleModule {
    locale: Locale,
    namespace: &'static str,
}

impl LocaleModule {
    pub const fn locale(self) -> Locale {
        self.locale
    }

    pub const fn namespace(self) -> &'static str {
        self.namespace
    }

    pub fn get(self, key: &str) -> Result<&'static str> {
        registry::lookup(self.locale, self.namespace, key)
    }

    pub fn format(self, key: &str, params: &[(&str, String)]) -> Result<String> {
        let mut values = BTreeMap::new();
        for (name, value) in params {
            if values.insert(*name, value.as_str()).is_some() {
                bail!(
                    "locale message {}.{} received duplicate parameter {name}",
                    self.namespace,
                    key
                );
            }
        }

        let template = self.get(key)?;
        let mut output = String::with_capacity(template.len());
        let mut remaining = template;
        while let Some(start) = remaining.find('{') {
            let (prefix, placeholder) = remaining.split_at(start);
            if prefix.contains('}') {
                bail!(
                    "locale message {}.{} contains an unmatched closing brace",
                    self.namespace,
                    key
                );
            }
            output.push_str(prefix);

            let end = placeholder.find('}').ok_or_else(|| {
                anyhow::anyhow!(
                    "locale message {}.{} contains an unclosed placeholder",
                    self.namespace,
                    key
                )
            })?;
            let name = &placeholder[1..end];
            if name.is_empty() || name.contains('{') {
                bail!(
                    "locale message {}.{} contains an invalid placeholder",
                    self.namespace,
                    key
                );
            }
            let value = values.get(name).ok_or_else(|| {
                anyhow::anyhow!(
                    "locale message {}.{} is missing parameter {name}",
                    self.namespace,
                    key
                )
            })?;
            output.push_str(value);
            remaining = &placeholder[end + 1..];
        }
        if remaining.contains('}') {
            bail!(
                "locale message {}.{} contains an unmatched closing brace",
                self.namespace,
                key
            );
        }
        output.push_str(remaining);
        Ok(output)
    }

    pub fn format_str(self, key: &str, params: &[(&str, &str)]) -> Result<String> {
        let owned: Vec<_> = params
            .iter()
            .map(|(name, value)| (*name, (*value).to_owned()))
            .collect();
        self.format(key, &owned)
    }
}

#[cfg(test)]
mod tests {
    use super::LocaleSnapshot;
    use crate::locale::Locale;

    #[test]
    fn namespace_keys_are_scoped_and_missing_keys_fail() {
        let locale = LocaleSnapshot::new(Locale::ZhCn);
        let defects = locale.create("report.defects").expect("defects");
        assert!(defects.get("section_title").is_ok());
        assert!(defects.get("not_registered").is_err());
        assert!(locale.create("report.unknown").is_err());
    }

    #[test]
    fn formatting_treats_parameter_values_as_literal_text() {
        let locale = LocaleSnapshot::new(Locale::ZhCn);
        let common = locale.create("report.common").expect("common locale");

        let rendered = common
            .format(
                "summary",
                &[
                    ("input", r#"{"name":"x"}"#.to_string()),
                    ("analyzed", "1".to_string()),
                    ("skipped", "{skipped}".to_string()),
                    ("defects", "0".to_string()),
                ],
            )
            .expect("format literal braces");

        assert!(rendered.contains(r#"{"name":"x"}"#));
        assert!(rendered.contains("{skipped}"));
    }

    #[test]
    fn formatting_rejects_missing_parameters() {
        let locale = LocaleSnapshot::new(Locale::ZhCn);
        let common = locale.create("report.common").expect("common locale");
        let error = common
            .format("summary", &[("input", "1".to_string())])
            .expect_err("missing parameters must fail");

        assert!(error.to_string().contains("missing parameter analyzed"));
    }
}
