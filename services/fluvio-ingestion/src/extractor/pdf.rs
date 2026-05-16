//! PDF text extractor.
//! Moved from src/ingestion_registry/documents/pdf/pdf_document.rs

use anyhow::Context;

pub struct PdfExtractor;

impl PdfExtractor {
    /// Extract all text from a PDF given its raw bytes.
    /// Uses pdf-extract for text extraction.
    pub fn extract(bytes: &[u8]) -> anyhow::Result<String> {
        let text = pdf_extract::extract_text_from_mem(bytes)
            .context("pdf text extraction failed")?;

        // Clean up whitespace — PDF extract often emits one token per line
        let cleaned = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        Ok(cleaned)
    }
}