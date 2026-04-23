"use client";

import { useMemo } from "react";
import type { BrainTab } from "@/lib/types";

type Props = {
  brainTab: BrainTab;
  className?: string;
};

/** Decorative mock drawing — not tied to live BIM; pairs with the knowledge graph in Design brain. */
export function DesignBlueprintMock({ brainTab, className = "" }: Props) {
  const { accent, subtitle } = useMemo(() => {
    switch (brainTab) {
      case "des_bim":
        return { accent: "#38bdf8", subtitle: "Federation + clash context (mock)" };
      case "des_arch_plans":
        return { accent: "#c084fc", subtitle: "Sheets + program (mock)" };
      case "des_structural":
        return { accent: "#f472b6", subtitle: "Members + drift envelope (mock)" };
      case "des_civil_site":
        return { accent: "#34d399", subtitle: "Site + utilities (mock)" };
      case "des_building_codes":
        return { accent: "#fbbf24", subtitle: "Loads + adopted edition (mock)" };
      case "des_physics_sim":
        return { accent: "#fb7185", subtitle: "Solver gates (mock)" };
      case "unified":
        return { accent: "#a5b4fc", subtitle: "Fused discipline view (mock)" };
      case "meta":
        return { accent: "#94a3b8", subtitle: "Control plane (mock)" };
      default:
        return { accent: "#64748b", subtitle: "Design slice (mock)" };
    }
  }, [brainTab]);

  const showBracing = brainTab === "des_structural" || brainTab === "unified";
  const showSite = brainTab === "des_civil_site" || brainTab === "unified";
  const showPhysics = brainTab === "des_physics_sim" || brainTab === "unified";
  const showClash = brainTab === "des_bim" || brainTab === "unified";

  return (
    <aside
      className={`relative flex flex-col border-t border-white/[0.08] bg-gradient-to-b from-zinc-950/90 to-[#0a0c14] lg:h-full lg:min-h-0 lg:w-[min(100%,380px)] lg:shrink-0 lg:border-l lg:border-t-0 ${className}`}
      aria-hidden
    >
      <div className="shrink-0 border-b border-white/[0.06] px-3 py-2.5 lg:px-4">
        <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600">Mock drawing</p>
        <p className="mt-0.5 text-[13px] font-medium tracking-tight text-zinc-200">Plan + axon sketch</p>
        <p className="mt-1 text-[11px] leading-snug text-zinc-500">{subtitle}</p>
      </div>

      <div className="relative min-h-[200px] flex-1 overflow-hidden px-2 py-3 sm:min-h-[240px] lg:min-h-0">
        <svg
          viewBox="0 0 320 360"
          className="mx-auto h-full max-h-[min(52vh,420px)] w-full max-w-[320px] drop-shadow-[0_0_24px_rgba(15,23,42,0.85)]"
          preserveAspectRatio="xMidYMid meet"
        >
          <defs>
            <style>{`
              @keyframes bpScan {
                0% { stroke-opacity: 0.25; }
                50% { stroke-opacity: 0.55; }
                100% { stroke-opacity: 0.25; }
              }
              @keyframes bpDrift {
                0%, 100% { transform: translateX(0); }
                50% { transform: translateX(2px); }
              }
              .bp-grid { animation: bpScan 3.2s ease-in-out infinite; }
              .bp-site { animation: bpScan 4s ease-in-out infinite; }
              .bp-arrow { animation: bpDrift 2.4s ease-in-out infinite; }
            `}</style>
            <linearGradient id="bpFloor" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0%" stopColor="#18181b" stopOpacity="0.95" />
              <stop offset="100%" stopColor="#0f172a" stopOpacity="0.98" />
            </linearGradient>
            <filter id="bpGlow" x="-20%" y="-20%" width="140%" height="140%">
              <feGaussianBlur stdDeviation="1.2" result="b" />
              <feMerge>
                <feMergeNode in="b" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {/* Axon "ground" */}
          <path
            d="M20 300 L160 340 L300 300 L160 260 Z"
            fill="#0c1222"
            stroke="#27272a"
            strokeWidth="1"
          />

          {showSite && (
            <path
              className="bp-site"
              d="M24 298 Q80 286 120 292 T200 288 T296 298"
              fill="none"
              stroke="#34d399"
              strokeWidth="1.2"
              strokeOpacity="0.5"
            />
          )}

          {/* Axon building mass */}
          <g filter="url(#bpGlow)">
            <path
              d="M100 120 L220 100 L240 200 L120 220 Z"
              fill="url(#bpFloor)"
              stroke={accent}
              strokeWidth="1.4"
              strokeOpacity="0.85"
            />
            <path
              d="M100 120 L120 40 L240 24 L220 100 Z"
              fill="#1e293b"
              stroke={accent}
              strokeWidth="1.2"
              strokeOpacity="0.55"
            />
            <path
              d="M220 100 L240 24 L260 120 L240 200 Z"
              fill="#151b2e"
              stroke="#334155"
              strokeWidth="1"
            />
          </g>

          {/* Floor plan (projected) */}
          <g transform="translate(118 148) scale(0.52)">
            <rect x="0" y="0" width="180" height="120" rx="4" fill="#09090b" stroke={accent} strokeWidth="2" opacity="0.9" />
            <g className="bp-grid" stroke="#3f3f46" strokeWidth="0.6">
              {[0, 30, 60, 90, 120, 150].map((x) => (
                <line key={`v${x}`} x1={x} y1={0} x2={x} y2={120} />
              ))}
              {[0, 40, 80, 120].map((y) => (
                <line key={`h${y}`} x1={0} y1={y} x2={180} y2={y} />
              ))}
            </g>
            <rect x="8" y="8" width="72" height="48" rx="2" fill="none" stroke="#71717a" strokeWidth="1.2" />
            <rect x="88" y="10" width="82" height="44" rx="2" fill="none" stroke="#71717a" strokeWidth="1.2" />
            <rect x="8" y="64" width="162" height="48" rx="2" fill="none" stroke="#52525b" strokeWidth="1" />
            <text x="14" y="32" fill="#a1a1aa" fontSize="11" fontFamily="ui-monospace, monospace">
              LAB
            </text>
            <text x="94" y="34" fill="#a1a1aa" fontSize="11" fontFamily="ui-monospace, monospace">
              CORE
            </text>
            {[[24, 88], [52, 88], [140, 88], [168, 88]].map(([cx, cy], i) => (
              <circle key={i} cx={cx} cy={cy} r="4" fill="#27272a" stroke={accent} strokeWidth="0.8" opacity="0.9" />
            ))}
            {showClash && (
              <g transform="translate(100 52)">
                <line x1="-6" y1="-6" x2="6" y2="6" stroke="#f87171" strokeWidth="1.8" strokeLinecap="round" />
                <line x1="6" y1="-6" x2="-6" y2="6" stroke="#f87171" strokeWidth="1.8" strokeLinecap="round" />
              </g>
            )}
            {showBracing && (
              <g stroke="#22d3ee" strokeWidth="1.2" strokeOpacity="0.65">
                <line x1="8" y1="120" x2="80" y2="8" />
                <line x1="100" y1="120" x2="170" y2="10" />
              </g>
            )}
          </g>

          {showPhysics && (
            <g className="bp-arrow" transform="translate(48 72)">
              <path d="M0 20 L28 4 L22 12 L40 12 L40 28 L22 28 L28 36 Z" fill="#fb7185" fillOpacity="0.35" stroke="#fb7185" strokeWidth="1" />
              <text x="44" y="26" fill="#fda4af" fontSize="9" fontFamily="ui-monospace, monospace">
                wind
              </text>
            </g>
          )}

          {/* North */}
          <g transform="translate(268 72)">
            <line x1="0" y1="20" x2="0" y2="0" stroke="#64748b" strokeWidth="1.2" />
            <polygon points="0,0 -5,10 5,10" fill="#64748b" />
            <text x="-4" y="34" fill="#64748b" fontSize="10" fontWeight="600" fontFamily="system-ui">
              N
            </text>
          </g>

          <text x="16" y="24" fill="#52525b" fontSize="9" fontFamily="ui-monospace, monospace">
            scale N.T.S · preview only
          </text>
        </svg>

        <div className="pointer-events-none absolute inset-x-0 bottom-0 flex justify-center pb-2">
          <span
            className="rounded-full border border-white/[0.08] bg-black/40 px-3 py-1 text-[10px] font-medium text-zinc-500 backdrop-blur-sm"
            style={{ borderColor: `${accent}33` }}
          >
            Graph nodes stay authoritative · drawing is illustrative
          </span>
        </div>
      </div>
    </aside>
  );
}
