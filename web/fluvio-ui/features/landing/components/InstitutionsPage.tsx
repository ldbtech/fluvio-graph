"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import { FluvioMark } from "./FluvioMark";
import {
  InstitutionsAgentFabric,
  InstitutionsFlowVisual,
  InstitutionsLogoMarquee,
  InstitutionsSatelliteStrip,
  InstitutionsUrgencyStrip,
  InstitutionsVerticalShowcase,
  institutionsVerticalImages,
} from "./InstitutionsHeroVisual";

const pillars = [
  {
    title: "Structured operational data",
    body: "One canonical layer across inventory, routes, cold chain, tanks, and maintenance events—so reporting and automation do not depend on brittle CSV hops.",
    icon: (
      <svg viewBox="0 0 24 24" className="size-6" fill="none" aria-hidden>
        <path d="M4 6h16M4 12h10M4 18h16" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="text-sky-400/90" />
        <rect x="14" y="10" width="6" height="4" rx="1" className="stroke-violet-400/80" strokeWidth="1.2" fill="none" />
      </svg>
    ),
  },
  {
    title: "ML & deep learning",
    body: "Demand and shelf-life signals, anomaly detection on flows and pressures, and models that learn from your own history—not a generic dashboard bolted on top.",
    icon: (
      <svg viewBox="0 0 24 24" className="size-6" fill="none" aria-hidden>
        <path
          d="M12 3v4M12 17v4M4 12h4M16 12h4"
          stroke="currentColor"
          strokeWidth="1.35"
          strokeLinecap="round"
          className="text-violet-400/90"
        />
        <circle cx="12" cy="12" r="3.2" className="stroke-sky-400/85" strokeWidth="1.25" fill="none" />
      </svg>
    ),
  },
  {
    title: "Generative AI",
    body: "Natural-language answers over your graph, draft incident summaries, and assistive workflows for planners and field teams—grounded in data you control.",
    icon: (
      <svg viewBox="0 0 24 24" className="size-6" fill="none" aria-hidden>
        <path d="M8 10h8M8 14h5" stroke="currentColor" strokeWidth="1.35" strokeLinecap="round" className="text-emerald-400/85" />
        <path d="M6 6l2 12h8l2-12H6z" className="stroke-zinc-500" strokeWidth="1.1" fill="none" />
      </svg>
    ),
  },
  {
    title: "Satellite signals in the same graph",
    body: "Fuse EO and aerial layers with yards, pipes, and SKUs—so ML and DL models can learn from canopy stress, thermal drift, and corridor activity alongside ERP and telemetry—not a disconnected GIS side project.",
    icon: (
      <svg viewBox="0 0 24 24" className="size-6" fill="none" aria-hidden>
        <circle cx="12" cy="12" r="7" className="stroke-cyan-400/85" strokeWidth="1.2" fill="none" />
        <path d="M12 5v3M12 16v3M5 12h3M16 12h3" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" className="text-cyan-300/70" />
        <circle cx="12" cy="12" r="1.8" className="fill-violet-400/90" />
      </svg>
    ),
  },
] as const;

const verticalShowcase = [
  {
    title: "Food distribution",
    body: "Spoilage and overproduction often start when WMS, routing, and retail systems disagree. We focus on the seams: freshness windows, returns, and last-mile variance.",
    imageSrc: institutionsVerticalImages.food,
    imageAlt: "Fresh produce at a market—representing food supply and cold-chain context.",
    accent: "emerald" as const,
  },
  {
    title: "Water distribution",
    body: "Losses show up as unbilled usage, slow leaks, and reactive maintenance. Unifying telemetry, work orders, and billing context makes those patterns visible earlier.",
    imageSrc: institutionsVerticalImages.water,
    imageAlt: "Industrial piping and valves—representing water distribution infrastructure.",
    accent: "sky" as const,
  },
];

const container = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.12, delayChildren: 0.06 },
  },
};

const itemFade = {
  hidden: { opacity: 0, y: 18 },
  show: { opacity: 1, y: 0, transition: { duration: 0.45, ease: "easeOut" as const } },
};

