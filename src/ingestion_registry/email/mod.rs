pub mod auth;
pub mod client;
pub mod normalizer;
pub mod connector;
pub mod routes;
pub mod sync;
pub mod gmail_query;
pub mod reply_agent;

pub use sync::progress::{GmailSyncProgress, GmailSyncProgressSnapshot, GmailSyncResultSummary};
pub use auth::{GmailToken, TokenStoreError, load_token, save_token, delete_token, credentials_exist};
pub use client::{
    GmailMessage, 
    GmailThread, 
    GmailLabel, 
    HistoryListResponse, 
    MessageListResponse, 
    ThreadListResponse, 
    LabelListResponse
};
pub use client::{GmailClientError};