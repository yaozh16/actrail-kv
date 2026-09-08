//! Loading and representing captured request corpora without interpreting business values.

mod loader;
mod types;

pub use loader::{CorpusLoadLimits, CorpusLoader, CorpusReadError};
pub use types::{
    AnalysisCorpus, CaptureComparison, CorpusLoadResult, CorpusRecord, CorpusSkip, CorpusSkipReason,
};
