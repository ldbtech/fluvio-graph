//! videos/extractor.rs
//! 
//! FFmpeg-next bindings for video ingestion.
//! Two jobs only:
//!   1. Extract video metadata (duration, fps, resolution, codec)
//!   2. Detect scene boundaries → Vec<SceneBoundary>
//! 
//! No JPEG writing. No filters. No thumbnails. No editing.
//! Editing is a separate system.
//! 
//! Scene detection algorithm:
//!   Sample frames at configurable rate (default every 15 frames)
//!   Compute mean absolute difference (MAD) between consecutive samples
//!   MAD > threshold → scene boundary at this timestamp
//!   Returns: Vec<SceneBoundary> with time_start, time_end, score
//! 
//! Each SceneBoundary becomes one graph node.
//! The scene is the unit of semantic understanding — not the frame.
//! 
use std::path::Path;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::{format, media, software::scaling, util::frame::video::Video};
use serde::{Deserialize, Serialize};

// --- VideoMetadata ------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub video_id:     String,
    pub duration:     f64,          // total duration in seconds
    pub fps:          f64,          // frames per second
    pub width:        u32,          // frame width in pixels
    pub height:       u32,          // frame height in pixels
    pub codec:        String,       // e.g. "h264", "hevc", "vp9"
    pub file_size:    u64,          // bytes
    pub total_frames: u64,          // estimated total frame count
    pub has_audio:    bool,
    pub audio_codec:  Option<String>,
}

// -- SceneBoundary ------------------------------------------------------------

// Detected scene in the video
// time_start/time_end define the temporal extent. 
// Score is the scene change confidence at the cut point (0.0 - 1.0)
// 
// Each SceneBoundary -> one Normalizedchunk -> One Graph Node.
// souce_text start empty - filled by LLaVA/Claude or any other Text providers 
// in a background task after ingestion.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneBoundary {
    pub index:        usize,
    pub time_start:   f64, // seconds
    pub time_end:     f64, // seconds
    pub duration:     f64, // time_end - time_start
    pub frame_start:  u64, // frame index
    pub frame_end:    u64, // approx frame number
    // Scene change score at the cut point
    // 0.0 : First Scene (no cut before it)
    // 0.3 - 1.0 = detected cut confidence 
    pub score:        f64,
    // Representation timestamp for visual sampling. 
    // Set to 40% into the scene - used by LLaVA background task.
    // To pick the most representative frame for understanding.
    pub sample_time:  f64,
}

impl SceneBoundary {
    pub fn new(
        index: usize,
        time_start: f64,
        time_end: f64,
        frame_start: u64,
        frame_end: u64,
        score: f64,
    ) -> Self {
        let duration = (time_end - time_start).max(0.0);
        let sample_time = time_start + duration * 0.4;
        Self {
            index, time_start, time_end, 
            duration, frame_start, frame_end, 
            score, sample_time,
        }
    }
}

// --- SceneDetectingConfig -----------------------------------------------------

// Configuration for scene detection.
// 
//Tune threshold per video type: 
//   0.15 = very sensitive (documentaries, slow cuts)
//   0.30 = sensitive (general content narrative film)
//   0.50 = aggressive (only major scene changes)
#[derive(Debug, Clone)]
pub struct SceneDetectionConfig {
    // Pixel difference threshold for scene cut detection (0.0 - 1.0)
    pub threshold: f64,
    // Sample every N Frames - higher = faster, less precise.
    pub sample_every: u32,
    // Minimum scene duration in seconds - prevents micro-scene noise.
    pub min_duration: f64,
    // Hard cap on scene count - prevents graph explosion on noisy video.
    pub max_scenes: usize,
}

impl Default for SceneDetectionConfig {
    fn default() -> Self {
        Self {
            threshold:    0.30,
            sample_every: 15,    // ~2fps for 30fps video
            min_duration: 1.5,
            max_scenes:   200,
        }
    }
}

// --- Extract_metadata ---------------------------------------------------------

