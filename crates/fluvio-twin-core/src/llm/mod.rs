//! LLM prompts and Anthropic API.

pub mod anthropic;
pub mod prompt;

pub use anthropic::{chat, chat_streaming, Message, MAX_TOKENS, MODEL};
pub use prompt::{
    build_context_from_seeds, build_system_prompt, BFS_DEPTH, SEED_K, SIM_TOP_K,
};
