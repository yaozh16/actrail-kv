//! Unit alignment and byte-safe stable-span/slot template extraction.

mod align;
mod extractor;
mod types;

pub use align::{AlignmentError, SequenceAligner, UnitAlignment};
pub use extractor::{TemplateExtractor, TemplateOptions};
pub use types::{
    RequestTemplate, StableSpan, TemplateExtractionResult, TemplateSkip, TemplateSlot,
};
