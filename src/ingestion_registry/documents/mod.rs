//! Document sources (PDF, plain text chunks, etc.) for the ingestion layer.

pub mod document;
pub mod pdf;
pub mod rule_linker;

pub use document::{Document, TextChunk};
