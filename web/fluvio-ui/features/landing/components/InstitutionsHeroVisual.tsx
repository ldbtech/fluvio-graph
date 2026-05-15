"use client";

import Image from "next/image";
import { motion } from "framer-motion";

const foodPhoto =
  "https://images.unsplash.com/photo-1542838132-92c53300491e?auto=format&fit=crop&w=960&q=78";
const waterPhoto =
  "https://images.unsplash.com/photo-1582719478250-c89cae4dc85b?auto=format&fit=crop&w=960&q=78";
/** Editorial satellite / Earth imagery (Unsplash)—placeholder until your EO tiles and pipelines are live. */
const satelliteEarth =
  "https://images.unsplash.com/photo-1451187580459-43490279c0fa?auto=format&fit=crop&w=1600&q=80";
const satelliteAerial =
  "https://images.unsplash.com/photo-1625246333195-78d9c38ad449?auto=format&fit=crop&w=1600&q=80";

export const institutionsVerticalImages = { food: foodPhoto, water: waterPhoto } as const;
export const institutionsSatelliteImages = { earth: satelliteEarth, aerial: satelliteAerial } as const;

const systemTypes = [
  { label: "ERP", hint: "Finance & orders" },
  { label: "WMS", hint: "Warehouse" },
  { label: "TMS", hint: "Routes" },
  { label: "SCADA", hint: "Telemetry" },
  { label: "CMMS", hint: "Maintenance" },
  { label: "GIS", hint: "Network" },
  { label: "IoT", hint: "Sensors" },
  { label: "Sheets", hint: "Shadow IT" },
] as const;

function SystemChip({ label, hint }: { label: string; hint: string }) {
  return (
    <div
      className="flex shrink-0 items-center gap-3 rounded-xl border border-white/[0.1] bg-zinc-900/90 px-4 py-2.5 shadow-[0_0_0_1px_rgba(255,255,255,0.03)_inset]"
      title={hint}
    >
      <span className="flex size-9 items-center justify-center rounded-lg bg-gradient-to-br from-violet-500/25 to-sky-500/20 text-[11px] font-bold tracking-tight text-white">
        {label.slice(0, 3)}
      </span>
      <span className="text-[13px] font-medium text-zinc-200">{label}</span>
    </div>
  );
}

