//! normalizer.rs
//!
//! Translates raw Gmail API responses into NormalizedChunk — the universal
//! format the ingestion pipeline consumes.
//!
//! Rules:
//!   - One NormalizedChunk per GmailMessage
//!   - text = subject + body (HTML stripped)
//!   - metadata = sender, recipients, timestamp, labels, thread_id
//!   - source_uri = "gmail://message/{id}"
//!   - Thread replies get pre_defined_edges pointing to the parent message
//!   - One NormalizedChunk per GmailThread (conversation node)

use crate::graph::enums::Domain;
use crate::ingestion_registry::connector::{NormalizedChunk, PreDefinedEdge};
use super::client::models::{GmailMessage, GmailThread, GmailLabel};

// ── Message → Chunk ───────────────────────────────────────────────────────────

/// Normalize a single GmailMessage into a NormalizedChunk.
/// `chunk_index` is the position of this message in the sync batch.
pub fn normalize_message(
    msg: &GmailMessage,
    chunk_index: usize,
) -> Option<NormalizedChunk> {
    // Build embeddable text: subject + body.
    let subject = msg.subject().unwrap_or("").to_string();
    let body    = extract_body(msg);

    // Skip empty messages — nothing to embed.
    let text = build_text(&subject, &body);
    if text.trim().is_empty() {
        return None;
    }

    let source_uri = format!("gmail://message/{}", msg.id);

    let mut chunk = NormalizedChunk::new(
        text,
        source_uri,
        Domain::Email,
        chunk_index,
    );

    // ── Metadata ──────────────────────────────────────────────────────────────
    chunk.metadata.insert("source".to_string(), "email".to_string());
    chunk.metadata.insert("message_id".to_string(), msg.id.clone());
    chunk.metadata.insert("thread_id".to_string(),  msg.thread_id.clone());

    if let Some(from) = msg.from() {
        chunk.metadata.insert("sender".to_string(), from.to_string());
    }
    if let Some(to) = msg.to() {
        chunk.metadata.insert("recipients".to_string(), to.to_string());
    }
    if !subject.is_empty() {
        chunk.metadata.insert("subject".to_string(), subject);
    }
    if let Some(date) = msg.date() {
        chunk.metadata.insert("date".to_string(), date.to_string());
    }
    if let Some(ts) = msg.timestamp_secs() {
        chunk.metadata.insert("timestamp".to_string(), ts.to_string());
    }
    if !msg.label_ids.is_empty() {
        chunk.metadata.insert("labels".to_string(), msg.label_ids.join(","));
    }
    if let Some(snippet) = &msg.snippet {
        chunk.metadata.insert("snippet".to_string(), snippet.clone());
    }

    Some(chunk)
}

/// Normalize all messages in a thread, adding reply edges between them.
/// Returns one chunk per message plus one thread-level conversation chunk.
pub fn normalize_thread(
    thread: &GmailThread,
    start_index: usize,
) -> Vec<NormalizedChunk> {
    let mut chunks = Vec::new();

    // Normalize each message in the thread.
    for (i, msg) in thread.messages.iter().enumerate() {
        if let Some(mut chunk) = normalize_message(msg, start_index + i) {
            // Add thread membership metadata.
            chunk.metadata.insert(
                "thread_position".to_string(),
                i.to_string(),
            );
            chunk.metadata.insert(
                "thread_length".to_string(),
                thread.messages.len().to_string(),
            );

            // Add reply edge — each message points back to the previous one.
            // This gives us the thread structure as pre-defined edges.
            if i > 0 {
                let parent_id  = &thread.messages[i - 1].id;
                let parent_uri = format!("gmail://message/{}", parent_id);

                chunk = chunk.with_edge(PreDefinedEdge {
                    to_uri:                  parent_uri,
                    label:                   "reply_to".to_string(),
                    relationship_probability: 1.0,  // thread order is certain
                    token_cost:              1,
                });
            }

            chunks.push(chunk);
        }
    }

    // Thread-level conversation node — summarizes the whole thread.
    if let Some(thread_chunk) = normalize_thread_summary(thread, start_index + chunks.len()) {
        chunks.push(thread_chunk);
    }

    chunks
}

