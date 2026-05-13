// video/normalizer.rs
//
// Converts Vec<SceneBoundary> + VideoMetadata into Vec<NormalizedChunk>
// for ingestion into DomainGraph.
//
// Two node types: 

// 1. Video Node - one per video file 
//    - source_uri = "video://{video_id}"
//    Contains edges to all scene nodes.
//
// 2. Scene Node - one per detected scene.
//!      source_uri: "video://{video_id}/scene/{index}"
//!      source_text: structural placeholder until LLaVA fills it in
//!      metadata: time_start, time_end, duration, frame_start, frame_end,
//!                score, sample_time, width, height, fps, understanding
//! 
//! Edges: 
//!   video   → contains   → scene_N    (one per scene)
//!   scene_N → part_of    → video
//!   scene_N → next_scene → scene_N+1
//!   scene_N → prev_scene → scene_N-1
//! 
//!
//! The `understanding` metadata field starts as "pending".
//! Background task (LLaVA/Claude) sets it to "complete" and
//! updates source_text with rich semantic description.

use std::collections::HashMap;
use crate::ingestion_registry::connector::{NormalizedChunk, PreDefinedEdge};
use crate::graph::enums::Domain;
use super::extractor::{SceneBoundary, VideoMetadata};

// --- Domain + URI Helpers ------------------------------------------------------
pub fn video_domain() -> Domain {
    Domain::Custom("video".to_string())
}

pub fn video_uri(video_id: &str) -> String {
    format!("video://{}", video_id)
}

pub fn scene_uri(video_id: &str, scene_index: usize) -> String {
    format!("video://{}/scene/{}", video_id, scene_index)
}


// Helpers --------------------------------------------------------------------
// --- scene_chunk ------------------------------------------------------------
// Build a single scene NormalizedChunk.
// Wires Temporal edges to adjacent scenes. 
fn scene_chunk(
    meta: &VideoMetadata,
    scene: &SceneBoundary,
    all_scenes: &[SceneBoundary],
    chunk_index: usize,
) -> NormalizedChunk {

    let video_id = &meta.video_id;
    let source_uri = scene_uri(video_id, scene.index);
    let i = scene.index;

    // Structural placeholder — LLaVA updates this asynchronously
    let text = format!(
        "video scene {idx} of {total}\nvideo: {video_id}\n\
         time: {ts:.2}s to {te:.2}s\nduration: {dur:.2}s\n\
         frames: {fs} to {fe}\nscene change score: {score:.3}\n\
         understanding: pending",
        idx   = i,
        total = all_scenes.len(),
        ts    = scene.time_start,
        te    = scene.time_end,
        dur   = scene.duration,
        fs    = scene.frame_start,
        fe    = scene.frame_end,
        score = scene.score,
    );

    let metadata = scene_metadata(meta, scene);
    let edges = scene_edges(video_id, i, all_scenes);

    NormalizedChunk {
        text,
        metadata,
        chunk_index,
        source_uri,
        domain:            video_domain(),
        pre_defined_edges: edges,
    }
}

// --- scene_edges ------------------------------------------------------------
// Build temporal + structural edges for a scene node.
// part_of -> video, next_scene -> prev_scene <-
fn scene_edges(
    video_id:   &str,
    scene_idx:  usize,
    all_scenes: &[SceneBoundary],
) -> Vec<PreDefinedEdge> {

    let mut edges = vec![
        // scene → part_of → video
        PreDefinedEdge {
            to_uri:                   video_uri(video_id),
            label:                    "part_of".to_string(),
            relationship_probability: 1.0,
            token_cost:               0,
        },
    ];

    if scene_idx + 1 < all_scenes.len() {
        edges.push(PreDefinedEdge {
            to_uri:                   scene_uri(video_id, scene_idx + 1),
            label:                    "next_scene".to_string(),
            relationship_probability: 1.0,
            token_cost:               1,
        });
    }

    if scene_idx > 0 {
        edges.push(PreDefinedEdge {
            to_uri:                   scene_uri(video_id, all_scenes[scene_idx - 1].index),
            label:                    "prev_scene".to_string(),
            relationship_probability: 1.0,
            token_cost:               1,
        });
    }

    edges
}

