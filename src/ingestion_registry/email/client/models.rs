//! models.rs
//! 
//! Raw Gmail API response structs 
//! These mirros exactly what the Gmail API returns - no logic, just shapes.
//! 
//! API Reference: https://developers.google.com/gmail/api/reference/rest
//! 
use serde::{Deserialize, Serialize};


/// --- Message ---
/// A single gmail message.
/// Returned by `user.messae.get` with `format=full`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GmailMessage {
    /// The immutable ID of the message.
    pub id: String,
 
    /// The ID of the thread the message belongs to.
    pub thread_id: String,
 
    /// List of label IDs applied to this message.
    #[serde(default)]
    pub label_ids: Vec<String>,
 
    /// A short part of the message text — shown in Gmail's list view.
    pub snippet: Option<String>,
 
    /// Estimated size of the message in bytes.
    pub size_estimate: Option<i64>,
 
    /// The parsed email structure (headers + body + attachments).
    pub payload: Option<MessagePart>,
 
    /// Unix timestamp (ms) when the message was received.
    #[serde(deserialize_with = "deserialize_timestamp", default)]
    pub internal_date: Option<i64>,
 
    /// The history ID after this message was modified.
    pub history_id: Option<String>,
}

/// A part of a message - can be the whole body or MIME Part
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePart {
    /// MIME TYPE of this part (e.g: text/plain, text/html, multipart/alternative, etc.)
    pub mime_type: Option<String>,

    /// Headers of this part (from, to, subject, date, ..etc)
    #[serde(default)]
    pub headers: Vec<MessageHeader>,

    /// Body of the part (text/plain or base64 encoded for attachments)
    pub body: Option<MessageBody>,

    /// List of parts inside this part (for multipart/alternative, multipart/mixed, etc.)
    #[serde(default)]
    pub parts: Vec<MessagePart>,
}

/// A single email header (name + value pair)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageHeader {
    pub name: String,
    pub value: String,
}

/// The body of a message part.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageBody {
    /// The body content as a base64 encoded string.
    /// Decode with `base64::engine::general_purpose::URL_SAFE_NO_PAD`
    pub data: Option<String>,

    /// For attachement - use this ID to fetch the full attachment.
    pub attachment_id: Option<String>,

    /// Size of the body in bytes
    pub size: Option<i64>,
}

// ---- Thread ---------------------------------------------------------------

/// A gmail thread - a group of related messages.
/// Returned by `user.threads.get`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GmailThread {
    pub id: String,
    pub snippet: Option<String>,
    pub history_id: Option<String>,
    pub messages: Vec<GmailMessage>,
}

// ---- Label ---------------------------------------------------------------

/// A Gmail Label (index, sent, custom labels, ..etc.)
/// Returned by `users.labels.list` and `users.labels.get`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GmailLabel {
    pub id: String,
    pub name: String,
    pub message_list_visibility: Option<String>,
    pub label_list_visibility: Option<String>,
    pub r#type: Option<String>, // "system" or "user"

    pub messages_total: Option<i64>,
    pub messages_unread: Option<i64>,
    pub threads_total: Option<i64>,
    pub threads_unread: Option<i64>,
}

// ---- List Responses ---------------------------------------------------------------

/// Response for `user.messages.list`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageListResponse {
    #[serde(default)]
    pub messages: Vec<MessageRef>,
    
    /// Pass this to the next request to get the next page.
    pub next_page_token: Option<String>,

    /// Total number of messages matching the query.
    pub result_size_estimate: Option<i32>,
}

/// Sparse message reference returned by list endpoints.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRef {
    pub id:           String,
    pub thread_id:    String,
}

/// Response for `user.threads.list`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadListResponse {
    #[serde(default)]
    pub threads: Vec<ThreadRef>,
    
    pub next_page_token: Option<String>,
    pub result_size_estimate: Option<i32>,
}

/// Sparse thread reference returned by list endpoints.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRef {
    pub id:            String,
    pub snippet:       Option<String>,
    pub history_id:    Option<String>,
}

/// Response from `users.labels.list`.
#[derive(Debug, Deserialize)]
pub struct LabelListResponse {
    #[serde(default)]
    pub labels: Vec<GmailLabel>,
}

/// Response from `user.history.list` - Used for incremental sync.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryListResponse {
    #[serde(default)]
    pub history:              Vec<HistoryItem>,

    pub next_page_token:      Option<String>,
    pub history_id: Option<String>,
}