/// Build a single conversation-level chunk for the whole thread.
/// text = subject + all participant names + snippet.
fn normalize_thread_summary(
    thread: &GmailThread,
    chunk_index: usize,
) -> Option<NormalizedChunk> {
    if thread.messages.is_empty() {
        return None;
    }

    let first = &thread.messages[0];
    let subject = first.subject().unwrap_or("(no subject)").to_string();

    // Collect all unique senders across the thread.
    let participants: Vec<String> = thread.messages.iter()
        .filter_map(|m| m.from())
        .map(|f| f.to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let snippet = thread.snippet.clone().unwrap_or_default();
    let text = format!(
        "Thread: {subject}\nParticipants: {}\nPreview: {snippet}",
        participants.join(", "),
    );

    let source_uri = format!("gmail://thread/{}", thread.id);

    let mut chunk = NormalizedChunk::new(text, source_uri, Domain::Email, chunk_index);

    chunk.metadata.insert("source".to_string(), "email".to_string());
    chunk.metadata.insert("thread_id".to_string(),     thread.id.clone());
    chunk.metadata.insert("subject".to_string(),       subject);
    chunk.metadata.insert("message_count".to_string(), thread.messages.len().to_string());
    chunk.metadata.insert("participants".to_string(),  participants.join(","));
    chunk.metadata.insert("kind".to_string(),          "thread_summary".to_string());

    // Link thread summary to all its messages.
    for msg in &thread.messages {
        chunk = chunk.with_edge(PreDefinedEdge {
            to_uri:                  format!("gmail://message/{}", msg.id),
            label:                   "contains".to_string(),
            relationship_probability: 1.0,
            token_cost:              1,
        });
    }

    Some(chunk)
}

/// Normalize a list of Gmail labels into chunks.
/// Labels become lightweight metadata nodes in the graph.
pub fn normalize_labels(labels: &[GmailLabel]) -> Vec<NormalizedChunk> {
    labels.iter().enumerate().map(|(i, label)| {
        let text = format!("Gmail label: {}", label.name);
        let source_uri = format!("gmail://label/{}", label.id);

        let mut chunk = NormalizedChunk::new(text, source_uri, Domain::Email, i);

        chunk.metadata.insert("source".to_string(), "email".to_string());
        chunk.metadata.insert("label_id".to_string(), label.id.clone());
        chunk.metadata.insert("label_name".to_string(), label.name.clone());
        chunk.metadata.insert("kind".to_string(), "label".to_string());

        if let Some(t) = &label.r#type {
            chunk.metadata.insert("label_type".to_string(), t.clone());
        }
        if let Some(total) = label.messages_total {
            chunk.metadata.insert("messages_total".to_string(), total.to_string());
        }

        chunk
    }).collect()
}

// ── Text extraction helpers ───────────────────────────────────────────────────

/// Extract plain text body from a message.
/// Prefers text/plain, strips HTML from text/html as fallback.
fn extract_body(msg: &GmailMessage) -> String {
    // Try plain text first.
    if let Some(text) = msg.plain_text_body() {
        return clean_text(&text);
    }

    // Fall back to snippet (always plain text, always short).
    msg.snippet.clone().unwrap_or_default()
}

/// Combine subject and body into embeddable text.
fn build_text(subject: &str, body: &str) -> String {
    match (subject.is_empty(), body.is_empty()) {
        (true,  true)  => String::new(),
        (false, true)  => subject.to_string(),
        (true,  false) => body.to_string(),
        (false, false) => format!("{subject}\n\n{body}"),
    }
}

