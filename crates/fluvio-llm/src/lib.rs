//! # fluvio-llm
//!
//! Shared multi-provider LLM client used by twin-server, collab-server, and
//! database-server. Three wire-format implementations cover four providers:
//! native Anthropic, native Gemini, and one OpenAI-compatible path that
//! serves OpenAI itself plus Ollama and any other self-hosted model that
//! speaks the OpenAI chat-completions shape.
//!
//! Also owns the AES-256-GCM encryption for BYOK credentials at rest, and the
//! HTTP client (`resolver`) every Rust service uses to resolve a per-user
//! provider connection from database-server's internal (non-GraphQL) route.

pub mod types;
pub mod chat;
pub mod crypto;
pub mod resolver;

mod anthropic;
mod gemini;
mod openai_compat;