/// A single hisory item record - represents one change event.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,

    #[serde(default)]
    pub messages_added: Vec<HistoryMessageAdded>,

    #[serde(default)]
    pub messages_deleted: Vec<HistoryMessageDeleted>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryMessageAdded {
    pub message: MessageRef,
}

/// A single message deleted from the history.
#[derive(Debug, Deserialize)]
pub struct HistoryMessageDeleted {
    pub message: MessageRef,
}

// ---- Helper functions ---------------------------------------------------------------

/// Gmail returns `internalDate` as a string containing a unix timestamp in milliseconds.
/// This deserializer handles both string and number formats.
fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;


    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Number(num)) => Ok(num.as_i64()),
        Some(serde_json::Value::String(s)) => {
            s.parse::<i64>().map(Some).map_err(|_| {
                D::Error::custom(format!("invalid timestamp format: {s}"))
            })
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected timestamp as string or number, got: {other}"
        ))),
    }
}

/// ---- Helper Methods ---------------------------------------------------------------
impl GmailMessage {
    pub fn headers(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.payload
            .as_ref()?
            .headers
            .iter()
            .find(|h| h.name.to_lowercase() == name_lower)
            .map(|h| h.value.as_str())
    }

    pub fn subject(&self) -> Option<&str> {
        self.headers("subject")
    }

    pub fn from(&self) -> Option<&str> {
        self.headers("from")
    }

    pub fn to(&self) -> Option<&str> {
        self.headers("to")
    }

    pub fn date(&self) -> Option<&str> {
        self.headers("date")
    }

    /// Recursively extract all email addresses from the message parts.
    /// Prefers `text/plain`, falls back to stripping `text/html` of tags.
    pub fn plain_text_body(&self) -> Option<String> {
        let payload = self.payload.as_ref()?;
        extract_plain_text(payload)
    }

    pub fn timestamp_secs(&self) -> Option<i64> {
        self.internal_date.map(|ms| ms / 1000)
    }
}

/// Recursively walk message parts to extract text content
fn extract_plain_text(part: &MessagePart) -> Option<String> {
    let mime = part.mime_type.as_deref().unwrap_or("");

    // Direct plain text part - decode base64 body
    if mime == "text/plain" {
        if let Some(body) = &part.body {
            if let Some(data) = &body.data {
                return decode_base64(data);
            }
        }
    }

    // Recurse into sub-parts (multipart/mixed, multipart/alternatie ..etc.);
    for sub in &part.parts {
        if let Some(text) = extract_plain_text(sub) {
            return Some(text);
        }
    }

    None
}

// DFecode Gmail's base64 encoded body data
pub fn decode_base64(data: &str) -> Option<String> {
    use base64::Engine;

    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .ok()?;

    String::from_utf8(bytes).ok()
}

impl GmailThread {
    pub fn messages(&self) -> impl Iterator<Item = &GmailMessage> {
        self.messages.iter()
    }
}


