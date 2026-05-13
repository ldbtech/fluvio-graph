//! PDF ingestion: memory-mapped file reads, page-chunk iterator, and optional text cleanup.

pub mod cleaner;
pub mod routes;

mod mmap_chunk;
mod pdf_document;

// Public surface for callers outside this crate (binary/tests import selectively).
#[allow(unused_imports)]
pub use mmap_chunk::{read_file_chunk, PDFChunkIterator};
#[allow(unused_imports)]
pub use pdf_document::Pdf;
