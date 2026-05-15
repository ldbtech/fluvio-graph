//! Document sources (PDF, plain text chunks, etc.) for the ingestion layer.

pub mod document;
pub mod pdf;

pub use document::{Document, TextChunk};