export function InstitutionsUrgencyStrip() {
  const chips = [
    "Cold-chain gaps",
    "Non-revenue water",
    "Spoilage & overstock",
    "Blind dispatch",
    "Reactive repairs",
    "Clock is ticking",
  ] as const;
  return (
    <div className="border-b border-amber-500/20 bg-gradient-to-r from-amber-500/[0.07] via-orange-500/[0.05] to-transparent">
      <div className="mx-auto flex max-w-4xl flex-col gap-2 px-5 py-3 sm:flex-row sm:items-center sm:justify-between sm:px-6">
        <p className="text-[13px] font-semibold uppercase tracking-[0.16em] text-amber-200/95">
          Resource pressure
        </p>
        <p className="text-[13px] leading-snug text-amber-100/85 sm:max-w-md sm:text-right">
          Shortages do not wait for your next integration project—we help teams see one truth faster.
        </p>
      </div>
      <div className="border-t border-amber-500/10 bg-black/25 py-2">
        <div className="mx-auto flex max-w-4xl flex-wrap justify-center gap-x-4 gap-y-1.5 px-4 text-[11px] font-medium text-amber-200/70 sm:justify-start sm:px-6">
          {chips.map((c) => (
            <span key={c} className="inline-flex items-center gap-1.5">
              <span className="size-1 rounded-full bg-amber-400/80" aria-hidden />
              {c}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

export function InstitutionsLogoMarquee() {
  const row = [...systemTypes, ...systemTypes];
  return (
    <div className="relative overflow-hidden border-y border-white/[0.06] bg-zinc-950/80 py-8">
      <div className="mx-auto mb-5 max-w-4xl px-5 text-center sm:px-6">
        <p className="text-[12px] font-semibold uppercase tracking-[0.22em] text-zinc-500">Systems that rarely agree</p>
        <p className="mt-1.5 text-[14px] text-zinc-400">Representative silos—yours will have different names, same fracture.</p>
      </div>
      <p className="sr-only">Scrolling row of generic system categories such as ERP, WMS, and telemetry.</p>
      <div className="inst-marquee-track items-center px-4">
        {row.map((s, i) => (
          <SystemChip key={`${s.label}-${i}`} label={s.label} hint={s.hint} />
        ))}
      </div>
      <div
        className="pointer-events-none absolute inset-y-0 left-0 z-[1] w-20 bg-gradient-to-r from-[#09090b] to-transparent sm:w-28"
        aria-hidden
      />
      <div
        className="pointer-events-none absolute inset-y-0 right-0 z-[1] w-20 bg-gradient-to-l from-[#09090b] to-transparent sm:w-28"
        aria-hidden
      />
    </div>
  );
}

function ArrowPulse({ delay }: { delay: number }) {
  return (
    <motion.div
      className="flex flex-col items-center gap-0.5 text-sky-400/90"
      initial={{ opacity: 0.35, x: -4 }}
      animate={{ opacity: [0.35, 1, 0.35], x: [-4, 4, -4] }}
      transition={{ duration: 2.4, repeat: Infinity, ease: "easeInOut", delay }}
      aria-hidden
    >
      <span className="text-lg leading-none">→</span>
      <span className="text-xs leading-none">→</span>
    </motion.div>
  );
}

export function InstitutionsFlowVisual() {
  const silos = [
    { title: "Orders", sub: "ERP / OMS" },
    { title: "Stock", sub: "WMS" },
    { title: "Field", sub: "IoT · CMMS" },
  ] as const;

  return (
    <div className="relative overflow-hidden rounded-2xl border border-white/[0.08] bg-gradient-to-b from-zinc-900/90 to-black/80 p-5 shadow-[0_32px_64px_-32px_rgba(0,0,0,0.85)] ring-1 ring-sky-500/10 sm:p-7">
      <div
        className="pointer-events-none absolute -right-20 -top-20 size-56 rounded-full bg-violet-500/10 blur-3xl"
        aria-hidden
      />
      <div
        className="pointer-events-none absolute -bottom-16 -left-16 size-48 rounded-full bg-sky-500/10 blur-3xl"
        aria-hidden
      />

      <p className="text-center text-[11px] font-semibold uppercase tracking-[0.2em] text-zinc-500">How it comes together</p>
      <p className="mx-auto mt-1 max-w-sm text-center text-[13px] text-zinc-400">
        Data stops dying in hand-offs. One graph + models + copilots + governed agents on the same facts.
      </p>

      <div className="mt-8 flex flex-col items-stretch gap-6 sm:mt-10 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
        <div className="flex flex-1 justify-center gap-2 sm:flex-col sm:gap-3 md:flex-row md:gap-2">
          {silos.map((s, i) => (
            <motion.div
              key={s.title}
              className="flex min-w-[5.5rem] flex-1 flex-col rounded-xl border border-white/[0.08] bg-white/[0.03] px-3 py-3 text-center sm:min-w-0 sm:flex-none sm:px-4"
              animate={{ y: [0, -3, 0] }}
              transition={{ duration: 3.2 + i * 0.2, repeat: Infinity, ease: "easeInOut" }}
            >
              <span className="text-[11px] font-medium uppercase tracking-wide text-zinc-500">{s.sub}</span>
              <span className="mt-1 text-sm font-semibold text-zinc-100">{s.title}</span>
            </motion.div>
          ))}
        </div>

        <div className="flex items-center justify-center gap-1 sm:flex-col sm:gap-2 md:flex-row">
          <ArrowPulse delay={0} />
          <ArrowPulse delay={0.35} />
          <ArrowPulse delay={0.7} />
        </div>

        <motion.div
          className="inst-hub-pulse relative mx-auto flex min-h-[8.5rem] min-w-[8.5rem] flex-col items-center justify-center rounded-2xl border border-violet-400/30 bg-gradient-to-br from-violet-500/20 via-zinc-900/80 to-sky-500/15 px-4 py-5 text-center sm:mx-0 sm:min-h-[9.5rem] sm:min-w-[9.5rem]"
          initial={{ scale: 0.94 }}
          animate={{ scale: [0.94, 1, 0.94] }}
          transition={{ duration: 3.5, repeat: Infinity, ease: "easeInOut" }}
        >
          <span className="text-[10px] font-bold uppercase tracking-[0.18em] text-violet-200/90">Fluvio</span>
          <span className="mt-1 text-lg font-semibold tracking-tight text-white">One graph</span>
          <span className="mt-1 text-[11px] leading-tight text-sky-200/80">ML · DL · Gen AI · Agents</span>
          <motion.span
            className="absolute -inset-1 -z-10 rounded-2xl bg-gradient-to-r from-violet-500/0 via-sky-400/15 to-violet-500/0 opacity-60 blur-md"
            animate={{ rotate: [0, 6, -4, 0] }}
            transition={{ duration: 8, repeat: Infinity, ease: "linear" }}
            aria-hidden
          />
        </motion.div>
      </div>
    </div>
  );
}

export type InstitutionVerticalItem = {
  title: string;
  body: string;
  imageSrc: string;
  imageAlt: string;
  accent: "emerald" | "sky";
};

export function InstitutionsVerticalShowcase({ items }: { items: InstitutionVerticalItem[] }) {
  const view = { once: true, margin: "-40px" } as const;
  const tagClass: Record<InstitutionVerticalItem["accent"], string> = {
    emerald: "text-emerald-300/90",
    sky: "text-sky-300/90",
  };

  return (
    <div className="mt-10 grid gap-8 sm:grid-cols-2">
      {items.map((item, idx) => (
        <motion.article
          key={item.title}
          initial={{ opacity: 0, y: 24 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={view}
          transition={{ duration: 0.5, delay: idx * 0.1 }}
          className="group flex flex-col overflow-hidden rounded-2xl border border-white/[0.08] bg-zinc-900/35 ring-1 ring-white/[0.03] transition hover:border-sky-500/25 hover:ring-sky-500/10"
        >
          <div className="relative aspect-[16/10] w-full shrink-0">
            <Image
              src={item.imageSrc}
              alt={item.imageAlt}
              fill
              className="object-cover transition duration-700 group-hover:scale-[1.03]"
              sizes="(max-width: 768px) 100vw, 50vw"
              priority={idx === 0}
            />
            <div className="absolute inset-0 bg-gradient-to-t from-black/88 via-black/25 to-transparent" />
            <div className="absolute bottom-0 left-0 right-0 p-4 sm:p-5">
              <p className={`text-[11px] font-semibold uppercase tracking-[0.18em] ${tagClass[item.accent]}`}>
                {item.title}
              </p>
            </div>
          </div>
          <div className="flex flex-1 flex-col p-6 pt-5">
            <p className="text-[15px] leading-relaxed text-zinc-400">{item.body}</p>
          </div>
        </motion.article>
      ))}
    </div>
  );
}

export function InstitutionsSatelliteStrip() {
  return (
    <section className="border-t border-white/[0.06] bg-black/40">
      <div className="mx-auto max-w-5xl px-5 py-12 sm:px-6 sm:py-16">
        <motion.div
          initial={{ opacity: 0, y: 14 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-50px" }}
          transition={{ duration: 0.45 }}
          className="mb-8 max-w-3xl"
        >
          <h2 className="text-[13px] font-semibold uppercase tracking-[0.2em] text-cyan-400/90">
            Satellite & Earth observation
          </h2>
          <p className="mt-3 text-[16px] leading-relaxed text-zinc-400 sm:text-[17px]">
            We integrate orbital and aerial imagery with your operational graph—vegetation stress, thermal hotspots, snowpack,
            water extent, night lights—so models can see{" "}
            <span className="text-zinc-200">context the yard and the ERP never shared</span>. That is how you get earlier
            warnings on drought, crop pressure, corridor congestion, and infrastructure risk—not just another pretty map.
          </p>
        </motion.div>

        <div className="grid gap-3 overflow-hidden rounded-2xl border border-white/[0.08] ring-1 ring-cyan-500/10 sm:grid-cols-2 sm:gap-0">
          <motion.div
            className="relative min-h-[200px] sm:min-h-[260px]"
            initial={{ opacity: 0, x: -12 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            <motion.div
              className="absolute inset-0"
              animate={{ scale: [1, 1.06, 1] }}
              transition={{ duration: 18, repeat: Infinity, ease: "easeInOut" }}
            >
              <Image
                src={institutionsSatelliteImages.earth}
                alt="Earth from space—symbolizing satellite data integrated into operations"
                fill
                className="object-cover"
                sizes="(max-width: 768px) 100vw, 50vw"
              />
            </motion.div>
            <div className="absolute inset-0 bg-gradient-to-t from-black/85 via-black/35 to-cyan-950/20" />
            <p className="absolute bottom-4 left-4 right-4 text-[12px] font-medium leading-snug text-white/95 sm:bottom-5 sm:left-5">
              Low-earth & geostationary feeds · harmonized to your regions and assets
            </p>
          </motion.div>
          <motion.div
            className="relative min-h-[200px] border-t border-white/[0.06] sm:border-l sm:border-t-0 sm:min-h-[260px]"
            initial={{ opacity: 0, x: 12 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.06 }}
          >
            <motion.div
              className="absolute inset-0"
              animate={{ scale: [1.05, 1, 1.05] }}
              transition={{ duration: 20, repeat: Infinity, ease: "easeInOut" }}
            >
              <Image
                src={institutionsSatelliteImages.aerial}
                alt="Aerial view of land patterns—symbolizing surface signals fused with distribution data"
                fill
                className="object-cover"
                sizes="(max-width: 768px) 100vw, 50vw"
              />
            </motion.div>
            <div className="absolute inset-0 bg-gradient-to-t from-black/85 via-black/30 to-emerald-950/15" />
            <p className="absolute bottom-4 left-4 right-4 text-[12px] font-medium leading-snug text-white/95 sm:bottom-5 sm:left-5">
              Aerial & derived indices · fused with routes, tanks, fields, and demand forecasts
            </p>
          </motion.div>
        </div>
      </div>
    </section>
  );
}

function AgentNode({
  title,
  sub,
  className,
  delay,
}: {
  title: string;
  sub: string;
  className?: string;
  delay: number;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-20px" }}
      transition={{ duration: 0.4, delay }}
      className={`rounded-xl border border-white/[0.1] bg-zinc-900/80 px-3 py-2.5 text-center shadow-[0_0_0_1px_rgba(255,255,255,0.04)_inset] ${className ?? ""}`}
    >
      <p className="text-[11px] font-semibold uppercase tracking-wide text-violet-300/95">{title}</p>
      <p className="mt-0.5 text-[10px] leading-tight text-zinc-500">{sub}</p>
    </motion.div>
  );
}

export function InstitutionsAgentFabric() {
  const specialists = [
    { title: "Ingest agent", sub: "EO tiles · PDF · SCADA" },
    { title: "Forecast agent", sub: "ML / DL ensemble" },
    { title: "Policy agent", sub: "routing · guardrails" },
    { title: "Gen agent", sub: "briefings · drafts" },
  ] as const;

  return (
    <section className="border-t border-white/[0.06] py-16 sm:py-20">
      <div className="mx-auto max-w-5xl px-5 sm:px-6">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-50px" }}
          transition={{ duration: 0.45 }}
          className="max-w-3xl"
        >
          <h2 className="text-[13px] font-semibold uppercase tracking-[0.2em] text-violet-400/90">Agent fabric you control</h2>
          <p className="mt-3 text-[16px] leading-relaxed text-zinc-400 sm:text-[17px]">
            Orchestrator agents can{" "}
            <span className="text-zinc-200">spin up child agents</span> for narrow jobs—satellite ingest, demand re-fit, leak
            triage, customer-safe summaries—each with its own tools and budgets.{" "}
            <span className="text-zinc-200">You decide</span> which agent is allowed to touch which subsystem, which model
            checkpoint runs where, and when human approval is required. Nothing runs as an opaque monolith.
          </p>
        </motion.div>

        <div className="mt-10 rounded-2xl border border-white/[0.08] bg-gradient-to-b from-zinc-900/60 to-black/60 p-6 sm:p-8">
          <p className="text-center text-[11px] font-semibold uppercase tracking-[0.2em] text-zinc-500">How agents delegate</p>
          <div className="mx-auto mt-8 flex max-w-3xl flex-col items-center gap-4">
            <AgentNode title="Orchestrator" sub="plans · delegates · audits" delay={0} className="min-w-[11rem] px-5 py-3" />
            <motion.div
              className="flex flex-col items-center gap-0.5 text-zinc-500"
              initial={{ opacity: 0 }}
              whileInView={{ opacity: 1 }}
              viewport={{ once: true }}
              transition={{ delay: 0.15 }}
              aria-hidden
            >
              <span className="text-lg leading-none">↓</span>
              <span className="text-[10px] uppercase tracking-wider">spawns and supervises</span>
            </motion.div>
            <div className="grid w-full grid-cols-2 gap-2 sm:grid-cols-4 sm:gap-3">
              {specialists.map((s, i) => (
                <AgentNode key={s.title} title={s.title} sub={s.sub} delay={0.08 + i * 0.06} />
              ))}
            </div>
          </div>

          <ul className="mx-auto mt-10 max-w-2xl space-y-2.5 text-[14px] leading-relaxed text-zinc-500">
            <li className="flex gap-2">
              <span className="mt-2 size-1 shrink-0 rounded-full bg-violet-400/80" aria-hidden />
              <span>
                <span className="font-medium text-zinc-300">Model routing:</span> pick which ML / DL weights apply per region,
                lane, or asset class—and freeze versions for compliance runs.
              </span>
            </li>
            <li className="flex gap-2">
              <span className="mt-2 size-1 shrink-0 rounded-full bg-sky-400/80" aria-hidden />
              <span>
                <span className="font-medium text-zinc-300">Agent routing:</span> map agents to data domains (e.g. satellite
                only after QA), rate limits, and escalation paths.
              </span>
            </li>
            <li className="flex gap-2">
              <span className="mt-2 size-1 shrink-0 rounded-full bg-emerald-400/80" aria-hidden />
              <span>
                <span className="font-medium text-zinc-300">Recursive specialists:</span> child agents can propose further
                sub-agents for micro-tasks; the orchestrator remains the kill switch.
              </span>
            </li>
          </ul>
        </div>
      </div>
    </section>
  );
}
