//! Loading and representing captured request corpora without interpreting business values.

mod loader;
mod types;

pub use loader::{CorpusLoadLimits, CorpusLoader};
pub use types::{AnalysisCorpus, CorpusLoadResult, CorpusRecord, CorpusSkip, CorpusSkipReason};