/// ---- Tests -------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
 
    fn make_message(headers: Vec<(&str, &str)>, body_data: Option<&str>) -> GmailMessage {
        GmailMessage {
            id:            "msg_001".to_string(),
            thread_id:     "thread_001".to_string(),
            label_ids:     vec!["INBOX".to_string()],
            snippet:       Some("Hello world".to_string()),
            size_estimate: Some(1024),
            internal_date: Some(1_700_000_000_000), // ms
            history_id:    Some("12345".to_string()),
            payload: Some(MessagePart {
                mime_type: Some("text/plain".to_string()),
                headers: headers
                    .into_iter()
                    .map(|(n, v)| MessageHeader {
                        name:  n.to_string(),
                        value: v.to_string(),
                    })
                    .collect(),
                body: body_data.map(|d| MessageBody {
                    data:          Some(d.to_string()),
                    attachment_id: None,
                    size:          Some(d.len() as i64),
                }),
                parts: vec![],
            }),
        }
    }
 
    #[test]
    fn test_header_extraction() {
        let msg = make_message(
            vec![
                ("From",    "alice@example.com"),
                ("To",      "bob@example.com"),
                ("Subject", "Hello Bob"),
                ("Date",    "Mon, 01 Jan 2024 00:00:00 +0000"),
            ],
            None,
        );
        assert_eq!(msg.from(),    Some("alice@example.com"));
        assert_eq!(msg.to(),      Some("bob@example.com"));
        assert_eq!(msg.subject(), Some("Hello Bob"));
        assert!(msg.date().is_some());
    }
 
    #[test]
    fn test_header_case_insensitive() {
        let msg = make_message(vec![("SUBJECT", "Test")], None);
        assert_eq!(msg.headers("subject"), Some("Test"));
        assert_eq!(msg.headers("Subject"), Some("Test"));
        assert_eq!(msg.headers("SUBJECT"), Some("Test"));
    }
 
    #[test]
    fn test_timestamp_secs() {
        let msg = make_message(vec![], None);
        // 1_700_000_000_000 ms → 1_700_000_000 s
        assert_eq!(msg.timestamp_secs(), Some(1_700_000_000));
    }
 
    #[test]
    fn test_decode_base64_body() {
        // base64url encode "Hello, Fluvio!" manually:
        // echo -n "Hello, Fluvio!" | base64 | tr '+/' '-_' | tr -d '='
        use base64::Engine;
        let original = "Hello, Fluvio!";
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(original.as_bytes());
 
        let decoded = decode_base64(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
 
    #[test]
    fn test_plain_text_body_extraction() {
        use base64::Engine;
        let text = "This is the email body.";
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(text.as_bytes());
 
        let msg = make_message(vec![], Some(&encoded));
        assert_eq!(msg.plain_text_body(), Some(text.to_string()));
    }
 
    #[test]
    fn test_deserialize_message_from_json() {
        let json = r#"{
            "id": "abc123",
            "threadId": "thread456",
            "labelIds": ["INBOX", "UNREAD"],
            "snippet": "Hey there",
            "internalDate": "1700000000000",
            "payload": {
                "mimeType": "text/plain",
                "headers": [
                    {"name": "From", "value": "sender@example.com"},
                    {"name": "Subject", "value": "Test email"}
                ],
                "body": {"size": 0},
                "parts": []
            }
        }"#;
 
        let msg: GmailMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.id, "abc123");
        assert_eq!(msg.thread_id, "thread456");
        assert_eq!(msg.label_ids, vec!["INBOX", "UNREAD"]);
        assert_eq!(msg.from(), Some("sender@example.com"));
        assert_eq!(msg.subject(), Some("Test email"));
        assert_eq!(msg.timestamp_secs(), Some(1_700_000_000));
    }
 
    #[test]
    fn test_deserialize_thread_from_json() {
        let json = r#"{
            "id": "thread001",
            "snippet": "Thread snippet",
            "messages": [
                {"id": "m1", "threadId": "thread001", "labelIds": []},
                {"id": "m2", "threadId": "thread001", "labelIds": ["SENT"]}
            ]
        }"#;
 
        let thread: GmailThread = serde_json::from_str(json).unwrap();
        assert_eq!(thread.id, "thread001");
        assert_eq!(thread.messages.len(), 2);
        assert_eq!(thread.messages[0].id, "m1");
        assert_eq!(thread.messages[1].label_ids, vec!["SENT"]);
    }
 
    #[test]
    fn test_deserialize_label_list() {
        let json = r#"{
            "labels": [
                {"id": "INBOX", "name": "INBOX", "type": "system"},
                {"id": "Label_1", "name": "Work", "type": "user"}
            ]
        }"#;
 
        let res: LabelListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.labels.len(), 2);
        assert_eq!(res.labels[0].id, "INBOX");
        assert_eq!(res.labels[1].name, "Work");
    }
 
    #[test]
    fn test_message_list_response_pagination() {
        let json = r#"{
            "messages": [
                {"id": "m1", "threadId": "t1"},
                {"id": "m2", "threadId": "t2"}
            ],
            "nextPageToken": "token_abc",
            "resultSizeEstimate": 42
        }"#;
 
        let res: MessageListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.messages.len(), 2);
        assert_eq!(res.next_page_token, Some("token_abc".to_string()));
        assert_eq!(res.result_size_estimate, Some(42));
    }
 
    #[test]
    fn test_empty_message_list() {
        let json = r#"{"resultSizeEstimate": 0}"#;
        let res: MessageListResponse = serde_json::from_str(json).unwrap();
        assert!(res.messages.is_empty());
        assert!(res.next_page_token.is_none());
    }
}