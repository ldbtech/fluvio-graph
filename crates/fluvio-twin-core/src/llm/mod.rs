//! LLM prompt assembly. The Anthropic-only client that used to live here has
//! moved to the shared, multi-provider `fluvio-llm` crate — see
//! `servers/twin-server/src/graphql/query.rs` for the call site.

pub mod prompt;

pub use prompt::{
    build_context_from_seeds, build_system_prompt, BFS_DEPTH, SEED_K, SIM_TOP_K,
};
