"use client";

import Link from "next/link";
import { LandingFutureVisual } from "./LandingFutureVisual";
import { LandingHeroVisual } from "./LandingHeroVisual";

const pillars = [
  {
    title: "Ingest",
    subtitle: "Sources write to one engine",
    body:
      "PDFs, Gmail, GitHub clones, and architecture briefs land in kg-engine as structured nodes, not a pile of embeddings you hope match later.",
  },
  {
    title: "Structure",
    subtitle: "A brain you can navigate",
    body:
      "Isolate slices per tab or fuse what you have enabled. The canvas is the contract: every answer can point back to a node you can see.",
  },
  {
    title: "Reason",
    subtitle: "Chat on the graph",
    body:
      "Docked chat stays grounded in the active slice, documents, mail, modules, or rooms, so the model walks edges instead of guessing.",
  },
] as const;

const live = [
  { name: "Gmail", hint: "Threads and labels as graph context." },
  { name: "PDF", hint: "Pages, chunks, and semantic edges." },
  { name: "GitHub", hint: "Clone, tree, resolve-import subgraphs." },
  { name: "Architecture", hint: "Plans and tools synced to the graph." },
] as const;

const soon = [
  { name: "Slack", hint: "Channels as first-class entities." },
  { name: "WhatsApp", hint: "Conversation exports, same pipeline." },
  { name: "More documents", hint: "DOCX, HTML, and more." },
  { name: "Video", hint: "Scenes and transcripts as graph slices." },
  { name: "Images & editing", hint: "Layers and variants as nodes." },
] as const;

function FluvioMark() {
  return (
    <span
      className="relative inline-flex h-7 w-7 items-center justify-center overflow-hidden rounded-md border border-sky-400/25 bg-slate-950/80 shadow-[inset_0_1px_0_rgba(125,211,252,0.25),0_0_24px_-10px_rgba(56,189,248,0.6)]"
      aria-hidden
    >
      <span className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_30%_15%,rgba(125,211,252,0.2),transparent_55%)]" />
      <svg viewBox="0 0 24 24" className="relative h-5 w-5 text-sky-300/90" fill="none">
        <path d="M12 3.5 L4.5 8.2 L4.5 15.8 L12 20.5 L19.5 15.8 L19.5 8.2 Z" stroke="currentColor" strokeWidth="1.1" opacity="0.7" />
        <circle cx="12" cy="7.7" r="1.5" className="fill-sky-200/95" />
        <circle cx="8.3" cy="14.7" r="1.35" className="fill-sky-300/85" />
        <circle cx="15.7" cy="14.7" r="1.35" className="fill-blue-300/85" />
        <path d="M12 9.2 L8.3 13.3 M12 9.2 L15.7 13.3 M8.3 14.7 L15.7 14.7" stroke="currentColor" strokeWidth="1.05" opacity="0.85" />
      </svg>
    </span>
  );
}

