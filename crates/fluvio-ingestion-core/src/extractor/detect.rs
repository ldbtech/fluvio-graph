//! Format detection from magic bytes and file extension.

#[derive(Debug, Clone, PartialEq)]
pub enum FileFormat {
    Pdf,
    Text,
    Markdown,
    Docx,
    Csv,
    Unknown,
}

impl FileFormat {
    /// Detect format from filename extension + magic bytes.
    pub fn detect(filename: &str, bytes: &[u8]) -> Self {
        // Magic bytes check first (more reliable than extension)
        if bytes.starts_with(b"%PDF") {
            return Self::Pdf;
        }
        // DOCX magic bytes (PK zip header)
        if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
            return Self::Docx;
        }

        // Fall back to extension
        let ext = filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "pdf"              => Self::Pdf,
            "txt"              => Self::Text,
            "md" | "markdown"  => Self::Markdown,
            "docx"             => Self::Docx,
            "csv"              => Self::Csv,
            _                  => Self::Unknown,
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Unknown)
    }
}