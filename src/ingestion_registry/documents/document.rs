use std::collections::HashMap;
use std::fmt::Debug;

pub trait Document: Debug {
    fn get_size(&self) -> usize;
    fn get_status(&self) -> String;
    fn read_chunk(&self, offset: usize, size: usize) -> Vec<u8>;

    fn extracted_text(&self) -> &str;

    fn metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

#[derive(Debug)]
pub struct TextChunk {
    pub text: String,
    pub metadata: HashMap<String, String>,
}

impl Document for TextChunk {
    fn get_size(&self) -> usize {
        self.text.len()
    }

    fn get_status(&self) -> String {
        "Ready".to_string()
    }

    fn read_chunk(&self, offset: usize, size: usize) -> Vec<u8> {
        self.text
            .as_bytes()
            .get(offset..offset + size)
            .unwrap_or_default()
            .to_vec()
    }

    fn extracted_text(&self) -> &str {
        &self.text
    }

    fn metadata(&self) -> HashMap<String, String> {
        self.metadata.clone()
    }
}