// --- scenes_to_chunks -----------------------------------------------------------

// Convert VideoMetadata + Vec<SceneBoundary> into NormalizedChunks.
// 
// Output order: 
// chunks[0] = video-level chunk
// chunks[1..N+1] = scene chunks (one per scene)
//
// start_index: chunk_index offset (for multi-video ingestion sessions).
pub fn scenes_to_chunks(
    meta:        &VideoMetadata,
    scenes:      &[SceneBoundary],
    start_index: usize,
) -> Vec<NormalizedChunk> {
    let mut chunks = Vec::with_capacity(1 + scenes.len());

    chunks.push(video_chunk(meta, scenes, start_index));

    for (i, scene) in scenes.iter().enumerate() {
        chunks.push(scene_chunk(meta, scene, scenes, start_index + 1 + i));
    }

    chunks
}

// Metadata: 
/// Build metadata for a scene node.
/// Build metadata for a scene node.
fn scene_metadata(meta: &VideoMetadata, scene: &SceneBoundary) -> HashMap<String, String> {
    HashMap::from([
        ("kind",          "scene".to_string()),
        ("video_id",      meta.video_id.clone()),
        ("scene_index",   scene.index.to_string()),
        ("time_start",    format!("{:.6}", scene.time_start)),
        ("time_end",      format!("{:.6}", scene.time_end)),
        ("duration",      format!("{:.6}", scene.duration)),
        ("frame_start",   scene.frame_start.to_string()),
        ("frame_end",     scene.frame_end.to_string()),
        ("score",         format!("{:.6}", scene.score)),
        ("sample_time",   format!("{:.6}", scene.sample_time)),
        ("width",         meta.width.to_string()),
        ("height",        meta.height.to_string()),
        ("fps",           format!("{:.3}", meta.fps)),
        ("understanding", "pending".to_string()),
    ].map(|(k, v)| (k.to_string(), v)))
}

/// Build metadata for the video node.
fn video_metadatas(meta: &VideoMetadata, scene_count: usize) -> HashMap<String, String> {
    let mut m = HashMap::from([
        ("kind",          "video".to_string()),
        ("video_id",      meta.video_id.clone()),
        ("duration",      format!("{:.3}", meta.duration)),
        ("fps",           format!("{:.3}", meta.fps)),
        ("width",         meta.width.to_string()),
        ("height",        meta.height.to_string()),
        ("codec",         meta.codec.clone()),
        ("file_size",     meta.file_size.to_string()),
        ("total_frames",  meta.total_frames.to_string()),
        ("has_audio",     meta.has_audio.to_string()),
        ("scene_count",   scene_count.to_string()),
        ("understanding", "pending".to_string()),
    ].map(|(k, v)| (k.to_string(), v)));

    if let Some(ref ac) = meta.audio_codec {
        m.insert("audio_codec".to_string(), ac.clone());
    }
    m
}