export function InstitutionsPage() {
  return (
    <div className="min-h-screen bg-[#09090b] text-zinc-100 antialiased">
      <div
        className="pointer-events-none fixed inset-0 -z-10 bg-[radial-gradient(ellipse_70%_50%_at_50%_-10%,rgba(59,130,246,0.08),transparent_55%),radial-gradient(ellipse_55%_40%_at_80%_30%,rgba(139,92,246,0.06),transparent_50%)]"
        aria-hidden
      />

      <header className="border-b border-white/[0.06] bg-[#09090b]/80 backdrop-blur-xl">
        <div className="mx-auto flex h-14 max-w-5xl items-center justify-between px-5 sm:h-16 sm:px-6">
          <Link href="/" className="flex items-center gap-2.5 text-white transition hover:opacity-90" aria-label="FluvioMe home">
            <FluvioMark />
            <span className="text-[17px] font-semibold tracking-[-0.03em]">FluvioMe</span>
          </Link>
          <nav className="flex items-center gap-3 sm:gap-4">
            <Link href="/#how" className="text-[14px] text-zinc-500 transition hover:text-zinc-300">
              Personal
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

      <InstitutionsUrgencyStrip />

      <main>
        <section className="mx-auto max-w-5xl px-5 pb-12 pt-12 sm:px-6 sm:pb-16 sm:pt-16">
          <div className="grid items-center gap-12 lg:grid-cols-[1fr_1.05fr] lg:gap-14">
            <motion.div initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5 }}>
              <p className="text-[13px] font-semibold uppercase tracking-[0.18em] text-sky-400/90">Institutions</p>
              <h1 className="mt-4 max-w-[20ch] text-balance text-[2rem] font-semibold leading-[1.12] tracking-[-0.04em] text-white sm:max-w-none sm:text-[2.5rem] sm:leading-[1.08] lg:text-[2.65rem]">
                Less waste starts when your systems finally agree.
              </h1>
              <p className="mt-6 max-w-xl text-[17px] leading-relaxed text-zinc-400 sm:text-[18px]">
                Water, food, and power grids are under real strain—yet distribution teams still lose days reconciling ERP, WMS,
                telemetry, and spreadsheets.{" "}
                <span className="text-zinc-200">
                  Every week without integration is spoilage, non-revenue water, and trucks running half-blind.
                </span>{" "}
                FluvioMe is building a path for{" "}
                <span className="text-zinc-200">
                  structured data, satellite-backed signals, ML, deep learning, governed agents, and generative AI
                </span>{" "}
                on one queryable layer—not another silo.
              </p>
            </motion.div>
            <motion.div
              initial={{ opacity: 0, scale: 0.97 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ duration: 0.55, delay: 0.08, ease: [0.22, 1, 0.36, 1] }}
            >
              <InstitutionsFlowVisual />
            </motion.div>
          </div>
        </section>

        <InstitutionsLogoMarquee />

        <InstitutionsSatelliteStrip />

        <InstitutionsAgentFabric />

        <section className="border-t border-white/[0.06] py-16 sm:py-20">
          <div className="mx-auto max-w-5xl px-5 sm:px-6">
            <motion.div
              initial={{ opacity: 0, y: 12 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: "-60px" }}
              transition={{ duration: 0.45 }}
            >
              <h2 className="text-[13px] font-semibold uppercase tracking-[0.2em] text-violet-400/90">Capability stack</h2>
              <p className="mt-3 max-w-2xl text-[15px] leading-relaxed text-zinc-500">
                Structured events, satellite-derived context, model ensembles, and agent swarms—under your routing rules and
                approvals—so you are not stuck exporting the truth every Monday.
              </p>
            </motion.div>
            <motion.ul
              className="mt-12 grid gap-8 sm:gap-10"
              variants={container}
              initial="hidden"
              whileInView="show"
              viewport={{ once: true, margin: "-40px" }}
            >
              {pillars.map((p) => (
                <motion.li
                  key={p.title}
                  variants={itemFade}
                  className="flex gap-5 rounded-2xl border border-white/[0.06] bg-white/[0.02] p-5 sm:gap-6 sm:p-6"
                >
                  <div className="flex size-12 shrink-0 items-center justify-center rounded-xl border border-white/[0.08] bg-black/40">
                    {p.icon}
                  </div>
                  <div className="min-w-0">
                    <h3 className="text-lg font-semibold tracking-[-0.03em] text-white sm:text-xl">{p.title}</h3>
                    <p className="mt-2 text-[16px] leading-relaxed text-zinc-500">{p.body}</p>
                  </div>
                </motion.li>
              ))}
            </motion.ul>
          </div>
        </section>

        <section className="border-t border-white/[0.06] py-16 sm:py-20">
          <div className="mx-auto max-w-5xl px-5 sm:px-6">
            <motion.div
              initial={{ opacity: 0, y: 12 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: "-60px" }}
              transition={{ duration: 0.45 }}
            >
              <h2 className="text-[13px] font-semibold uppercase tracking-[0.2em] text-emerald-400/85">Where we focus first</h2>
              <p className="mt-3 max-w-2xl text-[15px] leading-relaxed text-zinc-500">
                Two distribution verticals where disconnected systems show up directly as physical waste—and where faster
                integration buys back time, liters, and tons.
              </p>
            </motion.div>
            <InstitutionsVerticalShowcase items={verticalShowcase} />
            <p className="mt-6 text-center text-[11px] text-zinc-600">
              Photography and satellite imagery via{" "}
              <a href="https://unsplash.com" className="underline decoration-white/20 underline-offset-2 hover:text-zinc-400">
                Unsplash
              </a>{" "}
              (community license)—replace with your licensed tiles and basemaps in production.
            </p>
          </div>
        </section>

        <section id="pilot" className="border-t border-white/[0.06] py-16 sm:py-20">
          <motion.div
            className="mx-auto max-w-2xl px-5 text-center sm:px-6"
            initial={{ opacity: 0, y: 16 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            <h2 className="text-[1.5rem] font-semibold tracking-[-0.04em] text-white sm:text-[1.75rem]">Pilot with us</h2>
            <p className="mt-4 text-[16px] leading-relaxed text-zinc-500">
              We are onboarding institution-shaped workloads in small cohorts—distribution, utilities, and shared infrastructure
              teams. If your KPIs include liters saved, tons not lost, and hours not spent reconciling, we should talk this
              quarter.
            </p>
            <div className="mt-10 flex flex-col items-center justify-center gap-3 sm:flex-row sm:gap-4">
              <a
                href="mailto:institutions@fluviome.com?subject=Distribution%20%2F%20institution%20pilot"
                className="inline-flex h-12 min-h-12 w-full max-w-xs items-center justify-center rounded-full bg-white px-8 text-[16px] font-semibold text-zinc-950 transition hover:bg-zinc-100 sm:w-auto"
              >
                Email institutions
              </a>
              <Link
                href="/onboarding"
                className="inline-flex h-12 min-h-12 w-full max-w-xs items-center justify-center rounded-full border border-white/[0.12] px-8 text-[16px] font-medium text-zinc-300 transition hover:border-violet-500/40 hover:bg-violet-500/[0.06] hover:text-white sm:w-auto"
              >
                Open consumer setup
              </Link>
            </div>
          </motion.div>
        </section>

        <footer className="border-t border-white/[0.06] py-10">
          <div className="mx-auto flex max-w-5xl flex-col items-center justify-between gap-6 px-5 text-[14px] text-zinc-600 sm:flex-row sm:px-6">
            <p>© {new Date().getFullYear()} FluvioMe</p>
            <div className="flex flex-wrap justify-center gap-x-8 gap-y-2">
              <Link href="/" className="transition hover:text-zinc-400">
                Home
              </Link>
              <Link href="/dashboard" className="transition hover:text-zinc-400">
                Overview
              </Link>
            </div>
          </div>
        </footer>
      </main>
    </div>
  );
}