// Extract structural metadata from a video file.
// Fast - reads container headers only, does not decode frames.
pub fn extract_metadata(
    path: &Path,
    video_id: &str,
) -> anyhow::Result<VideoMetadata>{
    ffmpeg::init().map_err(|e| anyhow::anyhow!("ffmpeg init: {e}"))?;

    let ctx = format::input(path)
             .map_err(|e| anyhow::anyhow!("cannot open {} : {e}", path.display()))?;
    
    // duration 
    let duration = ctx.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE);

    // file size
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    // Video stream.
    let video_stream = ctx.streams()
                 .best(media::Type::Video)
                 .ok_or_else(|| anyhow::anyhow!("no videa stream found in {} ", path.display()))?;

    let params = video_stream.parameters();
    let codec_ctx = ffmpeg::codec::context::Context::from_parameters(params)
             .map_err(|e| anyhow::anyhow!("cannot create codec context: {e}"))?;
    let decoder = codec_ctx.decoder().video()
             .map_err(|e| anyhow::anyhow!("cannot create video decoder: {e}"))?;
    
    let fps = {
        let r = video_stream.avg_frame_rate();
        if r.denominator() == 0 {
            25.0
        } else {
            r.numerator() as f64 / r.denominator() as f64
        }
    };

    let width = decoder.width();
    let height = decoder.height();
    let codec_id = decoder.id();
    let total_frames = (duration * fps).ceil() as u64;

    // Audio Stream (optional)
    let audio_stream = ctx.streams().best(media::Type::Audio);
    let has_audio                  = audio_stream.is_some();
    let audio_codec   = audio_stream.and_then(|s| {
        ffmpeg::codec::context::Context::from_parameters(s.parameters()).ok()
               .and_then(|c| c.decoder().audio().ok())
               .map(|a| format!("{:?}", a.id()).to_lowercase())
    });

    Ok(VideoMetadata {
        video_id:     video_id.to_string(),
        duration,
        fps,
        width,
        height,
        codec:        format!("{codec_id:?}").to_lowercase(),
        file_size,
        total_frames,
        has_audio,
        audio_codec,
    })
}

// --- detect_scenes ------------------------------------------------------------

// Detect Scene boundaries in a video file.
// Returns Vec<SceneBoundary> - at minimum one scene (full video)
// 
// Algorithm:
//    1. Decode every `simple_every` frames.
//    2. Downscale ewach to 160*90 grayscale (fast MAD)
//    3. MAD > threshold -> scene cut at this timestamp.
//    4. Merge scenes shorter than min_duration.
//    5. Enforce max_scenes cap.
pub fn detect_scenes(
    path: &Path,
    config: &SceneDetectionConfig,
    meta: &VideoMetadata,
) -> anyhow::Result<Vec<SceneBoundary>> {
    ffmpeg::init().map_err(|e| anyhow::anyhow!("ffmpeg init: {e}"))?;

    let mut ctx = format::input(path)
             .map_err(|e| anyhow::anyhow!("cannot open {}: {e}", path.display()))?;
    
    let video_idx = ctx.streams().best(media::Type::Video)
             .ok_or_else(|| anyhow::anyhow!("No video stream"))?
             .index();

    // Build decoder.
    let params = ctx.stream(video_idx).unwrap().parameters();
    let decode_ctx = ffmpeg::codec::context::Context::from_parameters(params)
            .map_err(|e| anyhow::anyhow!("codec context: {e}"))?;

    let mut decoder = decode_ctx.decoder().video()
            .map_err(|e| anyhow::anyhow!("decoder: {e}"))?;
    
    let time_base = ctx.stream(video_idx).unwrap().time_base();

    // Scaler: decode resolution -> 160*90 grayscale (fast MAD)
    let scale_w = 160u32;
    let scale_h = 90u32;
    let mut scaler = scaling::context::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::GRAY8,
        scale_w,
        scale_h,
        scaling::Flags::BILINEAR,
    ).map_err(|e| anyhow::anyhow!("scaler: {e}"))?;

    // Cut points (timestamp_seconds, mad_score)
    let mut cuts: Vec<(f64, f64)> = vec![(0.0, 0.0)];

    let mut prev_pixels: Option<Vec<u8>> = None;
    let mut frame_num      = 0u64;
    let mut decoded      = Video::empty();
    let mut scaled       = Video::empty();
    let sample_every       = config.sample_every.max(1) as u64;

    for (stream, packet) in ctx.packets() {
        if stream.index() != video_idx { continue; }
        if decoder.send_packet(&packet).is_err() { continue; }

        while decoder.receive_frame(&mut decoded).is_ok() {
            frame_num += 1;

            if frame_num % sample_every != 0 { continue; }

            let pts = decoded.pts().unwrap_or(0);
            let ts = pts as f64 * time_base.numerator() as f64 / time_base.denominator() as f64;

            if ts < 0.0 || ts > meta.duration + 0.5 { continue;}

            if scaler.run(&decoded, &mut scaled).is_err() { continue; }
            let pixels = scaled.data(0).to_vec();

            if let Some(ref prev) = prev_pixels {
                let mad = mean_absolute_difference(prev, &pixels);
                if mad > config.threshold {
                    tracing::debug!("scene cut detected at {ts:.2}s (MAD: {mad:.2})");
                    cuts.push((ts, mad));
                }
            }

            prev_pixels = Some(pixels);
        }
    }

    // flush decoder
    decoder.send_eof().ok();
    while decoder.receive_frame(&mut decoded).is_ok() {}

    // Build scene boundaries.
    let fps = meta.fps.max(1.0) as f64;
    let total = meta.duration;
    let mut scenes: Vec<SceneBoundary> = Vec::new();

    for (i, &(t_start, score)) in cuts.iter().enumerate() {
        let t_end = cuts.get(i + 1).map(|&(t, _)| t).unwrap_or(total);
        let duration = t_end - t_start;

        // Skip short scenes (merge into next) unless it's the last one.
        if duration < config.min_duration && i + 1 < cuts.len() {
            tracing::debug!("[SceneDetect] merge short scene {t_start:.2}s - {t_end:.2}s");
            continue;
        }

        scenes.push(SceneBoundary::new(
            scenes.len(),
            t_start,
            t_end,
            (t_start * fps).round() as u64,
            (t_end * fps).round() as u64,
            score,
        ));

        if scenes.len() >= config.max_scenes { 
            tracing::warn!("[SceneDetect] max_scenes cap reached ({})", config.max_scenes);
            if let Some(last) = scenes.last_mut() {
                last.time_end = total;
                last.duration = total - last.time_start;
                last.frame_end = (total * fps).round() as u64;
                last.sample_time = last.time_start + last.duration * 0.4;
            }
            break; 
        }
    }

    if scenes.is_empty() {
        tracing::warn!("[SceneDetect] no scenes detected - returning full video as single scene");
        scenes.push(SceneBoundary::new(
            0, 0.0, total,
            0, (total * fps) as u64,
            0.0,
        ));
    }

    tracing::info!("[SceneDetect] detected {} scenes in {:.2}s video ({})",         scenes.len(), total, path.display());
    Ok(scenes)
}