// video_chunk ------------------------------------------------------------
// Build the video level NormalizedChunk.
// Contains edges to every scene node.
fn video_chunk(
    meta: &VideoMetadata,
    scenes: &[SceneBoundary],
    chunk_index: usize,
) -> NormalizedChunk {
    let video_id = &meta.video_id;

    let text = format!(
        "video: {video_id}\nduration: {dur:.1}s\nfps: {fps:.1}\n\
         resolution: {w}x{h}\ncodec: {codec}\nscenes: {sc}\naudio: {audio}",
        dur   = meta.duration,
        fps   = meta.fps,
        w     = meta.width,
        h     = meta.height,
        codec = meta.codec,
        sc    = scenes.len(),
        audio = if meta.has_audio {
            meta.audio_codec.as_deref().unwrap_or("yes")
        } else { "none" },
    );
    let metadata = video_metadatas(meta, scenes.len());

    // video -> contains -> every scene.
    let edges = scenes.iter().map(|s| PreDefinedEdge {
        to_uri:                      scene_uri(video_id, s.index    ),
        label:                       "contains".to_string(),
        relationship_probability:    1.0,
        token_cost:                  0,
    }).collect();

    NormalizedChunk {
        text,
        metadata,
        chunk_index,
        source_uri:        video_uri(video_id),
        domain:            video_domain(),
        pre_defined_edges: edges,
    }
}

// --- update_scene_description -------------------------------------------------
/// Update a scene node after LLaVA/Claude vision has processed it.
/// Replaces placeholder text with rich semantic description.
/// Sets understanding = "complete".
///
/// Called by the background vision task — not during initial ingestion.
pub fn update_scene_understanding(
    video_id:    &str,
    scene_index: usize,
    description: &str,
    graph:       &mut crate::graph::structs::DomainGraph,
) {
    let uri = scene_uri(video_id, scene_index);
 
    for node in graph.nodes.values_mut() {
        if node.source_uri != uri { continue; }
        node.source_text = description.to_string();
        node.metadata.insert("understanding".into(), "complete".into());
        node.metadata.insert("description".into(),   description.to_string());
        tracing::info!(
            "[VideoNorm] scene {scene_index} understanding complete: {}…",
            &description[..description.len().min(60)]
        );
        break;
    }
}

/// After a frame extract or Ollama/LLaVA error, mark the scene so `/video/{id}/status` can finish.
pub fn mark_scene_understanding_failed(
    video_id: &str,
    scene_index: usize,
    err: &str,
    graph: &mut crate::graph::structs::DomainGraph,
) {
    let uri = scene_uri(video_id, scene_index);
    let short: String = err.chars().take(280).collect();
    for node in graph.nodes.values_mut() {
        if node.source_uri != uri {
            continue;
        }
        node.metadata.insert("understanding".into(), "failed".into());
        node.metadata.insert("understanding_error".into(), short.clone());
        if node.source_text.contains("understanding: pending") {
            node.source_text = node
                .source_text
                .replace("understanding: pending", &format!("understanding: failed — {short}"));
        }
        tracing::warn!("[VideoNorm] scene {scene_index} understanding failed: {short}");
        break;
    }
}