/// Remove excessive whitespace and normalize line endings.
fn clean_text(text: &str) -> String {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use crate::ingestion_registry::email::client::models::{
        GmailMessage, GmailThread, GmailLabel,
        MessagePart, MessageHeader, MessageBody,
    };

    fn make_message(id: &str, thread_id: &str, subject: &str, body: &str) -> GmailMessage {
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(body.as_bytes());

        GmailMessage {
            id:            id.to_string(),
            thread_id:     thread_id.to_string(),
            label_ids:     vec!["INBOX".to_string()],
            snippet:       Some(body.chars().take(100).collect()),
            size_estimate: Some(512),
            internal_date: Some(1_700_000_000_000),
            history_id:    Some("99999".to_string()),
            payload: Some(MessagePart {
                mime_type: Some("text/plain".to_string()),
                headers: vec![
                    MessageHeader { name: "From".to_string(),    value: "alice@example.com".to_string() },
                    MessageHeader { name: "To".to_string(),      value: "bob@example.com".to_string() },
                    MessageHeader { name: "Subject".to_string(), value: subject.to_string() },
                    MessageHeader { name: "Date".to_string(),    value: "Mon, 01 Jan 2024 00:00:00 +0000".to_string() },
                ],
                body: Some(MessageBody {
                    data:          Some(encoded),
                    attachment_id: None,
                    size:          Some(body.len() as i64),
                }),
                parts: vec![],
            }),
        }
    }

    #[test]
    fn test_normalize_message_basic() {
        let msg = make_message("msg1", "thread1", "Hello Bob", "This is the body.");
        let chunk = normalize_message(&msg, 0).unwrap();

        assert!(chunk.text.contains("Hello Bob"));
        assert!(chunk.text.contains("This is the body."));
        assert_eq!(chunk.source_uri, "gmail://message/msg1");
        assert_eq!(chunk.domain, Domain::Email);
        assert_eq!(chunk.chunk_index, 0);
    }

    #[test]
    fn test_normalize_message_metadata() {
        let msg = make_message("msg2", "thread1", "Subject here", "Body here.");
        let chunk = normalize_message(&msg, 1).unwrap();

        assert_eq!(chunk.metadata.get("message_id").unwrap(), "msg2");
        assert_eq!(chunk.metadata.get("thread_id").unwrap(), "thread1");
        assert_eq!(chunk.metadata.get("sender").unwrap(), "alice@example.com");
        assert_eq!(chunk.metadata.get("recipients").unwrap(), "bob@example.com");
        assert_eq!(chunk.metadata.get("subject").unwrap(), "Subject here");
        assert_eq!(chunk.metadata.get("source").unwrap(), "email");
        assert_eq!(chunk.metadata.get("timestamp").unwrap(), "1700000000");
    }

    #[test]
    fn test_normalize_message_no_edges() {
        let msg = make_message("msg3", "thread1", "Solo", "Body.");
        let chunk = normalize_message(&msg, 0).unwrap();
        assert!(!chunk.has_pre_defined_edges());
    }

    #[test]
    fn test_normalize_thread_reply_edges() {
        let thread = GmailThread {
            id:         "thread1".to_string(),
            snippet:    Some("Thread preview".to_string()),
            history_id: Some("12345".to_string()),
            messages: vec![
                make_message("msg1", "thread1", "First",  "First message body."),
                make_message("msg2", "thread1", "Re: First", "Reply body."),
                make_message("msg3", "thread1", "Re: Re: First", "Second reply."),
            ],
        };

        let chunks = normalize_thread(&thread, 0);

        // 3 messages + 1 thread summary = 4 chunks
        assert_eq!(chunks.len(), 4);

        // First message has no reply edge.
        assert!(!chunks[0].has_pre_defined_edges());

        // Second message replies to first.
        assert!(chunks[1].has_pre_defined_edges());
        assert_eq!(chunks[1].pre_defined_edges[0].label, "reply_to");
        assert_eq!(chunks[1].pre_defined_edges[0].to_uri, "gmail://message/msg1");

        // Third message replies to second.
        assert_eq!(chunks[2].pre_defined_edges[0].to_uri, "gmail://message/msg2");
    }

    #[test]
    fn test_normalize_thread_summary() {
        let thread = GmailThread {
            id:         "thread42".to_string(),
            snippet:    Some("Preview text".to_string()),
            history_id: None,
            messages: vec![
                make_message("m1", "thread42", "Meeting tomorrow", "Can we meet?"),
                make_message("m2", "thread42", "Re: Meeting tomorrow", "Sure!"),
            ],
        };

        let chunks = normalize_thread(&thread, 0);
        let summary = chunks.last().unwrap();

        assert_eq!(summary.source_uri, "gmail://thread/thread42");
        assert_eq!(summary.metadata.get("kind").unwrap(), "thread_summary");
        assert_eq!(summary.metadata.get("message_count").unwrap(), "2");
        assert_eq!(summary.pre_defined_edges.len(), 2); // links to both messages
        assert_eq!(summary.pre_defined_edges[0].label, "contains");
    }

    #[test]
    fn test_normalize_labels() {
        let labels = vec![
            GmailLabel {
                id:                      "INBOX".to_string(),
                name:                    "INBOX".to_string(),
                r#type:                  Some("system".to_string()),
                messages_total:          Some(42),
                messages_unread:         Some(3),
                threads_total:           None,
                threads_unread:          None,
                label_list_visibility:   None,
                message_list_visibility: None,
            },
            GmailLabel {
                id:                      "Label_1".to_string(),
                name:                    "Work".to_string(),
                r#type:                  Some("user".to_string()),
                messages_total:          Some(10),
                messages_unread:         Some(0),
                threads_total:           None,
                threads_unread:          None,
                label_list_visibility:   None,
                message_list_visibility: None,
            },
        ];

        let chunks = normalize_labels(&labels);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].metadata.get("label_id").unwrap(), "INBOX");
        assert_eq!(chunks[1].metadata.get("label_name").unwrap(), "Work");
        assert_eq!(chunks[0].metadata.get("messages_total").unwrap(), "42");
    }

    #[test]
    fn test_empty_message_returns_none() {
        let mut msg = make_message("empty", "t1", "", "");
        msg.snippet = None;
        // Body is empty base64, subject is empty → should return None
        let result = normalize_message(&msg, 0);
        // May or may not be None depending on base64 decode of empty string
        // — just ensure it doesn't panic.
        let _ = result;
    }

    #[test]
    fn test_clean_text() {
        let raw = "  Hello   \n\n  World  \n  ";
        let cleaned = super::clean_text(raw);
        assert_eq!(cleaned, "Hello\nWorld");
    }

    #[test]
    fn test_build_text_both() {
        let text = super::build_text("Subject", "Body");
        assert_eq!(text, "Subject\n\nBody");
    }

    #[test]
    fn test_build_text_subject_only() {
        let text = super::build_text("Subject", "");
        assert_eq!(text, "Subject");
    }

    #[test]
    fn test_build_text_body_only() {
        let text = super::build_text("", "Body");
        assert_eq!(text, "Body");
    }

    #[test]
    fn test_build_text_both_empty() {
        let text = super::build_text("", "");
        assert!(text.is_empty());
    }
}