//! 本模块提供可验证、原子输出的离线分析运行入口与配置。

mod options;
mod pipeline;

pub use options::AnalysisOptions;
pub use pipeline::{analyze_file, analyze_reader};
