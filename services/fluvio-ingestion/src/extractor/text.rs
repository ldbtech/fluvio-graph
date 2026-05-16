//! Plain text and markdown extractor.

pub struct TextExtractor;

impl TextExtractor {
    /// Extract text from plain text or markdown bytes.
    pub fn extract(bytes: &[u8]) -> anyhow::Result<String> {
        let text = String::from_utf8_lossy(bytes).to_string();
        Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    pub fn extract_str(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}