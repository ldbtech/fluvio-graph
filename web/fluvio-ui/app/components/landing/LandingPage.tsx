"use client";

import Link from "next/link";
import { WIFI_NFC_PREORDER_ENABLED } from "@/lib/onboardingFlags";
import { LandingHeroVisual } from "./LandingHeroVisual";

const steps = [
  {
    n: 1,
    title: "Say who you are",
    body: "One short setup. FluvioMe remembers the story you want told.",
  },
  {
    n: 2,
    title: "Wallet or tap",
    body: "Apple Wallet on the phone—or a card with NFC. Same link, same you.",
  },
  {
    n: 3,
    title: "They open you",
    body: "No hunting for an app. Lock screen or tap—conversation starts there.",
  },
] as const;

function FluvioMark() {
  return (
    <span
      className="relative inline-flex h-7 w-7 items-center justify-center overflow-hidden rounded-md border border-violet-500/20 bg-violet-500/[0.08]"
      aria-hidden
    >
      <svg viewBox="0 0 24 24" className="relative h-5 w-5 text-violet-200/90" fill="none">
        <path d="M12 3.5 L4.5 8.2 L4.5 15.8 L12 20.5 L19.5 15.8 L19.5 8.2 Z" stroke="currentColor" strokeWidth="1.1" opacity="0.65" />
        <circle cx="12" cy="7.7" r="1.5" className="fill-violet-100" />
        <circle cx="8.3" cy="14.7" r="1.35" className="fill-violet-200/90" />
        <circle cx="15.7" cy="14.7" r="1.35" className="fill-violet-200/80" />
        <path d="M12 9.2 L8.3 13.3 M12 9.2 L15.7 13.3 M8.3 14.7 L15.7 14.7" stroke="currentColor" strokeWidth="1.05" opacity="0.8" />
      </svg>
    </span>
  );
}