export function LandingPage() {
  return (
    <div className="min-h-screen bg-[#070a12] text-slate-100">
      {/* Cool blue ambient on near-black (reads less brown than warm neutrals) */}
      <div
        className="pointer-events-none fixed inset-0 -z-10 bg-[radial-gradient(ellipse_90%_55%_at_50%_-18%,rgba(56,189,248,0.12),transparent_52%),radial-gradient(ellipse_70%_45%_at_100%_0%,rgba(59,130,246,0.06),transparent_50%),radial-gradient(ellipse_60%_40%_at_0%_100%,rgba(14,165,233,0.05),transparent_45%)]"
        aria-hidden
      />

      <header className="fixed inset-x-0 top-0 z-50 border-b border-sky-500/[0.08] bg-[#070a12]/85 backdrop-blur-xl backdrop-saturate-150">
        <div className="mx-auto flex h-14 max-w-6xl items-center justify-between gap-2 px-3.5 sm:px-8">
          <Link
            href="/"
            className="inline-flex shrink-0 items-center gap-2 text-[15px] font-medium tracking-[-0.02em] text-slate-100 transition hover:text-sky-100"
          >
            <FluvioMark />
            <span>Fluvio</span>
          </Link>
          <nav className="flex items-center justify-end gap-0.5 sm:gap-2">
            <a
              href="#product"
              className="hidden rounded-full px-3 py-2 text-[13px] font-medium text-slate-400 transition hover:bg-sky-500/10 hover:text-sky-200 sm:inline-flex sm:px-4"
            >
              Product
            </a>
            <a
              href="#future"
              className="hidden rounded-full px-3 py-2 text-[13px] font-medium text-slate-400 transition hover:bg-sky-500/10 hover:text-sky-200 sm:inline-flex sm:px-4"
            >
              Future
            </a>
            <Link
              href="/qa"
              className="rounded-full px-2.5 py-2 text-[12px] font-medium text-slate-400 transition hover:bg-sky-500/10 hover:text-sky-200 sm:px-4 sm:text-[13px]"
            >
              QA
            </Link>
            <Link
              href="/workspace"
              className="rounded-full bg-sky-500 px-3 py-2 text-[12px] font-medium text-white shadow-[0_0_20px_-4px_rgba(14,165,233,0.45)] transition hover:bg-sky-400 sm:px-5 sm:text-[13px]"
            >
              Open workspace
            </Link>
          </nav>
        </div>
      </header>

      <main>
        {/* Hero, split on large screens like Linear / Apple product */}
        <section className="mx-auto grid max-w-6xl gap-10 px-5 pb-16 pt-20 sm:px-8 sm:pb-20 sm:pt-28 lg:grid-cols-2 lg:items-center lg:gap-16 lg:pt-32">
          <div className="max-w-xl lg:max-w-none">
            <p className="text-[11px] font-medium uppercase tracking-[0.16em] text-sky-400/70">Knowledge workspace</p>
            <h1 className="mt-3 text-balance text-[clamp(1.72rem,9vw,3.25rem)] font-medium leading-[1.08] tracking-[-0.045em] text-white sm:mt-4">
              Ground your AI in data it can navigate.
            </h1>
            <p className="mt-4 max-w-md text-pretty text-[14px] leading-[1.6] text-slate-400 sm:mt-5 sm:text-[16px]">
              One workspace for ingest, exploration, and chat, so retrieval follows structure instead of vibes.
            </p>
            <p className="mt-2.5 max-w-md text-pretty text-[13px] leading-relaxed text-slate-500 sm:mt-3 sm:text-[14px]">
              Built for teams that live in Gmail, repositories, and documents, and want the model to follow links and
              nodes, not only similar paragraphs.
            </p>
            <div className="mt-7 flex flex-wrap items-center gap-2.5 sm:mt-9 sm:gap-3">
              <Link
                href="/workspace"
                className="inline-flex h-10 items-center justify-center rounded-full bg-sky-500 px-5 text-[13px] font-medium text-white shadow-[0_0_24px_-6px_rgba(14,165,233,0.5)] transition hover:bg-sky-400 sm:h-11 sm:px-7 sm:text-[14px]"
              >
                Enter workspace
              </Link>
              <a
                href="#product"
                className="inline-flex h-10 items-center justify-center rounded-full border border-sky-500/25 bg-sky-500/[0.06] px-4.5 text-[13px] font-medium text-sky-100/90 transition hover:border-sky-400/35 hover:bg-sky-500/10 sm:h-11 sm:px-6 sm:text-[14px]"
              >
                How it works
              </a>
            </div>
          </div>

          <div className="relative min-w-0">
            <div
              className="relative overflow-hidden rounded-[1.25rem] ring-1 ring-sky-500/15 sm:rounded-3xl"
              style={{
                boxShadow:
                  "0 0 0 1px rgba(56,189,248,0.06) inset, 0 40px 80px -40px rgba(0,0,0,0.75), 0 24px 56px -28px rgba(14,165,233,0.12)",
              }}
            >
              <div
                className="pointer-events-none absolute inset-0 z-[1] bg-[radial-gradient(ellipse_75%_65%_at_50%_0%,rgba(56,189,248,0.08),transparent_60%)]"
                aria-hidden
              />
              <LandingHeroVisual />
            </div>
            <p className="mt-4 text-center text-[11px] font-medium tracking-[0.02em] text-slate-500 sm:text-left">
              Ingest · graph · reason
            </p>
          </div>
        </section>

        {/* Bento, Notion / Apple–style feature blocks */}
        <section
          id="product"
          className="scroll-mt-24 border-t border-sky-500/[0.08] bg-gradient-to-b from-sky-950/30 via-[#070a12]/80 to-[#070a12] py-16 sm:py-28"
        >
          <div className="mx-auto max-w-6xl px-5 sm:px-8">
            <div className="max-w-2xl">
              <h2 className="text-[11px] font-medium uppercase tracking-[0.16em] text-sky-400/70">How it works</h2>
              <p className="mt-3 text-2xl font-medium tracking-[-0.03em] text-white sm:text-[1.75rem] sm:leading-snug">
                Three beats. One loop.
              </p>
            </div>

            <div className="mt-10 grid gap-3.5 sm:mt-14 sm:grid-cols-3 sm:gap-5">
              {pillars.map((p) => (
                <article
                  key={p.title}
                  className="flex flex-col rounded-2xl border border-sky-500/10 bg-slate-950/40 p-5 shadow-[inset_0_1px_0_rgba(56,189,248,0.06)] sm:rounded-3xl sm:p-8"
                >
                  <h3 className="text-lg font-medium tracking-[-0.02em] text-white">{p.title}</h3>
                  <p className="mt-1 text-[13px] font-medium text-slate-500">{p.subtitle}</p>
                  <p className="mt-4 flex-1 text-[14px] leading-[1.65] text-slate-400">{p.body}</p>
                </article>
              ))}
            </div>
          </div>
        </section>

        {/* Sources: compact cards by status (not twin document columns) */}
        <section className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-28">
          <div className="max-w-2xl">
            <h2 className="text-[11px] font-medium uppercase tracking-[0.16em] text-sky-400/70">Sources</h2>
            <p className="mt-3 text-2xl font-medium tracking-[-0.03em] text-white sm:text-[1.75rem]">
              Connectors, one graph
            </p>
            <p className="mt-3 max-w-xl text-[14px] leading-relaxed text-slate-500 sm:text-[15px]">
              Same graph for the canvas and chat. What you enable is what you can traverse.
            </p>
          </div>

          <div className="mt-8 space-y-8 sm:mt-10 sm:space-y-10">
            <div>
              <div className="mb-3 flex items-center gap-2">
                <span className="size-1.5 shrink-0 rounded-full bg-sky-400 shadow-[0_0_8px_rgba(56,189,248,0.35)]" aria-hidden />
                <h3 className="text-[13px] font-medium tracking-[-0.01em] text-slate-200">Live today</h3>
              </div>
              <ul className="grid gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
                {live.map((x) => (
                  <li
                    key={x.name}
                    className="flex flex-col rounded-xl border border-sky-500/15 bg-slate-950/50 px-3.5 py-3 shadow-[inset_0_1px_0_rgba(56,189,248,0.08)] sm:px-4 sm:py-3.5"
                  >
                    <span className="text-[14px] font-medium tracking-[-0.02em] text-white">{x.name}</span>
                    <span className="mt-1 text-[12px] leading-snug text-slate-500">{x.hint}</span>
                  </li>
                ))}
              </ul>
            </div>

            <div>
              <div className="mb-3 flex items-center gap-2">
                <span className="size-1.5 shrink-0 rounded-full bg-blue-400/80" aria-hidden />
                <h3 className="text-[13px] font-medium tracking-[-0.01em] text-slate-400">Roadmap</h3>
              </div>
              <ul className="grid gap-2.5 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5">
                {soon.map((x) => (
                  <li
                    key={x.name}
                    className="flex flex-col rounded-xl border border-dashed border-blue-400/20 bg-slate-950/30 px-3.5 py-3 sm:px-4 sm:py-3.5"
                  >
                    <span className="text-[14px] font-medium tracking-[-0.02em] text-slate-300">{x.name}</span>
                    <span className="mt-1 text-[12px] leading-snug text-slate-600">{x.hint}</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </section>

        {/* Vision: media → graph → autonomous edits (no HITL) */}
        <section
          id="future"
          className="scroll-mt-24 border-t border-sky-500/[0.08] bg-gradient-to-b from-blue-950/35 via-[#070a12] to-transparent py-16 sm:py-28"
        >
          <div className="mx-auto max-w-6xl px-5 sm:px-8">
            <div className="max-w-2xl">
              <h2 className="text-[11px] font-medium uppercase tracking-[0.16em] text-sky-400/70">Future</h2>
              <p className="mt-3 text-2xl font-medium tracking-[-0.03em] text-white sm:text-[1.75rem] sm:leading-snug">
                Video & images through the graph, edits without a human in the loop
              </p>
              <p className="mt-4 text-[15px] leading-relaxed text-slate-500">
                Same contract as today: everything lands as nodes and edges. Policies, budgets, and kill-switches replace
                you babysitting every click. The model proposes structured edits; the engine applies what you allow.
              </p>
            </div>

            <div className="mt-8 rounded-2xl border border-sky-500/12 bg-slate-950/40 p-3.5 shadow-[inset_0_1px_0_rgba(56,189,248,0.06)] sm:mt-12 sm:rounded-3xl sm:p-8">
              <LandingFutureVisual />
            </div>

          </div>
        </section>

        {/* Closing CTA, single focal, no gradient frame shouting */}
        <section className="border-t border-sky-500/[0.08] py-16 sm:py-24">
          <div className="mx-auto max-w-2xl px-5 text-center sm:px-8">
            <h2 className="text-2xl font-medium tracking-[-0.03em] text-white sm:text-[1.65rem]">Run it against your engine</h2>
            <p className="mx-auto mt-3 max-w-md text-[15px] leading-relaxed text-slate-500">
              Sources, brain canvas, and docked chat. Open the workspace when your kg-engine instance is running.
            </p>
            <Link
              href="/workspace"
              className="mt-7 inline-flex h-10 items-center justify-center rounded-full bg-sky-500 px-6 text-[13px] font-medium text-white shadow-[0_0_28px_-6px_rgba(14,165,233,0.55)] transition hover:bg-sky-400 sm:mt-8 sm:h-11 sm:px-8 sm:text-[14px]"
            >
              Open workspace
            </Link>
            <p className="mx-auto mt-8 max-w-lg text-[13px] leading-relaxed text-slate-600">
              Fluvio and kg-engine are not published as a public repository. Distribution stays private so the same
              capabilities are harder to turn into a generic mass scanning or “drive by repo” workflow against third
              party code or documents you do not own.
            </p>
          </div>
        </section>

        <footer className="border-t border-sky-500/[0.08] py-10">
          <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-4 px-5 text-[12px] text-slate-600 sm:flex-row sm:px-8">
            <span>
              © {new Date().getFullYear()} Fluvio · Closed source · KG workspace for kg-engine
            </span>
            <div className="flex flex-wrap items-center justify-center gap-x-6 gap-y-2">
              <Link href="/workspace" className="transition hover:text-sky-400">
                Workspace
              </Link>
              <Link href="/qa" className="transition hover:text-sky-400">
                QA
              </Link>
            </div>
          </div>
        </footer>
      </main>
    </div>
  );
}
