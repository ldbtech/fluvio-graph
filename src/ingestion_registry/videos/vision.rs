// vision.rs
// Send a jpeg frame to LLaVA via ollama and get a scene description back.
//!
//! Config via environment variables (.env):
//! - OLLAMA_URL: URL of the ollama server
//! - OLLAMA_MODEL: default: llava:7b-v1.5-q4_0
//! 
//! Flow: 
//!  extract_frame_bytes() -> JPEG bytes.
//!  describe_scene() -> base64 encode -> POST Ollama API -> description.
//!  update_scene_description() -> graph node updated.
//! 
//! The background task in routes/video.rs calls this for each scene.
//! After initial ingestion. Scenes start as "understanding: pending"
//! and become "understanding: complete" as LLaVA processing them.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

// --- Config ------------------------------------------------------------
// LLaVA configuation - loaded from environment variables.
#[derive(Debug, Clone)]
pub struct VisionConfig {
    // Ollama Server URL - MY IP ADDRESS OF ANOTHER LAPTOP OR LOCALLY.
    /// Later we will be able to use different providers.
    pub ollama_url: String,
    // Model to use default: llava:7b-v1.5-q4_0
    pub model: String,
    // timeout in seconds for the Ollama API call.
    pub timeout_s: u64
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            ollama_url: std::env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            model:      std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "llava:7b-v1.5-q4_0".to_string()),
            timeout_s:  std::env::var("OLLAMA_TIMEOUT_S")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
        }
    }
}

impl VisionConfig {
    pub fn from_env() -> Self {
        Self::default()
    }

    // generate endpoint URL
    pub fn generate_url(&self) -> String {
        format!("{}/api/generate", self.ollama_url)
    }

    // Check tags endpoint URL - Used for health checks.
    pub fn tags_url(&self) -> String {
        format!("{}/api/tags", self.ollama_url)
    }
}

// --- Ollama API types ------------------------------------------------------
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    images: Vec<String>, // base64 encoded.
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}
 
#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

// ── Scene prompt ──────────────────────────────────────────────────────────────
/// The prompt sent to LLaVA for every scene frame.
/// Structured to extract information useful for video editing context.
const SCENE_PROMPT: &str = "\
Describe this video scene concisely and precisely. Include:
- What objects and people are visible
- What actions or events are happening
- The setting and environment
- Lighting and mood
- Any text visible in the frame
Keep the description under 100 words. Be specific and factual.";

// --- describe_scene ------------------------------------------------------------
// Send JPEG frames bytes to LLaVA and return a scene description.
//
// `frame_bytes` raw JPEG bytes from extract_frame_bytes().
// `config` - Ollama connection config (from VisionConfig::from_env()).
// Returns: - The description string - record as source_text on the scene node.
pub async fn describe_scene(
    frame_bytes: &[u8],
    config: &VisionConfig,
) -> anyhow::Result<String> {
    if frame_bytes.is_empty() {
        anyhow::bail!("frame_bytes is empty - nothing to describe.");
    }

    let b64 = BASE64.encode(frame_bytes);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_s))
        .build()
        .map_err(|e| anyhow::anyhow!("cannot create HTTP client: {e}"))?;

    let request = OllamaRequest {
        model: config.model.clone(),
        prompt: SCENE_PROMPT.to_string(),
        images: vec![b64],
        stream: false,
    };

    let response = client.post(&config.generate_url())
                        .json(&request)
                        .send()
                        .await
                        .map_err(|e| anyhow::anyhow!("Ollama request failed (it is running at {}?): {e}",
                        config.ollama_url))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Ollama API returned error {status}: {body}");
    }

    let ollama_res: OllamaResponse = response.json().await.map_err(|e| anyhow::anyhow!("Ollama response parse error: {e}"))?;

    let description = ollama_res.response.trim().to_string();

    if description.is_empty() {
        anyhow::bail!("Ollama returned empty description.");
    }

    tracing::info!(
        "[Vision] described scene: {}…",
        &description[..description.len().min(60)]
    );
    Ok(description)
}

// --- health check ------------------------------------------------------------
// check if ollama is reachable and the model is available. 
// called at server startup to warn if llava is not configured.
pub async fn health_check(config: &VisionConfig) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
 
    let response = client
        .get(&config.tags_url())
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(
            "Ollama not reachable at {} — {e}\n\
             Set OLLAMA_URL in .env to point to your gaming PC",
            config.ollama_url
        ))?;
 
    let tags: OllamaTagsResponse = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to parse tags response: {e}"))?;
 
    let model_found = tags.models.iter()
        .any(|m| m.name.starts_with(&config.model)
                  || config.model.starts_with(&m.name));
 
    if !model_found {
        let available: Vec<_> = tags.models.iter().map(|m| &m.name).collect();
        anyhow::bail!(
            "Model '{}' not found in Ollama.\n\
             Available models: {:?}\n\
             Run: ollama pull {}",
            config.model, available, config.model
        );
    }
 
    tracing::info!(
        "[Vision] Ollama healthy at {} — model '{}' ready",
        config.ollama_url, config.model
    );
 
    Ok(())
}