// --- tests --------------------------------------------------------------------
// ── Tests ─────────────────────────────────────────────────────────────────────
 
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::extractor::{SceneBoundary, VideoMetadata};
 
    fn meta() -> VideoMetadata {
        VideoMetadata {
            video_id: "test-001".into(), duration: 45.2, fps: 30.0,
            width: 1920, height: 1080, codec: "h264".into(),
            file_size: 1_048_576, total_frames: 1356,
            has_audio: true, audio_codec: Some("aac".into()),
        }
    }
 
    fn scenes() -> Vec<SceneBoundary> {
        vec![
            SceneBoundary::new(0,  0.0,  8.3, 0,   249,  0.0),
            SceneBoundary::new(1,  8.3, 22.1, 249, 663,  0.42),
            SceneBoundary::new(2, 22.1, 45.2, 663, 1356, 0.38),
        ]
    }
 
    #[test]
    fn test_chunk_count() {
        assert_eq!(scenes_to_chunks(&meta(), &scenes(), 0).len(), 4);
    }
 
    #[test]
    fn test_video_chunk_first() {
        let c = scenes_to_chunks(&meta(), &scenes(), 0);
        assert_eq!(c[0].metadata["kind"],     "video");
        assert_eq!(c[0].source_uri,           "video://test-001");
        assert_eq!(c[0].domain,               video_domain());
    }
 
    #[test]
    fn test_video_contains_edges() {
        let c = scenes_to_chunks(&meta(), &scenes(), 0);
        let contains: Vec<_> = c[0].pre_defined_edges.iter()
            .filter(|e| e.label == "contains").collect();
        assert_eq!(contains.len(), 3);
    }
 
    #[test]
    fn test_scene_metadata() {
        let c = scenes_to_chunks(&meta(), &scenes(), 0);
        assert_eq!(c[1].metadata["kind"],          "scene");
        assert_eq!(c[1].metadata["scene_index"],   "0");
        assert_eq!(c[1].metadata["understanding"], "pending");
    }
 
    #[test]
    fn test_uri_formats() {
        assert_eq!(video_uri("v1"),       "video://v1");
        assert_eq!(scene_uri("v1", 0),   "video://v1/scene/0");
        assert_eq!(scene_uri("v1", 2),   "video://v1/scene/2");
    }
 
    #[test]
    fn test_next_scene_edge() {
        let c    = scenes_to_chunks(&meta(), &scenes(), 0);
        let next: Vec<_> = c[1].pre_defined_edges.iter()
            .filter(|e| e.label == "next_scene").collect();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].to_uri, "video://test-001/scene/1");
    }
 
    #[test]
    fn test_prev_scene_edge() {
        let c    = scenes_to_chunks(&meta(), &scenes(), 0);
        let prev: Vec<_> = c[2].pre_defined_edges.iter()
            .filter(|e| e.label == "prev_scene").collect();
        assert_eq!(prev.len(), 1);
        assert_eq!(prev[0].to_uri, "video://test-001/scene/0");
    }
 
    #[test]
    fn test_first_scene_no_prev() {
        let c = scenes_to_chunks(&meta(), &scenes(), 0);
        let p: Vec<_> = c[1].pre_defined_edges.iter()
            .filter(|e| e.label == "prev_scene").collect();
        assert_eq!(p.len(), 0);
    }
 
    #[test]
    fn test_last_scene_no_next() {
        let c = scenes_to_chunks(&meta(), &scenes(), 0);
        let n: Vec<_> = c.last().unwrap().pre_defined_edges.iter()
            .filter(|e| e.label == "next_scene").collect();
        assert_eq!(n.len(), 0);
    }
 
    #[test]
    fn test_part_of_on_every_scene() {
        let c = scenes_to_chunks(&meta(), &scenes(), 0);
        for chunk in c.iter().skip(1) {
            let p: Vec<_> = chunk.pre_defined_edges.iter()
                .filter(|e| e.label == "part_of").collect();
            assert_eq!(p.len(), 1);
            assert_eq!(p[0].to_uri, "video://test-001");
        }
    }
 
    #[test]
    fn test_chunk_indices_sequential() {
        let c = scenes_to_chunks(&meta(), &scenes(), 10);
        for (i, chunk) in c.iter().enumerate() {
            assert_eq!(chunk.chunk_index, 10 + i);
        }
    }
 
    #[test]
    fn test_all_uris_unique() {
        let c    = scenes_to_chunks(&meta(), &scenes(), 0);
        let uris: std::collections::HashSet<_> = c.iter()
            .map(|c| &c.source_uri).collect();
        assert_eq!(uris.len(), c.len());
    }
 
    #[test]
    fn test_empty_scenes() {
        let c = scenes_to_chunks(&meta(), &[], 0);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].metadata["kind"], "video");
    }
 
    #[test]
    fn test_single_scene_no_temporal() {
        let s = vec![SceneBoundary::new(0, 0.0, 45.2, 0, 1356, 0.0)];
        let c = scenes_to_chunks(&meta(), &s, 0);
        let t: Vec<_> = c[1].pre_defined_edges.iter()
            .filter(|e| e.label == "next_scene" || e.label == "prev_scene")
            .collect();
        assert_eq!(t.len(), 0);
    }
}
