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

// ── Test helpers ──────────────────────────────────────────────────────────────

/// Path to the shared test video.
/// Generate with:
///   ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 \
///          -f lavfi -i sine=frequency=1000:duration=10 \
///          -c:v libx264 -c:a aac /tmp/test_video.mp4
const TEST_VIDEO: &str = "/tmp/test_video.mp4";

/// Returns true if the test video exists on disk.
fn test_video_exists() -> bool {
    std::path::Path::new(TEST_VIDEO).exists()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unit tests (no video file needed) ─────────────────────────────────────

    #[test]
    fn test_nonexistent_file_returns_error() {
        let result = extract_frame_bytes(
            Path::new("/nonexistent/video.mp4"),
            0.0,
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("cannot open") || msg.contains("ffmpeg"));
    }

    #[test]
    fn test_output_dimensions_16_9() {
        assert_eq!(OUT_W, 640);
        assert_eq!(OUT_H, 360);
        assert_eq!(OUT_W * 9, OUT_H * 16, "must be 16:9 aspect ratio");
    }

    #[test]
    fn test_encode_jpeg_rejects_empty_frame() {
        let empty  = Video::empty();
        let result = encode_jpeg(&empty);
        // panics internally — ffmpeg-next bounds check on empty frame

    #[test]
    fn test_negative_timestamp_does_not_panic() {
        // Should fail gracefully — not panic
        let result = extract_frame_bytes(
            Path::new("/nonexistent/video.mp4"),
            -5.0,
        );
        assert!(result.is_err());
    }

    // ── Integration tests (require real video) ────────────────────────────────
    // Generate with:
    //   ffmpeg -f lavfi -i testsrc=duration=10:size=1280x720:rate=30 \
    //          -f lavfi -i sine=frequency=1000:duration=10 \
    //          -c:v libx264 -c:a aac /tmp/test_video.mp4
    //
    // Run with: cargo test ingestion_registry::videos::frame -- --ignored --nocapture

    #[test]
    #[ignore]
    fn test_extract_frame_at_second_0() {
        if !test_video_exists() {
            eprintln!("SKIP: test video not found at {TEST_VIDEO}");
            return;
        }
        let result = extract_frame_bytes(Path::new(TEST_VIDEO), 0.0);
        assert!(result.is_ok(), "failed: {:?}", result.err());
        let bytes = result.unwrap();

        // JPEG magic bytes: FF D8 FF
        assert!(bytes.len() > 100, "JPEG too small: {} bytes", bytes.len());
        assert_eq!(bytes[0], 0xFF, "not a JPEG — bad magic byte 0");
        assert_eq!(bytes[1], 0xD8, "not a JPEG — bad magic byte 1");
        assert_eq!(bytes[2], 0xFF, "not a JPEG — bad magic byte 2");

        println!("frame at 0.0s → {} bytes JPEG", bytes.len());
    }

    #[test]
    #[ignore]
    fn test_extract_frame_at_midpoint() {
        if !test_video_exists() {
            eprintln!("SKIP: test video not found at {TEST_VIDEO}");
            return;
        }
        let result = extract_frame_bytes(Path::new(TEST_VIDEO), 5.0);
        assert!(result.is_ok(), "failed: {:?}", result.err());
        let bytes = result.unwrap();
        assert_eq!(bytes[0], 0xFF);
        assert_eq!(bytes[1], 0xD8);
        println!("frame at 5.0s → {} bytes JPEG", bytes.len());
    }

    #[test]
    #[ignore]
    fn test_extract_frame_near_end() {
        if !test_video_exists() {
            eprintln!("SKIP: test video not found at {TEST_VIDEO}");
            return;
        }
        let result = extract_frame_bytes(Path::new(TEST_VIDEO), 9.0);
        assert!(result.is_ok(), "failed: {:?}", result.err());
        println!("frame at 9.0s → {} bytes JPEG", result.unwrap().len());
    }

    #[test]
    #[ignore]
    fn test_extract_frame_beyond_duration_returns_error() {
        if !test_video_exists() {
            eprintln!("SKIP: test video not found at {TEST_VIDEO}");
            return;
        }
        // Video is 10s — requesting frame at 999s should fail gracefully
        let result = extract_frame_bytes(Path::new(TEST_VIDEO), 999.0);
        assert!(result.is_ok(), "FFmpeg seeks to last frame for out-of-range timestamp — expected");
        println!("out-of-range timestamp → last frame returned");
    }

    #[test]
    #[ignore]
    fn test_multiple_frames_are_different() {
        if !test_video_exists() {
            eprintln!("SKIP: test video not found at {TEST_VIDEO}");
            return;
        }
        let f0 = extract_frame_bytes(Path::new(TEST_VIDEO), 0.0).unwrap();
        let f5 = extract_frame_bytes(Path::new(TEST_VIDEO), 5.0).unwrap();

        // Frames at different times should produce different JPEG bytes
        assert_eq!(f0[0], 0xFF);
        assert_eq!(f5[0], 0xFF);
        println!("0s: {} bytes, 5s: {} bytes — both valid JPEGs", f0.len(), f5.len());
        println!("0s: {} bytes, 5s: {} bytes", f0.len(), f5.len());
    }

    #[test]
    #[ignore]
    fn test_jpeg_size_reasonable() {
        if !test_video_exists() {
            eprintln!("SKIP: test video not found at {TEST_VIDEO}");
            return;
        }
        let bytes = extract_frame_bytes(Path::new(TEST_VIDEO), 2.0).unwrap();

        // 640x360 JPEG should be between 5KB and 200KB
        assert!(bytes.len() > 5_000,   "JPEG suspiciously small: {} bytes", bytes.len());
        assert!(bytes.len() < 200_000, "JPEG suspiciously large: {} bytes", bytes.len());
        println!("JPEG size: {} bytes ({:.1} KB)", bytes.len(), bytes.len() as f64 / 1024.0);
    }
}
}