//! Provides typed TOML configuration, field comments, and opt-in file persistence.

mod config;
#[doc(hidden)]
pub mod derive_support;

pub use config_contract_derive::Config;
pub use config::{Config, ConfigError, ConfigState, RootConfig};
pub use toml_edit::{self, Table};
