"use client";

import { useEffect, useState } from "react";

/** Public-domain style sample; MDN hosts CC0 WebM used in docs examples. */
const SAMPLE_WEBM =
  "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.webm";
const SAMPLE_MP4_FALLBACK =
  "https://storage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4";

/**
 * Vision block: real muted sample video + layered motion (scan → graph → auto tool).
 * Falls back to a still frame if streams fail (network / adblock).
 */
export function LandingFutureVisual() {
  const [reduceMotion, setReduceMotion] = useState(false);
  const [videoOk, setVideoOk] = useState(true);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReduceMotion(mq.matches);
    const on = () => setReduceMotion(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);

  return (
    <div className="grid gap-10 lg:grid-cols-2 lg:items-stretch lg:gap-12">
      <style>{`
        @keyframes future-scan {
          0% { transform: translateY(-8%); opacity: 0; }
          8% { opacity: 0.85; }
          92% { opacity: 0.85; }
          100% { transform: translateY(108%); opacity: 0; }
        }
        @keyframes future-graph-in {
          0%, 18% { opacity: 0; transform: scale(0.92); }
          32%, 78% { opacity: 1; transform: scale(1); }
          100% { opacity: 0.35; transform: scale(0.98); }
        }
        @keyframes future-tool {
          0%, 38% { stroke-dashoffset: 1; opacity: 0.15; }
          48%, 86% { stroke-dashoffset: 0; opacity: 1; }
          100% { stroke-dashoffset: 0; opacity: 0.35; }
        }
        .future-tool-path {
          stroke-dashoffset: 1;
        }
        .future-tool-path--run {
          animation: future-tool 4.8s ease-in-out infinite;
        }
        @keyframes future-film {
          0%, 100% { opacity: 0.45; }
          50% { opacity: 0.75; }
        }
        @keyframes future-image-sweep {
          0% { clip-path: inset(0 100% 0 0); }
          45% { clip-path: inset(0 0 0 0); }
          100% { clip-path: inset(0 0 0 0); }
        }
        @keyframes future-glow {
          0%, 100% { box-shadow: 0 0 0 0 rgba(56, 189, 248, 0); }
          50% { box-shadow: 0 0 40px -8px rgba(56, 189, 248, 0.25); }
        }
      `}</style>

      {/* Video → graph → autonomous tool */}
      <div className="flex flex-col gap-4">
        <p className="text-[11px] font-medium uppercase tracking-[0.14em] text-sky-400/65">Video → graph → edits</p>
        <div className="relative aspect-video w-full overflow-hidden rounded-2xl border border-sky-500/15 bg-slate-950 ring-1 ring-sky-500/10 sm:rounded-3xl">
          {videoOk ? (
            <video
              className="absolute inset-0 h-full w-full object-cover opacity-[0.88]"
              autoPlay
              muted
              loop
              playsInline
              preload="metadata"
              onError={() => setVideoOk(false)}
              aria-label="Sample video loop illustrating future video ingest"
            >
              <source src={SAMPLE_WEBM} type="video/webm" />
              <source src={SAMPLE_MP4_FALLBACK} type="video/mp4" />
            </video>
          ) : (
            <div
              className="absolute inset-0 bg-[linear-gradient(135deg,#18181b_0%,#0c4a6e_45%,#18181b_100%)]"
              aria-hidden
            />
          )}
          <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/80 via-black/25 to-black/50" aria-hidden />

          {/* Filmstrip hint */}
          <div
            className="pointer-events-none absolute left-4 top-4 flex gap-1.5 rounded-lg border border-white/10 bg-black/40 px-2 py-1.5 backdrop-blur-sm"
            style={{ animation: reduceMotion ? "none" : "future-film 3.2s ease-in-out infinite" }}
            aria-hidden
          >
            {[0, 1, 2, 3].map((i) => (
              <span key={i} className="h-7 w-5 rounded-sm bg-white/15 ring-1 ring-white/10" />
            ))}
          </div>

          {/* Scan line */}
          {!reduceMotion && (
            <div
              className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-sky-400/90 to-transparent shadow-[0_0_24px_rgba(56,189,248,0.45)]"
              style={{ animation: "future-scan 4.8s ease-in-out infinite" }}
              aria-hidden
            />
          )}

          {/* Graph overlay */}
          <div
            className="pointer-events-none absolute inset-0 flex items-center justify-center p-6"
            style={{ animation: reduceMotion ? "none" : "future-graph-in 4.8s ease-in-out infinite" }}
            aria-hidden
          >
            <svg viewBox="0 0 200 120" className="h-[min(42%,180px)] w-[min(88%,320px)] text-sky-300/90" fill="none">
              <circle cx="100" cy="28" r="7" className="fill-white/90" opacity="0.95" />
              <circle cx="58" cy="78" r="6" className="fill-white/75" />
              <circle cx="100" cy="92" r="6" className="fill-white/75" />
              <circle cx="142" cy="78" r="6" className="fill-white/75" />
              <path d="M100 35 L58 72 M100 35 L100 86 M100 35 L142 72" stroke="currentColor" strokeWidth="1.2" opacity="0.5" />
              <path d="M58 78 L100 92 L142 78" stroke="currentColor" strokeWidth="1" opacity="0.35" />
            </svg>
          </div>

          {/* Autonomous tool path */}
          <svg
            className="pointer-events-none absolute bottom-6 right-6 h-20 w-32 text-sky-400/95 sm:h-24 sm:w-40"
            viewBox="0 0 120 72"
            fill="none"
            aria-hidden
          >
            <path
              className={`future-tool-path ${reduceMotion ? "" : "future-tool-path--run"}`}
              d="M8 56 C 32 12, 72 8, 108 40"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              pathLength={1}
              strokeDasharray={1}
              strokeDashoffset={reduceMotion ? 0 : undefined}
            />
            <circle cx="8" cy="56" r="4" className="fill-sky-300/90" />
            <circle cx="108" cy="40" r="5" className="fill-sky-200" />
          </svg>

          <p className="pointer-events-none absolute bottom-3 left-4 max-w-[70%] text-[10px] font-medium leading-snug text-slate-400">
            Illustrative sample media · production would run scene detection, ASR, and policy-gated tools on your graph.
          </p>
        </div>
      </div>

      {/* Images → graph → auto adjustments */}
      <div className="flex flex-col gap-4">
        <p className="text-[11px] font-medium uppercase tracking-[0.14em] text-sky-400/65">Images → graph → auto edit</p>
        <div className="relative flex min-h-[220px] flex-1 flex-col justify-between overflow-hidden rounded-2xl border border-sky-500/12 bg-[#080c16] p-5 ring-1 ring-sky-500/8 sm:min-h-0 sm:rounded-3xl sm:p-6">
          <div className="flex gap-4">
            <div
              className="relative h-28 flex-1 overflow-hidden rounded-xl bg-gradient-to-br from-blue-950/60 via-slate-950 to-slate-950 ring-1 ring-sky-500/15"
              style={{ animation: reduceMotion ? "none" : "future-glow 3.6s ease-in-out infinite" }}
              aria-hidden
            >
              <div className="absolute inset-2 rounded-lg bg-[url('data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 width=%2280%22 height=%2280%22%3E%3Cfilter id=%22n%22%3E%3CfeTurbulence type=%22fractalNoise%22 baseFrequency=%220.9%22 numOctaves=%222%22/%3E%3C/filter%3E%3Crect width=%2280%22 height=%2280%22 filter=%22url(%23n)%22 opacity=%220.15%22/%3E%3C/svg%3E')] opacity-60" />
              <span className="absolute bottom-2 left-2 rounded bg-black/50 px-1.5 py-0.5 font-mono text-[9px] text-slate-400">
                RAW
              </span>
            </div>
            <div
              className="relative h-28 flex-1 overflow-hidden rounded-xl bg-gradient-to-br from-sky-950/50 via-slate-950 to-blue-950/40 ring-1 ring-sky-400/25"
              aria-hidden
            >
              <div
                className="absolute inset-0 bg-gradient-to-tr from-sky-500/25 via-transparent to-blue-500/15"
                style={{ animation: reduceMotion ? "none" : "future-image-sweep 4.2s ease-in-out infinite" }}
              />
              <div className="absolute inset-2 rounded-lg border border-sky-500/15 bg-sky-500/[0.04]" />
              <span className="absolute bottom-2 left-2 rounded bg-black/50 px-1.5 py-0.5 font-mono text-[9px] text-sky-300/95">
                AUTO
              </span>
            </div>
          </div>
          <div className="mt-5 flex items-center justify-center gap-2">
            {[0, 1, 2, 3, 4].map((i) => (
              <span
                key={i}
                className="h-2 w-2 rounded-full bg-white/25"
                style={
                  reduceMotion
                    ? undefined
                    : {
                        animation: `future-film ${2.4 + i * 0.12}s ease-in-out infinite`,
                        animationDelay: `${i * 0.1}s`,
                      }
                }
              />
            ))}
            <span className="ml-2 font-mono text-[10px] text-slate-500">graph nodes ← variants</span>
          </div>
          <p className="mt-3 text-[12px] leading-relaxed text-slate-500">
            Same idea as documents: regions, layers, and edits become typed nodes so agents can batch-change assets without
            you clicking every frame.
          </p>
        </div>
      </div>
    </div>
  );
}
