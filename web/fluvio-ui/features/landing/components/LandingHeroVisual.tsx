"use client";

import { useEffect, useState } from "react";

/**
 * Product-style hero visual: SVG pipeline + soft ambient motion.
 * Easier to ship at “premium marketing” quality than ad-hoc Three.js without a dedicated art pass.
 */
export function LandingHeroVisual() {
  const [reduceMotion, setReduceMotion] = useState(false);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReduceMotion(mq.matches);
    const on = () => setReduceMotion(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);

  return (
    <div className="relative flex min-h-[280px] w-full flex-col justify-center overflow-hidden sm:min-h-[320px]">
      <style>{`
        @keyframes landing-ambient-1 {
          0%, 100% { transform: translate(0, 0) scale(1); opacity: 0.35; }
          50% { transform: translate(8%, -6%) scale(1.08); opacity: 0.5; }
        }
        @keyframes landing-ambient-2 {
          0%, 100% { transform: translate(0, 0) scale(1); opacity: 0.2; }
          50% { transform: translate(-10%, 8%) scale(1.12); opacity: 0.32; }
        }
        @keyframes landing-float {
          0%, 100% { transform: translateY(0); }
          50% { transform: translateY(-5px); }
        }
      `}</style>

      {/* Ambient layers, editorial, not arcade */}
      <div
        className="pointer-events-none absolute -left-1/4 top-0 h-[140%] w-[70%] rounded-full bg-[radial-gradient(circle_at_30%_30%,rgba(255,255,255,0.07),transparent_55%)] blur-3xl"
        style={{ animation: reduceMotion ? "none" : "landing-ambient-1 14s ease-in-out infinite" }}
        aria-hidden
      />
      <div
        className="pointer-events-none absolute -right-1/4 bottom-0 h-[120%] w-[65%] rounded-full bg-[radial-gradient(circle_at_70%_60%,rgba(255,255,255,0.04),transparent_58%)] blur-3xl"
        style={{ animation: reduceMotion ? "none" : "landing-ambient-2 18s ease-in-out infinite" }}
        aria-hidden
      />

      <div
        className="relative z-[2] mx-auto w-full max-w-lg px-4 py-8 sm:max-w-none sm:px-8 sm:py-10"
        style={{ animation: reduceMotion ? "none" : "landing-float 7s ease-in-out infinite" }}
      >
        <svg
          viewBox="0 0 560 200"
          className="h-auto w-full text-white/55"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
          aria-hidden
        >
          <defs>
            <linearGradient id="landing-line" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stopColor="rgba(255,255,255,0.06)" />
              <stop offset="50%" stopColor="rgba(255,255,255,0.22)" />
              <stop offset="100%" stopColor="rgba(255,255,255,0.06)" />
            </linearGradient>
            <filter id="landing-soft" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="0.8" result="b" />
              <feMerge>
                <feMergeNode in="b" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {/* Connectors */}
          <path
            id="path-a"
            d="M 118 100 L 198 100"
            stroke="url(#landing-line)"
            strokeWidth="1.25"
            strokeLinecap="round"
            pathLength="1"
            strokeDasharray="1"
            strokeDashoffset={reduceMotion ? 0 : 1}
          >
            {!reduceMotion && (
              <animate
                attributeName="stroke-dashoffset"
                values="1;0;0;1"
                keyTimes="0;0.22;0.78;1"
                dur="5.5s"
                repeatCount="indefinite"
              />
            )}
          </path>
          <path
            id="path-b"
            d="M 362 100 L 442 100"
            stroke="url(#landing-line)"
            strokeWidth="1.25"
            strokeLinecap="round"
            pathLength="1"
            strokeDasharray="1"
            strokeDashoffset={reduceMotion ? 0 : 1}
          >
            {!reduceMotion && (
              <animate
                attributeName="stroke-dashoffset"
                values="1;0;0;1"
                keyTimes="0;0.22;0.78;1"
                dur="5.5s"
                begin="0.35s"
                repeatCount="indefinite"
              />
            )}
          </path>

          {/* Stage 1, sources */}
          <g filter="url(#landing-soft)">
            <rect x="28" y="58" width="90" height="84" rx="12" className="fill-white/[0.04]" stroke="rgba(255,255,255,0.14)" strokeWidth="1" />
            <rect x="38" y="68" width="70" height="10" rx="2" className="fill-white/[0.07]" />
            <rect x="38" y="84" width="52" height="8" rx="2" className="fill-white/[0.05]" />
            <rect x="38" y="98" width="62" height="8" rx="2" className="fill-white/[0.05]" />
            <text x="73" y="154" textAnchor="middle" className="fill-white/40" style={{ fontSize: "10px", fontWeight: 600, letterSpacing: "0.1em" }}>
              YOU
            </text>
          </g>

          {/* Stage 2, graph */}
          <g>
            <rect x="208" y="48" width="144" height="104" rx="14" className="fill-white/[0.03]" stroke="rgba(255,255,255,0.12)" strokeWidth="1" />
            {/* nodes */}
            <circle cx="252" cy="88" r="5" className="fill-white/90" />
            <circle cx="288" cy="76" r="4.5" className="fill-white/75" />
            <circle cx="308" cy="102" r="4.5" className="fill-white/65" />
            <circle cx="276" cy="118" r="4" className="fill-white/55" />
            <circle cx="318" cy="124" r="3.5" className="fill-white/45" />
            <line x1="252" y1="88" x2="288" y2="76" stroke="rgba(255,255,255,0.28)" strokeWidth="1" />
            <line x1="288" y1="76" x2="308" y2="102" stroke="rgba(255,255,255,0.22)" strokeWidth="1" />
            <line x1="252" y1="88" x2="276" y2="118" stroke="rgba(255,255,255,0.18)" strokeWidth="1" />
            <line x1="308" y1="102" x2="318" y2="124" stroke="rgba(255,255,255,0.2)" strokeWidth="1" />
            <line x1="276" y1="118" x2="318" y2="124" stroke="rgba(255,255,255,0.14)" strokeWidth="1" />
            <text x="280" y="168" textAnchor="middle" className="fill-white/40" style={{ fontSize: "10px", fontWeight: 600, letterSpacing: "0.1em" }}>
              TWIN
            </text>
          </g>

          {/* Stage 3, reason */}
          <g filter="url(#landing-soft)">
            <rect x="452" y="58" width="90" height="84" rx="12" className="fill-white/[0.04]" stroke="rgba(255,255,255,0.12)" strokeWidth="1" />
            <rect x="466" y="74" width="62" height="5" rx="1.5" className="fill-white/[0.12]" />
            <rect x="466" y="86" width="54" height="5" rx="1.5" className="fill-white/[0.08]" />
            <rect x="466" y="98" width="48" height="5" rx="1.5" className="fill-white/[0.06]" />
            <rect x="466" y="114" width="58" height="16" rx="4" className="fill-white/[0.06]" stroke="rgba(255,255,255,0.08)" strokeWidth="0.75" />
            <text x="497" y="154" textAnchor="middle" className="fill-white/40" style={{ fontSize: "10px", fontWeight: 600, letterSpacing: "0.1em" }}>
              SHARE
            </text>
          </g>
        </svg>
      </div>

      <p className="relative z-[2] px-6 pb-5 text-center text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-600 sm:px-8">
        You → twin → Wallet or tap
      </p>
    </div>
  );
}
