"use client";

import { useCallback, useRef, useState, useSyncExternalStore } from "react";

function subscribeReducedMotion(cb: () => void) {
  const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
  mq.addEventListener("change", cb);
  return () => mq.removeEventListener("change", cb);
}

function reducedMotionSnapshot() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function reducedMotionServerSnapshot() {
  return false;
}

type TwinCardAccent = "sky" | "violet" | "cyan";

type TwinCardDef = {
  eyebrow: string;
  title: string;
  body: string;
  accent: TwinCardAccent;
  decor: "nodes" | "pulse" | "shard";
};

const cards: TwinCardDef[] = [
  {
    eyebrow: "01 · You, encoded",
    title: "Built from your real context",
    body:
      "The twin learns from what you already have: mail, repositories, PDFs, and notes. It doesn’t improvise a personality from thin air—it cites the graph of your life and work.",
    accent: "sky",
    decor: "nodes",
  },
  {
    eyebrow: "02 · Presence",
    title: "A card that is you in the room",
    body:
      "Most people start with a wallet pass in Apple Wallet or Google Wallet (next to Apple Pay and Google Pay)—no ship, no cart. Add NFC or other formats later if you want a physical tap. Same mind as your private workspace.",
    accent: "violet",
    decor: "pulse",
  },
  {
    eyebrow: "03 · Boundaries",
    title: "Your lines, not the model’s",
    body:
      "You decide what it can quote, suggest, or automate. Guardrails and policies live in the engine so your twin stays professional, on-brand, and safe to share.",
    accent: "cyan",
    decor: "shard",
  },
];

const accentRing: Record<TwinCardAccent, string> = {
  sky: "from-sky-400/50 via-sky-300/15 to-transparent",
  violet: "from-violet-400/55 via-violet-300/12 to-transparent",
  cyan: "from-cyan-400/50 via-cyan-300/12 to-transparent",
};

const accentGlow: Record<TwinCardAccent, string> = {
  sky: "shadow-[0_0_52px_-12px_rgba(56,189,248,0.35)]",
  violet: "shadow-[0_0_52px_-12px_rgba(167,139,250,0.32)]",
  cyan: "shadow-[0_0_52px_-12px_rgba(34,211,238,0.3)]",
};

const accentChip: Record<TwinCardAccent, string> = {
  sky: "border-sky-400/25 bg-sky-500/[0.12] text-sky-100/95",
  violet: "border-violet-400/25 bg-violet-500/[0.12] text-violet-100/95",
  cyan: "border-cyan-400/25 bg-cyan-500/[0.1] text-cyan-50/95",
};