// ── mean_absolute_difference ──────────────────────────────────────────────────
 
// Compute mean absolute difference between two GRAY8 pixel buffers.
// Returns [0.0, 1.0] — 0.0 = identical, 1.0 = completely different.
fn mean_absolute_difference(a: &[u8], b: &[u8]) -> f64 {
    if a.is_empty() || a.len() != b.len() { return 0.0; }
    let sum: u64 = a.iter().zip(b.iter())
        .map(|(&pa, &pb)| (pa as i32 - pb as i32).unsigned_abs() as u64)
        .sum();
    sum as f64 / (a.len() as f64 * 255.0)
}

// ── Tests ────────────────────────────────────────────
// ── Tests ─────────────────────────────────────────────────────────────────────
 
#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn test_mad_identical() {
        let a = vec![100u8; 100];
        assert_eq!(mean_absolute_difference(&a, &a.clone()), 0.0);
    }
 
    #[test]
    fn test_mad_opposite() {
        let a = vec![0u8;   100];
        let b = vec![255u8; 100];
        assert!((mean_absolute_difference(&a, &b) - 1.0).abs() < 0.001);
    }
 
    #[test]
    fn test_mad_empty() {
        assert_eq!(mean_absolute_difference(&[], &[]), 0.0);
    }
 
    #[test]
    fn test_mad_length_mismatch() {
        let a = vec![100u8; 10];
        let b = vec![100u8; 20];
        assert_eq!(mean_absolute_difference(&a, &b), 0.0);
    }
 
    #[test]
    fn test_mad_half_difference() {
        let a = vec![0u8;   100];
        let b = vec![128u8; 100];
        let mad = mean_absolute_difference(&a, &b);
        assert!(mad > 0.45 && mad < 0.55);
    }
 
    #[test]
    fn test_scene_boundary_new() {
        let s = SceneBoundary::new(0, 0.0, 10.0, 0, 300, 0.0);
        assert_eq!(s.index,       0);
        assert_eq!(s.time_start,  0.0);
        assert_eq!(s.time_end,    10.0);
        assert_eq!(s.duration,    10.0);
        assert_eq!(s.frame_start, 0);
        assert_eq!(s.frame_end,   300);
        assert!((s.sample_time - 4.0).abs() < 0.01);
    }
 
    #[test]
    fn test_scene_sample_time_at_40_percent() {
        let s = SceneBoundary::new(1, 8.0, 22.0, 240, 660, 0.42);
        let expected = 8.0 + (22.0 - 8.0) * 0.4;
        assert!((s.sample_time - expected).abs() < 0.01);
    }
 
    #[test]
    fn test_scene_duration_computed() {
        let s = SceneBoundary::new(0, 5.5, 18.3, 165, 549, 0.35);
        assert!((s.duration - 12.8).abs() < 0.01);
    }
 
    #[test]
    fn test_video_metadata_total_frames() {
        let meta = VideoMetadata {
            video_id: "t".into(), duration: 10.0, fps: 30.0,
            width: 1920, height: 1080, codec: "h264".into(),
            file_size: 0, total_frames: 300,
            has_audio: false, audio_codec: None,
        };
        assert_eq!(meta.total_frames, 300);
        assert!((meta.fps - 30.0).abs() < 0.01);
    }
}