"use client";

import type { ReactNode } from "react";
import Link from "next/link";

type Tile = {
  href: string;
  title: string;
  subtitle: string;
  span: string;
  mock: ReactNode;
};

function WindowDots() {
  return (
    <div className="flex items-center gap-1.5 border-b border-white/[0.06] bg-black/25 px-3 py-2.5">
      <span className="size-[7px] rounded-full bg-white/12" aria-hidden />
      <span className="size-[7px] rounded-full bg-white/10" aria-hidden />
      <span className="size-[7px] rounded-full bg-white/10" aria-hidden />
    </div>
  );
}

function CreateFluvioMeMock() {
  return (
    <div className="relative flex min-h-[200px] flex-1 flex-col gap-5 p-5 sm:min-h-[240px]" aria-hidden>
      <div className="flex gap-4">
        <div className="flex size-16 shrink-0 items-center justify-center rounded-xl border border-white/[0.1] bg-white/[0.03]">
          <div className="size-[52px] rounded-full border border-white/10 bg-black/40" />
        </div>
        <div className="flex flex-1 flex-col justify-center gap-2">
          <div className="h-2 max-w-[11rem] rounded-full bg-white/10" />
          <div className="h-2 max-w-[7rem] rounded-full bg-white/[0.06]" />
        </div>
      </div>
      <div className="rounded-xl border border-white/[0.08] bg-white/[0.02] px-4 py-4">
        <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-white/75">Add to Wallet</p>
        <p className="mt-2 text-[12px] text-zinc-500">Apple Wallet · Google Wallet</p>
      </div>
    </div>
  );
}

function WorkspaceMock() {
  return (
    <div className="relative flex min-h-[150px] flex-1 gap-3 p-4 sm:p-5" aria-hidden>
      <div className="flex w-7 shrink-0 flex-col gap-1 rounded-lg border border-white/[0.06] bg-black/40 p-1">
        {[4, 3, 4, 2].map((h, i) => (
          <div key={i} className="rounded bg-white/[0.05]" style={{ height: `${h * 6}px` }} />
        ))}
      </div>
      <div className="flex flex-1 flex-col justify-end rounded-xl border border-white/[0.06] bg-black/30 p-3">
        <div className="h-9 w-full rounded-lg bg-white/[0.05]" />
      </div>
    </div>
  );
}

function ChatMock() {
  return (
    <div className="flex min-h-[130px] flex-col justify-center gap-2.5 p-4 sm:min-h-[150px]" aria-hidden>
      <div className="rounded-2xl border border-white/[0.06] bg-white/[0.03] px-3.5 py-3">
        <p className="text-[12px] leading-snug text-zinc-500">Who are you?</p>
      </div>
      <div className="ml-8 rounded-2xl border border-white/[0.08] bg-white/[0.04] px-3.5 py-3">
        <div className="h-2 w-[76%] rounded-full bg-white/10" />
        <div className="mt-2 h-2 w-[44%] rounded-full bg-white/[0.07]" />
      </div>
    </div>
  );
}

function DashboardMock() {
  return (
    <div className="grid flex-1 grid-cols-2 gap-2 p-4 sm:p-5" aria-hidden>
      <div className="rounded-xl border border-white/[0.06] bg-white/[0.02] p-3">
        <div className="h-10 w-full rounded-lg bg-black/35" />
        <p className="mt-2.5 text-[10px] font-medium uppercase tracking-[0.1em] text-zinc-500">Pass</p>
      </div>
      <div className="rounded-xl border border-white/[0.06] bg-white/[0.02] p-3">
        <div className="mx-auto mt-3 size-9 rounded-full border border-white/10 bg-white/[0.04]" />
        <p className="mt-3 text-center text-[10px] uppercase tracking-[0.1em] text-zinc-500">Tap</p>
      </div>
    </div>
  );
}

export function LandingProductPreview() {
  const tiles: Tile[] = [
    {
      href: "/onboarding",
      title: "Setup",
      subtitle: "You · Wallet · optional NFC",
      span: "md:col-span-8 md:row-span-2 md:col-start-1 md:row-start-1",
      mock: (
        <>
          <WindowDots />
          <CreateFluvioMeMock />
        </>
      ),
    },
    {
      href: "/chat",
      title: "Chat",
      subtitle: "Your twin answers",
      span: "md:col-span-4 md:col-start-9 md:row-start-1",
      mock: (
        <>
          <WindowDots />
          <ChatMock />
        </>
      ),
    },
    {
      href: "/dashboard",
      title: "Passes",
      subtitle: "Wallet & NFC status",
      span: "md:col-span-4 md:col-start-9 md:row-start-2",
      mock: (
        <>
          <WindowDots />
          <DashboardMock />
        </>
      ),
    },
    {
      href: "/tap",
      title: "Tap",
      subtitle: "What they see after NFC",
      span: "md:col-span-6 md:col-start-1 md:row-start-3",
      mock: (
        <>
          <WindowDots />
          <div className="flex flex-1 items-center justify-center p-10" aria-hidden>
            <div className="rounded-full border border-white/15 bg-white/[0.04] px-10 py-4 text-[12px] font-semibold uppercase tracking-[0.18em] text-white/85">
              Tap
            </div>
          </div>
        </>
      ),
    },
    {
      href: "/workspace",
      title: "Tune",
      subtitle: "Edit how you sound",
      span: "md:col-span-6 md:col-start-7 md:row-start-3",
      mock: (
        <>
          <WindowDots />
          <WorkspaceMock />
        </>
      ),
    },
  ];

  return (
    <section id="product" className="scroll-mt-24 border-t border-white/[0.06] py-20 sm:py-28">
      <div className="mx-auto max-w-5xl px-5 sm:px-10">
        <p className="text-[13px] font-semibold uppercase tracking-[0.16em] text-zinc-500">The product</p>
        <h2 className="mt-4 max-w-lg text-[1.875rem] font-semibold tracking-[-0.04em] text-white sm:text-[2rem]">
          Five screens.
          <br />
          Same you.
        </h2>

        <div className="mt-14 grid grid-cols-1 gap-4 sm:gap-4 md:grid-cols-12 md:grid-rows-3 md:auto-rows-min md:gap-4">
          {tiles.map((t) => (
            <Link
              key={t.href}
              href={t.href}
              className={[
                "group flex flex-col overflow-hidden rounded-2xl border border-white/[0.06] bg-zinc-900/30 outline-none transition hover:border-white/[0.14] hover:bg-zinc-900/50 focus-visible:ring-2 focus-visible:ring-white/25 lg:rounded-3xl",
                t.span,
              ].join(" ")}
            >
              <div className="flex items-start justify-between gap-4 border-b border-white/[0.05] px-4 py-4 sm:px-5 sm:py-5">
                <div>
                  <h3 className="text-[16px] font-semibold tracking-[-0.02em] text-white">{t.title}</h3>
                  <p className="mt-1 text-[13px] text-zinc-500">{t.subtitle}</p>
                </div>
                <span className="shrink-0 pt-0.5 text-[13px] font-medium text-white/35 transition group-hover:text-white">
                  Open →
                </span>
              </div>
              <div className="flex min-h-0 flex-1 flex-col bg-black/20">{t.mock}</div>
            </Link>
          ))}
        </div>
      </div>
    </section>
  );
}
