//! videos/frame.rs
//!
//! Extract a single frame from a video file at a given timestamp.
//! Returns JPEG bytes in memory — no disk write.
//!
//! Used by the LLaVA background task to sample representative
//! frames from each detected scene for visual understanding.

use std::path::Path;

use ffmpeg_next as ffmpeg;
use ffmpeg_next::{format, media, software::scaling};
use ffmpeg_next::util::frame::video::Video;
use image::ImageFormat;

/// Output dimensions sent to LLaVA.
/// 640x360 — good balance between quality and payload size.
const OUT_W: u32 = 640;
const OUT_H: u32 = 360;

/// Seek to `timestamp` seconds in the video and return the first
/// decoded frame as JPEG bytes in memory.
///
/// No files are written to disk.
/// Caller owns the returned bytes — pass directly to LLaVA as base64.
pub fn extract_frame_bytes(
    path:      &Path,
    timestamp: f64,
) -> anyhow::Result<Vec<u8>> {
    ffmpeg::init()
        .map_err(|e| anyhow::anyhow!("ffmpeg init: {e}"))?;

    let mut ctx = format::input(path)
        .map_err(|e| anyhow::anyhow!("cannot open {}: {e}", path.display()))?;

    // Find best video stream
    let video_idx = ctx
        .streams()
        .best(media::Type::Video)
        .ok_or_else(|| anyhow::anyhow!("no video stream in {}", path.display()))?
        .index();

    // Seek to timestamp
    let time_base = ctx.stream(video_idx).unwrap().time_base();
    let pts = (timestamp
        * time_base.denominator() as f64
        / time_base.numerator()   as f64) as i64;

    ctx.seek(pts, ..pts)
        .map_err(|e| anyhow::anyhow!("seek to {timestamp:.2}s failed: {e}"))?;

    // Build decoder
    let params    = ctx.stream(video_idx).unwrap().parameters();
    let codec_ctx = ffmpeg::codec::context::Context::from_parameters(params)
        .map_err(|e| anyhow::anyhow!("codec context: {e}"))?;
    let mut decoder = codec_ctx.decoder().video()
        .map_err(|e| anyhow::anyhow!("video decoder: {e}"))?;

    // Build scaler: native format → RGB24 at OUT_W x OUT_H
    let mut scaler = scaling::context::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        OUT_W,
        OUT_H,
        scaling::Flags::BILINEAR,
    ).map_err(|e| anyhow::anyhow!("scaler: {e}"))?;

    let mut decoded = Video::empty();
    let mut scaled  = Video::empty();

    for (stream, packet) in ctx.packets() {
        if stream.index() != video_idx { continue; }
        if decoder.send_packet(&packet).is_err() { continue; }

        while decoder.receive_frame(&mut decoded).is_ok() {
            scaler.run(&decoded, &mut scaled)
                .map_err(|e| anyhow::anyhow!("scaler run: {e}"))?;
            return encode_jpeg(&scaled);
        }
    }

    // Flush decoder — catches last frame in some codecs
    decoder.send_eof().ok();
    while decoder.receive_frame(&mut decoded).is_ok() {
        scaler.run(&decoded, &mut scaled)
            .map_err(|e| anyhow::anyhow!("scaler run (flush): {e}"))?;
        return encode_jpeg(&scaled);
    }

    anyhow::bail!(
        "no frame found at {timestamp:.2}s in {}",
        path.display()
    )
}

// ── encode_jpeg ───────────────────────────────────────────────────────────────

/// Encode a scaled RGB24 Video frame as JPEG bytes in memory.
/// Handles FFmpeg row stride/padding correctly.
fn encode_jpeg(scaled: &Video) -> anyhow::Result<Vec<u8>> {
    let rgb    = scaled.data(0);
    let stride = scaled.stride(0);
    let w      = OUT_W as usize;
    let h      = OUT_H as usize;

    // FFmpeg may pad rows — copy only actual pixel data per row
    let mut pixels = Vec::with_capacity(w * h * 3);
    for row in 0..h {
        let start = row * stride;
        let end   = start + w * 3;
        pixels.extend_from_slice(&rgb[start..end]);
    }

    let img = image::RgbImage::from_raw(OUT_W, OUT_H, pixels)
        .ok_or_else(|| anyhow::anyhow!("failed to create RgbImage from frame"))?;

    let mut jpeg_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut jpeg_bytes), ImageFormat::Jpeg)
        .map_err(|e| anyhow::anyhow!("JPEG encode failed: {e}"))?;

    Ok(jpeg_bytes)
}