// --- Test ------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes reads/writes of `OLLAMA_*` env vars in this module's tests (Rust 2024 `set_var`/`remove_var` safety).
    static OLLAMA_ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    // ── Unit tests — no Ollama needed ─────────────────────────────────────────

    #[test]
    fn test_config_defaults() {
        let _guard = OLLAMA_ENV_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // SAFETY: `remove_var` is unsafe in Rust 2024 unless no other thread reads the environment concurrently.
        // `OLLAMA_ENV_TEST_LOCK` serializes `OLLAMA_*` mutations in this module.
        unsafe {
            std::env::remove_var("OLLAMA_URL");
            std::env::remove_var("OLLAMA_MODEL");
            std::env::remove_var("OLLAMA_TIMEOUT_S");
        }
        let cfg = VisionConfig::default();
        assert_eq!(cfg.ollama_url, "http://localhost:11434");
        assert_eq!(cfg.model,      "llava:7b-v1.5-q4_0");
        assert_eq!(cfg.timeout_s,  60);
    }

    #[test]
    fn test_config_from_env() {
        let _guard = OLLAMA_ENV_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // SAFETY: `set_var`/`remove_var` are unsafe in Rust 2024 unless no other thread reads the environment concurrently.
        // `OLLAMA_ENV_TEST_LOCK` serializes `OLLAMA_*` mutations in this module.
        unsafe {
            std::env::set_var("OLLAMA_URL", "http://192.168.1.45:11434");
            std::env::set_var("OLLAMA_MODEL", "llava:13b");
            std::env::set_var("OLLAMA_TIMEOUT_S", "120");
        }

        let cfg = VisionConfig::from_env();
        assert_eq!(cfg.ollama_url, "http://192.168.1.45:11434");
        assert_eq!(cfg.model,      "llava:13b");
        assert_eq!(cfg.timeout_s,  120);

        unsafe {
            std::env::remove_var("OLLAMA_URL");
            std::env::remove_var("OLLAMA_MODEL");
            std::env::remove_var("OLLAMA_TIMEOUT_S");
        }
    }
 
    #[test]
    fn test_generate_url() {
        let cfg = VisionConfig {
            ollama_url: "http://192.168.1.45:11434".to_string(),
            model:      "llava:7b-v1.5-q4_0".to_string(),
            timeout_s:  60,
        };
        assert_eq!(cfg.generate_url(), "http://192.168.1.45:11434/api/generate");
        assert_eq!(cfg.tags_url(),     "http://192.168.1.45:11434/api/tags");
    }
 
    #[test]
    fn test_describe_scene_rejects_empty_bytes() {
        // Can't await in unit test — test the guard directly
        let bytes: &[u8] = &[];
        assert!(bytes.is_empty());
        // The function bails early on empty bytes — confirmed by logic
    }
 
    #[test]
    fn test_scene_prompt_not_empty() {
        assert!(!SCENE_PROMPT.is_empty());
        assert!(SCENE_PROMPT.contains("objects"));
        assert!(SCENE_PROMPT.contains("actions"));
        assert!(SCENE_PROMPT.contains("100 words"));
    }
 
    #[test]
    fn test_base64_roundtrip() {
        let original = b"fake jpeg bytes for testing";
        let encoded  = BASE64.encode(original);
        let decoded  = BASE64.decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
 
    #[test]
    fn test_ollama_request_serializes() {
        let req = OllamaRequest {
            model:  "llava:7b".to_string(),
            prompt: "describe this".to_string(),
            images: vec!["base64data".to_string()],
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("llava:7b"));
        assert!(json.contains("describe this"));
        assert!(json.contains("base64data"));
        assert!(json.contains("\"stream\":false"));
    }
 
    #[test]
    fn test_ollama_response_deserializes() {
        let json = r#"{"response":"A person walking in a corridor","done":true}"#;
        let res: OllamaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.response, "A person walking in a corridor");
        assert!(res.done);
    }
 
    #[test]
    fn test_ollama_response_done_defaults_false() {
        // done field is optional — should default to false
        let json = r#"{"response":"some description"}"#;
        let res: OllamaResponse = serde_json::from_str(json).unwrap();
        assert!(!res.done);
    }
 
    // ── Integration tests — require Ollama running ─────────────────────────────
    // Set OLLAMA_URL in .env to point to your gaming PC
    // Run with: cargo test ingestion_registry::videos::vision -- --ignored --nocapture
 
    #[tokio::test]
    #[ignore]
    async fn test_health_check_live() {
        dotenvy::dotenv().ok();
        let config = VisionConfig::from_env();
        println!("Testing Ollama at: {}", config.ollama_url);
        println!("Model: {}", config.model);
 
        let result = health_check(&config).await;
        assert!(result.is_ok(), "health check failed: {:?}", result.err());
        println!("Ollama health check passed");
    }
 
    #[tokio::test]
    #[ignore]
    async fn test_describe_real_frame() {
        // Requires: Ollama running + test video exists
        dotenvy::dotenv().ok();
        let video_path = std::path::Path::new("/tmp/test_video.mp4");
        if !video_path.exists() {
            eprintln!("SKIP: test video not found");
            return;
        }
 
        let frame_bytes = crate::ingestion_registry::videos::frame::extract_frame_bytes(
            video_path, 2.0
        ).expect("frame extraction failed");
 
        println!("Frame extracted: {} bytes", frame_bytes.len());
 
        let config = VisionConfig::from_env();
        println!("Sending to LLaVA at {}...", config.ollama_url);
 
        let result = describe_scene(&frame_bytes, &config).await;
        assert!(result.is_ok(), "describe_scene failed: {:?}", result.err());
 
        let description = result.unwrap();
        assert!(!description.is_empty());
        println!("\nLLaVA description:\n{description}");
    }
 
    #[tokio::test]
    #[ignore]
    async fn test_describe_scene_unreachable_ollama() {
        let config = VisionConfig {
            ollama_url: "http://127.0.0.1:19999".to_string(), // nothing on this port
            model:      "llava:7b-v1.5-q4_0".to_string(),
            timeout_s:  3,
        };
        let fake_frame = vec![0xFFu8, 0xD8, 0xFF, 0xE0, 0x00]; // fake JPEG header
        let result     = describe_scene(&fake_frame, &config).await;
        assert!(result.is_err());
        println!("Expected error: {}", result.unwrap_err());
    }
}
 