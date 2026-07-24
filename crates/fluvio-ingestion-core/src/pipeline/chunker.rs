//! Token-aware text chunker.
//!
//! Splits text into chunks of ~512 tokens with 64-token overlap.
//! Uses whitespace splitting as a fast approximation — accurate enough
//! for BGE-small which uses a similar tokenization scheme.
//!
//! For production-grade token counting, swap `word_count` for a real
//! tokenizer (the `tokenizers` crate is available as a dep).

/// Default chunk size in approximate tokens (words).
pub const DEFAULT_CHUNK_TOKENS: usize = 512;

/// Default overlap between consecutive chunks in approximate tokens.
pub const DEFAULT_OVERLAP_TOKENS: usize = 64;

/// A single text chunk with its position metadata.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text:        String,
    pub chunk_index: usize,
    pub token_count: usize,
}

/// Split `text` into overlapping chunks.
///
/// Uses word boundaries as a fast approximation for token boundaries.
/// BGE-small tokenizes similarly to word splitting for English text.
pub fn chunk_text(text: &str) -> Vec<Chunk> {
    chunk_text_with_config(text, DEFAULT_CHUNK_TOKENS, DEFAULT_OVERLAP_TOKENS)
}

pub fn chunk_text_with_config(
    text:           &str,
    chunk_tokens:   usize,
    overlap_tokens: usize,
) -> Vec<Chunk> {
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return vec![];
    }

    // If text fits in one chunk, return it as-is
    if words.len() <= chunk_tokens {
        return vec![Chunk {
            text:        words.join(" "),
            chunk_index: 0,
            token_count: words.len(),
        }];
    }

    let step = chunk_tokens.saturating_sub(overlap_tokens).max(1);
    let mut chunks  = Vec::new();
    let mut start   = 0usize;
    let mut idx     = 0usize;

    while start < words.len() {
        let end  = (start + chunk_tokens).min(words.len());
        let text = words[start..end].join(" ");
        let token_count = end - start;

        // Skip chunks that are too small to be meaningful (< 10 tokens)
        if token_count >= 10 {
            chunks.push(Chunk { text, chunk_index: idx, token_count });
            idx += 1;
        }

        if end >= words.len() { break; }
        start += step;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        assert!(chunk_text("").is_empty());
        assert!(chunk_text("   ").is_empty());
    }

    #[test]
    fn test_short_text() {
        let chunks = chunk_text("hello world");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "hello world");
    }

    #[test]
    fn test_overlap() {
        // 20-word chunks, 5-word overlap on 50 words
        let words: Vec<String> = (0..50).map(|i| format!("word{i}")).collect();
        let text = words.join(" ");
        let chunks = chunk_text_with_config(&text, 20, 5);

        // Verify overlap: last words of chunk N == first words of chunk N+1
        assert!(chunks.len() > 1);
        let c0_words: Vec<&str> = chunks[0].text.split_whitespace().collect();
        let c1_words: Vec<&str> = chunks[1].text.split_whitespace().collect();
        let overlap_start = &c0_words[c0_words.len() - 5..];
        let overlap_end   = &c1_words[..5];
        assert_eq!(overlap_start, overlap_end);
    }

    #[test]
    fn test_chunk_indices_sequential() {
        let text = (0..200).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ");
        let chunks = chunk_text(&text);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
        }
    }
}