function TwinCardDecor({ kind, accent }: { kind: TwinCardDef["decor"]; accent: TwinCardAccent }) {
  const stroke =
    accent === "sky"
      ? "stroke-sky-400/35"
      : accent === "violet"
        ? "stroke-violet-400/35"
        : "stroke-cyan-400/35";

  if (kind === "nodes") {
    const n1 = accent === "sky" ? "text-sky-400/85" : accent === "violet" ? "text-violet-400/85" : "text-cyan-400/85";
    const n2 = accent === "sky" ? "text-sky-300/55" : accent === "violet" ? "text-violet-300/55" : "text-cyan-300/55";
    const n3 = accent === "sky" ? "text-blue-300/55" : accent === "violet" ? "text-fuchsia-300/55" : "text-teal-300/55";
    return (
      <svg viewBox="0 0 120 72" className={`h-[4.25rem] w-auto ${stroke}`} fill="none" aria-hidden>
        <circle cx="24" cy="36" r="5" className={`fill-current ${n1}`} strokeWidth="1" stroke="currentColor" opacity="0.5" />
        <circle cx="60" cy="18" r="4" className={`fill-current ${n2}`} strokeWidth="1" stroke="currentColor" opacity="0.38" />
        <circle cx="96" cy="44" r="5" className={`fill-current ${n3}`} strokeWidth="1" stroke="currentColor" opacity="0.42" />
        <path d="M29 36 L56 22 M64 22 L91 42 M29 40 L56 50 M64 50 L91 46" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" opacity="0.85" />
      </svg>
    );
  }
  if (kind === "pulse") {
    const core =
      accent === "sky" ? "text-sky-300/90" : accent === "violet" ? "text-violet-300/90" : "text-cyan-300/90";
    return (
      <svg viewBox="0 0 120 72" className={`h-[4.25rem] w-auto ${stroke}`} fill="none" aria-hidden>
        <path
          d="M12 36 H38 M82 36 H108 M60 12 V24 M60 48 V60"
          stroke="currentColor"
          strokeWidth="1.05"
          strokeLinecap="round"
          opacity="0.45"
        />
        <circle cx="60" cy="36" r="18" className="fill-none" stroke="currentColor" strokeWidth="1" opacity="0.35" />
        <circle cx="60" cy="36" r="10" className="fill-none" stroke="currentColor" strokeWidth="1.2" opacity="0.65" />
        <circle cx="60" cy="36" r="3" className={`fill-current ${core}`} opacity="0.9" />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 120 72" className={`h-[4.25rem] w-auto ${stroke}`} fill="none" aria-hidden>
      <path d="M18 52 L44 16 L98 22 L72 58 Z" className="fill-current text-cyan-400/10" stroke="currentColor" strokeWidth="1.05" />
      <path d="M34 48 L52 28 L86 32 L68 52 Z" className="fill-current text-cyan-300/8" stroke="currentColor" strokeWidth="0.95" opacity="0.85" />
      <path d="M60 24 L60 48" stroke="currentColor" strokeWidth="1" strokeLinecap="round" opacity="0.5" />
    </svg>
  );
}

function DigitalTwinCard({ card, reduceMotion }: { card: TwinCardDef; reduceMotion: boolean }) {
  const root = useRef<HTMLDivElement>(null);
  const [tilt, setTilt] = useState("perspective(1100px) rotateX(0deg) rotateY(0deg) translateZ(0px)");
  const [glare, setGlare] = useState({ x: 50, y: 40, o: 0 });

  const onMove = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (reduceMotion) return;
      const el = root.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      const x = e.clientX - r.left;
      const y = e.clientY - r.top;
      const px = x / r.width - 0.5;
      const py = y / r.height - 0.5;
      const maxX = 11;
      const maxY = 13;
      const rx = (-py * maxY).toFixed(2);
      const ry = (px * maxX).toFixed(2);
      setTilt(`perspective(1100px) rotateX(${rx}deg) rotateY(${ry}deg) translateZ(10px)`);
      const gx = Math.min(100, Math.max(0, (x / r.width) * 100));
      const gy = Math.min(100, Math.max(0, (y / r.height) * 100));
      setGlare({ x: gx, y: gy, o: 0.55 });
    },
    [reduceMotion],
  );

  const onLeave = useCallback(() => {
    setTilt("perspective(1100px) rotateX(0deg) rotateY(0deg) translateZ(0px)");
    setGlare((g) => ({ ...g, o: 0 }));
  }, []);

  const transition = reduceMotion ? undefined : "transform 160ms ease-out, box-shadow 220ms ease";

  return (
    <div
      ref={root}
      className="relative min-h-[220px] [transform-style:preserve-3d] sm:min-h-[260px]"
      onMouseMove={onMove}
      onMouseLeave={onLeave}
      style={{ perspective: "1100px" }}
    >
      {/* Depth stack: faint back planes */}
      <div
        className="pointer-events-none absolute inset-0 -z-10 translate-x-2 translate-y-3 scale-[0.98] rounded-[1.35rem] border border-white/[0.04] bg-slate-950/30 opacity-60 blur-[0.5px] sm:rounded-3xl"
        style={{ transform: "translateZ(-18px)" }}
        aria-hidden
      />
      <div
        className="pointer-events-none absolute inset-0 -z-10 translate-x-4 translate-y-5 scale-[0.965] rounded-[1.35rem] border border-sky-500/[0.05] bg-slate-950/20 opacity-40 sm:rounded-3xl"
        style={{ transform: "translateZ(-34px)" }}
        aria-hidden
      />

      <article
        className={[
          "relative flex h-full flex-col overflow-hidden rounded-[1.35rem] border border-white/[0.08] bg-gradient-to-br from-slate-950/90 via-slate-950/70 to-slate-950/40 p-[1px] shadow-[inset_0_1px_0_rgba(255,255,255,0.06)] sm:rounded-3xl",
          accentGlow[card.accent],
        ].join(" ")}
        style={{ transform: tilt, transition }}
      >
        <div className="pointer-events-none absolute inset-0 rounded-[1.34rem] bg-[radial-gradient(ellipse_80%_55%_at_50%_-10%,rgba(255,255,255,0.07),transparent_58%)] sm:rounded-[calc(1.5rem-1px)]" />
        <div
          className="pointer-events-none absolute inset-0 transition-opacity duration-200"
          style={{
            opacity: reduceMotion ? 0 : glare.o,
            background: `radial-gradient(520px circle at ${glare.x}% ${glare.y}%, rgba(255,255,255,0.16), transparent 55%)`,
          }}
        />

        <div className="relative flex h-full flex-col rounded-[1.3rem] border border-white/[0.06] bg-slate-950/55 p-5 backdrop-blur-[2px] sm:rounded-[calc(1.5rem-2px)] sm:p-7">
          <div
            className={`pointer-events-none absolute -right-16 -top-20 h-48 w-48 rounded-full bg-gradient-to-br ${accentRing[card.accent]} blur-3xl`}
            aria-hidden
          />

          <div className="flex items-start justify-between gap-4">
            <div>
              <p className="text-[10px] font-medium uppercase tracking-[0.22em] text-slate-500 sm:text-[11px]">{card.eyebrow}</p>
              <h3 className="mt-2 text-lg font-medium tracking-[-0.03em] text-white sm:text-xl">{card.title}</h3>
            </div>
            <span
              className={[
                "inline-flex shrink-0 rounded-full border px-2.5 py-1 text-[10px] font-medium tracking-wide sm:px-3 sm:text-[11px]",
                accentChip[card.accent],
              ].join(" ")}
            >
              Your twin
            </span>
          </div>

          <div className="relative mt-5 flex flex-1 flex-col gap-4 sm:mt-6">
            <div className="flex items-center justify-between gap-3">
              <TwinCardDecor kind={card.decor} accent={card.accent} />
              <div className="hidden h-px flex-1 bg-gradient-to-r from-white/10 via-white/5 to-transparent sm:block" aria-hidden />
            </div>
            <p className="max-w-prose text-[13px] leading-[1.65] text-slate-400 sm:text-[14px]">{card.body}</p>
          </div>

          <div
            className="pointer-events-none absolute bottom-4 right-5 h-10 w-24 rounded-full bg-gradient-to-r from-white/[0.04] to-transparent opacity-70"
            style={{ transform: "translateZ(24px)" }}
            aria-hidden
          />
        </div>
      </article>
    </div>
  );
}

