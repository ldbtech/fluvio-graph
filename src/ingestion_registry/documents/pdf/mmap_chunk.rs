#![allow(dead_code)]
use std::fs::File;
use std::error::Error;
use memmap2::MmapOptions;
use pdf_extract::Document;

/// Byte-range read of a file via mmap (same mechanism as `PDFChunkIterator`).
/// Used by `Pdf::read_chunk` when the source lives on disk.
pub fn read_file_chunk(path: &str, offset: usize, size: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    let start = offset.min(mmap.len());
    let end = start.saturating_add(size).min(mmap.len());
    Ok(mmap[start..end].to_vec())
}

/// Reads a PDF in page chunks using memory-mapped I/O (`memmap2`).
pub struct PDFChunkIterator {
    doc: Document,
    pages: Vec<u32>,
    pages_per_chunk: usize,
    index: usize,
}

impl PDFChunkIterator {
    pub fn new(path: &str, pages_per_chunk: usize) -> Result<Self, Box<dyn Error>> {
        if pages_per_chunk == 0 {
            return Err("pages_per_chunk must be at least 1".into());
        }
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let doc = Document::load_mem(&mmap)?;

        let pages: Vec<u32> = doc.get_pages().keys().cloned().collect();

        Ok(Self {
            doc,
            pages,
            pages_per_chunk,
            index: 0,
        })
    }
}

impl Iterator for PDFChunkIterator {
    type Item = Result<String, Box<dyn Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.pages.len() {
            return None;
        }

        let end = (self.index + self.pages_per_chunk).min(self.pages.len());
        let slice = &self.pages[self.index..end];

        self.index = end;

        let raw = match self.doc.extract_text(slice) {
            Ok(r) => r,
            Err(e) => return Some(Err(Box::new(e))),
        };

        let cleaned = clean_text(&raw);

        Some(Ok(cleaned))
    }
}

fn clean_text(text: &str) -> String {
    // PDF extract often emits one token per line; joining line-by-line preserves that junk.
    // Collapse all whitespace runs into single spaces for readable prose.
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