export function LandingPage() {
  return (
    <div className="min-h-screen bg-[#09090b] text-zinc-100 antialiased">
      <div
        className="pointer-events-none fixed inset-0 -z-10 bg-[radial-gradient(ellipse_75%_55%_at_50%_-15%,rgba(139,92,246,0.12),transparent_55%)]"
        aria-hidden
      />

      <header className="fixed inset-x-0 top-0 z-50 border-b border-white/[0.06] bg-[#09090b]/75 backdrop-blur-xl">
        <div className="mx-auto flex h-14 max-w-4xl items-center justify-between px-5 sm:h-16 sm:px-6">
          <Link href="/" className="flex items-center gap-2.5 text-white transition hover:opacity-90" aria-label="FluvioMe home">
            <FluvioMark />
            <span className="text-[17px] font-semibold tracking-[-0.03em]">FluvioMe</span>
          </Link>
          <nav className="flex items-center gap-2 sm:gap-4">
            <Link
              href="/chat"
              className="hidden text-[15px] text-zinc-500 transition hover:text-zinc-300 sm:inline"
            >
              Chat
            </Link>
            <Link
              href="/dashboard"
              className="hidden text-[15px] text-zinc-500 transition hover:text-zinc-300 md:inline"
            >
              Overview
            </Link>
            <Link
              href="/onboarding"
              className="rounded-full bg-violet-500 px-4 py-2 text-[14px] font-semibold text-white shadow-[0_0_0_1px_rgba(255,255,255,0.06)_inset] transition hover:bg-violet-400 sm:px-5"
            >
              Get started
            </Link>
          </nav>
        </div>
      </header>

      <main>
        {/* Hero — Clawvisor-style: one strong idea, two lines, little else */}
        <section className="mx-auto max-w-4xl px-5 pb-16 pt-28 text-center sm:px-6 sm:pb-20 sm:pt-32 md:pt-40">
          <h1 className="mx-auto max-w-[18ch] text-balance text-[2.5rem] font-semibold leading-[1.08] tracking-[-0.045em] text-white sm:max-w-none sm:text-[3.25rem] sm:leading-[1.05] md:text-[3.5rem]">
            You show up once.
            <br />
            <span className="bg-gradient-to-r from-violet-200 via-violet-300 to-violet-400 bg-clip-text text-transparent">
              Your twin keeps going.
            </span>
          </h1>
          <p className="mx-auto mt-8 max-w-md text-balance text-[17px] leading-relaxed text-zinc-500 sm:mt-10 sm:max-w-lg sm:text-[18px]">
            Wallet or tap—FluvioMe gives them your AI twin, not another dead contact in their phone.
          </p>
          <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row sm:gap-4">
            <Link
              href="/onboarding"
              className="inline-flex h-12 min-h-12 w-full max-w-xs touch-manipulation items-center justify-center rounded-full bg-white px-8 text-[16px] font-semibold text-zinc-950 transition hover:bg-zinc-100 sm:w-auto"
            >
              Get started free
            </Link>
            <a
              href="#how"
              className="inline-flex h-12 min-h-12 w-full max-w-xs items-center justify-center rounded-full border border-white/[0.12] bg-transparent px-8 text-[16px] font-medium text-zinc-300 transition hover:border-violet-500/40 hover:bg-violet-500/[0.06] hover:text-white sm:w-auto"
            >
              How it works
            </a>
          </div>
        </section>

        {/* One product frame — simple, no fake window chrome */}
        <section className="mx-auto max-w-3xl px-5 pb-20 sm:px-6 sm:pb-28">
          <div className="overflow-hidden rounded-2xl border border-white/[0.08] bg-zinc-950 shadow-[0_40px_80px_-40px_rgba(0,0,0,0.85)] ring-1 ring-violet-500/10">
            <div className="relative bg-[#0a0a0c]">
              <div
                className="pointer-events-none absolute inset-0 z-[1] bg-[radial-gradient(ellipse_80%_45%_at_50%_0%,rgba(139,92,246,0.08),transparent_60%)]"
                aria-hidden
              />
              <LandingHeroVisual />
            </div>
          </div>
        </section>

        {/* How it works — numbered, spacious */}
        <section id="how" className="scroll-mt-24 border-t border-white/[0.06] py-20 sm:py-28">
          <div className="mx-auto max-w-4xl px-5 sm:px-6">
            <h2 className="text-center text-[13px] font-semibold uppercase tracking-[0.2em] text-violet-400/90">
              How it works
            </h2>
            <p className="mx-auto mt-4 max-w-xl text-center text-[15px] text-zinc-500">
              Three steps. No jargon.
            </p>
            <ol className="mx-auto mt-16 grid max-w-3xl gap-12 sm:gap-16">
              {steps.map((s) => (
                <li key={s.n} className="flex gap-6 sm:gap-8">
                  <div
                    className="flex h-12 w-12 shrink-0 items-center justify-center rounded-full border border-violet-500/35 bg-violet-500/[0.12] text-lg font-semibold tabular-nums text-violet-100 sm:h-14 sm:w-14 sm:text-xl"
                    aria-hidden
                  >
                    {s.n}
                  </div>
                  <div className="min-w-0 pt-0.5">
                    <h3 className="text-xl font-semibold tracking-[-0.03em] text-white sm:text-[1.35rem]">{s.title}</h3>
                    <p className="mt-2 text-[16px] leading-relaxed text-zinc-500">{s.body}</p>
                  </div>
                </li>
              ))}
            </ol>
          </div>
        </section>

        {/* Two paths — minimal cards */}
        <section className="border-t border-white/[0.06] py-20 sm:py-28">
          <div className="mx-auto max-w-4xl px-5 sm:px-6">
            <h2 className="text-center text-[1.75rem] font-semibold tracking-[-0.04em] text-white sm:text-[2rem]">
              Two ways to hand off you
            </h2>
            <p className="mx-auto mt-4 max-w-md text-center text-[16px] leading-relaxed text-zinc-500">
              Pick one or both. Same FluvioMe on the other side.
            </p>
            <div className="mx-auto mt-14 grid gap-5 sm:grid-cols-2 sm:gap-6">
              <Link
                href="/onboarding?path=wallet"
                className="group rounded-2xl border border-white/[0.08] bg-white/[0.02] p-8 transition hover:border-violet-500/35 hover:bg-violet-500/[0.04]"
              >
                <p className="text-[13px] font-medium uppercase tracking-[0.14em] text-violet-400/90">Wallet</p>
                <p className="mt-3 text-lg font-semibold text-white">Pass on the lock screen</p>
                <p className="mt-2 text-[15px] leading-relaxed text-zinc-500">Apple Wallet from Safari. Add once.</p>
                <span className="mt-6 inline-flex text-[15px] font-medium text-violet-300 group-hover:text-violet-200">
                  Set up →
                </span>
              </Link>
              <Link
                href="/onboarding?path=nfc"
                className="group rounded-2xl border border-white/[0.08] bg-white/[0.02] p-8 transition hover:border-violet-500/35 hover:bg-violet-500/[0.04]"
              >
                <p className="text-[13px] font-medium uppercase tracking-[0.14em] text-violet-400/90">Tap</p>
                <p className="mt-3 text-lg font-semibold text-white">Card in the hand</p>
                <p className="mt-2 text-[15px] leading-relaxed text-zinc-500">
                  {WIFI_NFC_PREORDER_ENABLED
                    ? "Design and order NFC—or pre-order Wi‑Fi NFC."
                    : "Design and order your NFC tap card. Wi‑Fi NFC variant coming soon."}
                </p>
                <span className="mt-6 inline-flex text-[15px] font-medium text-violet-300 group-hover:text-violet-200">
                  Design →
                </span>
              </Link>
            </div>
          </div>
        </section>

        {/* Final CTA */}
        <section className="border-t border-white/[0.06] py-20 sm:py-24">
          <div className="mx-auto max-w-xl px-5 text-center sm:px-6">
            <h2 className="text-[1.75rem] font-semibold tracking-[-0.04em] text-white sm:text-[2rem]">Ready when you are</h2>
            <p className="mt-4 text-[16px] leading-relaxed text-zinc-500">Takes a few minutes. Free while we open access.</p>
            <Link
              href="/onboarding"
              className="mx-auto mt-10 inline-flex h-12 items-center justify-center rounded-full bg-violet-500 px-10 text-[16px] font-semibold text-white transition hover:bg-violet-400"
            >
              Open FluvioMe
            </Link>
          </div>
        </section>

        <footer className="border-t border-white/[0.06] py-10">
          <div className="mx-auto flex max-w-4xl flex-col items-center justify-between gap-6 px-5 text-[14px] text-zinc-600 sm:flex-row sm:px-6">
            <p>© {new Date().getFullYear()} FluvioMe</p>
            <div className="flex flex-wrap justify-center gap-x-8 gap-y-2">
              <Link href="/onboarding" className="transition hover:text-zinc-400">
                Setup
              </Link>
              <Link href="/dashboard" className="transition hover:text-zinc-400">
                Overview
              </Link>
              <Link href="/chat" className="transition hover:text-zinc-400">
                Chat
              </Link>
            </div>
          </div>
        </footer>
      </main>
    </div>
  );
}