export function LandingDigitalTwinCards() {
  const reduceMotion = useSyncExternalStore(
    subscribeReducedMotion,
    reducedMotionSnapshot,
    reducedMotionServerSnapshot,
  );

  return (
    <section
      id="digital-twins"
      className="scroll-mt-24 border-t border-sky-500/[0.08] bg-gradient-to-b from-[#070a12] via-slate-950/40 to-[#070a12] py-16 sm:py-24"
    >
      <div className="mx-auto max-w-6xl px-5 sm:px-8">
        <div className="max-w-2xl">
          <h2 className="text-[11px] font-medium uppercase tracking-[0.16em] text-sky-400/70">Digital twin of you</h2>
          <p className="mt-3 text-2xl font-medium tracking-[-0.03em] text-white sm:text-[1.75rem] sm:leading-snug">
            Three pillars of how yours comes to life.
          </p>
          <p className="mt-3 max-w-xl text-[14px] leading-relaxed text-slate-500 sm:text-[15px]">
            Teach it from your sources, share via wallet first or add NFC when you prefer, and keep control of what “you”
            sounds like—all on the same grounded graph as your workspace.
          </p>
        </div>

        <div className="relative mt-10 sm:mt-14">
          <div
            className="pointer-events-none absolute left-1/2 top-1/2 h-[min(92vw,640px)] w-[min(92vw,640px)] -translate-x-1/2 -translate-y-1/2 rounded-full bg-[radial-gradient(circle_at_center,rgba(56,189,248,0.07),transparent_62%)] blur-2xl"
            aria-hidden
          />

          <div
            className="relative grid gap-6 lg:grid-cols-3 lg:gap-7"
            style={{ perspective: "1400px", transformStyle: "preserve-3d" }}
          >
            <div
              className={[
                "origin-[72%_45%] will-change-transform",
                reduceMotion ? "" : "lg:[transform:rotateY(6deg)] lg:transition-transform lg:duration-300 lg:hover:-translate-y-1",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              <DigitalTwinCard card={cards[0]} reduceMotion={reduceMotion} />
            </div>
            <div
              className={[
                "origin-center will-change-transform",
                reduceMotion ? "" : "lg:-mt-1 lg:mb-1 lg:transition-transform lg:duration-300 lg:hover:-translate-y-1",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              <DigitalTwinCard card={cards[1]} reduceMotion={reduceMotion} />
            </div>
            <div
              className={[
                "origin-[28%_45%] will-change-transform",
                reduceMotion ? "" : "lg:[transform:rotateY(-6deg)] lg:transition-transform lg:duration-300 lg:hover:-translate-y-1",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              <DigitalTwinCard card={cards[2]} reduceMotion={reduceMotion} />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
