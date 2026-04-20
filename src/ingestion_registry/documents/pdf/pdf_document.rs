use std::path::Path;

use super::mmap_chunk;

use crate::ingestion_registry::documents::document::Document;

#[derive(Debug)]
pub struct Pdf {
    pub bytes: Vec<u8>,
    text: String,
    file_path: Option<String>,
}

impl Pdf {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_bytes_with_path(bytes, "in-memory.pdf")
    }

    pub fn from_bytes_with_path(bytes: Vec<u8>, file_path: &str) -> Self {
        let text = pdf_extract::extract_text_from_mem(&bytes)
            .ok()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());

        let file_path = Path::new(file_path)
            .is_file()
            .then(|| file_path.to_string());

        Self {
            bytes,
            text,
            file_path,
        }
    }
}

impl Document for Pdf {
    fn get_size(&self) -> usize {
        self.bytes.len()
    }

    fn read_chunk(&self, offset: usize, size: usize) -> Vec<u8> {
        if let Some(ref path) = self.file_path {
            if let Ok(chunk) = mmap_chunk::read_file_chunk(path, offset, size) {
                return chunk;
            }
        }
        let start = offset.min(self.bytes.len());
        let end = start.saturating_add(size).min(self.bytes.len());
        self.bytes[start..end].to_vec()
    }

    fn extracted_text(&self) -> &str {
        &self.text
    }

    fn get_status(&self) -> String {
        "Ready".to_string()
    }
}
