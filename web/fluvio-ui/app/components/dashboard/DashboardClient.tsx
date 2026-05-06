"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import { DashboardAppleWallet } from "./DashboardAppleWallet";
import { DashboardForceGraph } from "./DashboardForceGraph";
import { getKgEngineUrl } from "@/lib/constants";
import {
  fetchFluvioAccount,
  fetchFluvioSocialGraph,
  getOwnerId,
  postFluvioAccountProfile,
  postFluvioIngest,
  postTwinSetup,
  toGraphNodesEdges,
  type FluvioAccount,
} from "@/lib/fluvioDashboardApi";
import type { GraphEdge, GraphNode } from "@/lib/types";

function basenamePdf(name: string) {
  return name.replace(/\.pdf$/i, "").trim() || "Resume";
}

function isSetupDocumentKind(kind: string): boolean {
  const normalized = kind.trim().toLowerCase();
  return normalized === "note" || normalized === "pdf";
}

export function DashboardClient() {
  /** Set after client mount from `twin_owner_id` (private mode has no id). */
  const [sessionOwnerId, setSessionOwnerId] = useState<string | null | "pending">("pending");
  const [account, setAccount] = useState<FluvioAccount | null>(null);
  const [socialNodes, setSocialNodes] = useState<GraphNode[]>([]);
  const [socialEdges, setSocialEdges] = useState<GraphEdge[]>([]);
  const [selectedConn, setSelectedConn] = useState<string | null>(null);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [ingestTitle, setIngestTitle] = useState("");
  const [ingestBody, setIngestBody] = useState("");
  const [ingestBusy, setIngestBusy] = useState(false);
  const [quickBusy, setQuickBusy] = useState<string | null>(null);
  const [profileEmail, setProfileEmail] = useState("");
  const [profilePhone, setProfilePhone] = useState("");
  const [profileSaveBusy, setProfileSaveBusy] = useState(false);
  const [setupName, setSetupName] = useState("");
  const [setupEmail, setSetupEmail] = useState("");
  const [setupBusy, setSetupBusy] = useState(false);
  const [setupErr, setSetupErr] = useState<string | null>(null);
  const pdfRef = useRef<HTMLInputElement>(null);

  const reload = useCallback(async () => {
    const id = getOwnerId();
    setSessionOwnerId(id);
    if (!id) {
      setAccount(null);
      setSocialNodes([]);
      setSocialEdges([]);
      return;
    }
    setLoadErr(null);
    const ac = new AbortController();
    const t = window.setTimeout(() => ac.abort(), 25_000);
    try {
      const [acc, soc] = await Promise.all([
        fetchFluvioAccount(ac.signal),
        fetchFluvioSocialGraph(ac.signal),
      ]);
      setAccount(acc);
      const g = toGraphNodesEdges(soc);
      setSocialNodes(g.nodes);
      setSocialEdges(g.edges);
    } catch (e) {
      const aborted =
        (e instanceof DOMException && e.name === "AbortError") ||
        (e instanceof Error && e.name === "AbortError");
      setLoadErr(
        aborted
          ? `Request timed out (25s) — ensure kg-engine is running at ${getKgEngineUrl()}`
          : e instanceof Error
            ? e.message
            : "Failed to load dashboard",
      );
    } finally {
      window.clearTimeout(t);
    }
  }, []);

  /** One effect: hydrate `twin_owner_id` from localStorage then load dashboard (fixes race where load never fired). */
  useEffect(() => {
    void reload();
  }, [reload]);

  const runQuickIngest = useCallback(
    async (key: string, payload: Parameters<typeof postFluvioIngest>[0]) => {
      setQuickBusy(key);
      setLoadErr(null);
      try {
        await postFluvioIngest(payload);
        await reload();
      } catch (e) {
        setLoadErr(e instanceof Error ? e.message : "Ingest failed");
      } finally {
        setQuickBusy(null);
      }
    },
    [reload],
  );

  useEffect(() => {
    if (!account) return;
    setProfileEmail(account.email ?? "");
    setProfilePhone(account.phone ?? "");
  }, [account]);

  const onSaveProfile = useCallback(async () => {
    setProfileSaveBusy(true);
    setLoadErr(null);
    try {
      await postFluvioAccountProfile({
        email: profileEmail.trim(),
        phone: profilePhone.trim(),
      });
      await reload();
    } catch (e) {
      setLoadErr(e instanceof Error ? e.message : "Could not save profile");
    } finally {
      setProfileSaveBusy(false);
    }
  }, [profileEmail, profilePhone, reload]);

  const openConnection = useCallback((id: string) => {
    setSelectedConn(id);
  }, []);

  const onIngest = useCallback(async () => {
    setIngestBusy(true);
    try {
      await postFluvioIngest({
        title: ingestTitle || undefined,
        body: ingestBody || undefined,
        kind: "note",
      });
      setIngestTitle("");
      setIngestBody("");
      await reload();
    } catch (e) {
      setLoadErr(e instanceof Error ? e.message : "Ingest failed");
    } finally {
      setIngestBusy(false);
    }
  }, [ingestTitle, ingestBody, reload]);

  const setupDocuments = account?.documents.filter((d) => isSetupDocumentKind(d.kind)) ?? [];
  const selectedConnection = account?.connections.find((x) => x.id === selectedConn) ?? null;
  const ownerGraphCenterId = account?.user_id ?? "";

  const onCreateTwinAccount = useCallback(async () => {
    const name = setupName.trim();
    if (!name) {
      setSetupErr("Enter the name that should appear on your profile.");
      return;
    }
    setSetupBusy(true);
    setSetupErr(null);
    try {
      await postTwinSetup({
        name,
        email: setupEmail.trim() || undefined,
      });
      await reload();
    } catch (e) {
      setSetupErr(e instanceof Error ? e.message : "Could not create account");
    } finally {
      setSetupBusy(false);
    }
  }, [setupName, setupEmail, reload]);

  return (
    <div className="min-h-[100dvh] bg-[#0A0A0F] text-[#FFFFFF]">
      <header className="sticky top-0 z-20 border-b border-white/[0.06] bg-[#0A0A0F]/90 backdrop-blur-md pt-[max(0.5rem,env(safe-area-inset-top))]">
        <div className="mx-auto flex max-w-5xl flex-col gap-3 px-4 pb-3 pt-1 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <p className="text-[10px] font-medium uppercase tracking-[0.16em] text-[#5F5E5A]">Personal</p>
            <h1 className="truncate text-lg font-medium tracking-[-0.02em]">Fluvio dashboard</h1>
          </div>
          <nav className="scrollbar-none flex shrink-0 flex-wrap items-center gap-1 overflow-x-auto pb-0.5 sm:flex-nowrap sm:justify-end sm:gap-2 sm:pb-0 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            <Link
              href="/"
              className="whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] text-[#888780] hover:bg-white/[0.04] hover:text-white active:bg-white/[0.06] sm:px-2 sm:py-1.5 sm:text-[12px]"
            >
              Home
            </Link>
            <Link
              href="/chat"
              className="whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] text-[#888780] hover:bg-white/[0.04] hover:text-white active:bg-white/[0.06] sm:px-2 sm:py-1.5 sm:text-[12px]"
            >
              Twin chat
            </Link>
            <Link
              href="/graph"
              className="whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] text-[#888780] hover:bg-white/[0.04] hover:text-white active:bg-white/[0.06] sm:px-2 sm:py-1.5 sm:text-[12px]"
            >
              Graph
            </Link>
            <Link
              href="/onboarding"
              className="whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] text-[#7F77DD] hover:bg-white/[0.05] hover:text-[#AFA9EC] active:bg-white/[0.07] sm:px-2 sm:py-1.5 sm:text-[12px]"
            >
              Set up
            </Link>
            <Link
              href="/product"
              className="whitespace-nowrap rounded-lg px-3 py-2.5 text-[13px] text-[#5F5E5A] hover:bg-white/[0.04] hover:text-[#888780] active:bg-white/[0.06] sm:px-2 sm:py-1.5 sm:text-[12px]"
            >
              Product
            </Link>
          </nav>
        </div>
      </header>

      <main className="mx-auto max-w-5xl space-y-6 px-4 pb-[max(2rem,env(safe-area-inset-bottom))] pt-6">
        {sessionOwnerId === "pending" ? (
          <p className="text-[14px] text-[#888780]">Loading…</p>
        ) : !sessionOwnerId ? (
          <section className="rounded-2xl border border-white/[0.08] bg-[#1A1828]/50 p-5 sm:p-8">
            <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-[#5F5E5A]">Not signed in on this device</p>
            <h2 className="mt-2 text-xl font-medium tracking-[-0.03em] text-white sm:text-2xl">
              Create your Fluvio twin to open the dashboard
            </h2>
            <p className="mt-3 max-w-xl text-pretty text-[14px] leading-relaxed text-[#888780]">
              This browser has no saved owner yet (typical in a private window). Create an account on kg-engine here, or go
              through onboarding first—your user id is stored only in this browser as{" "}
              <span className="font-mono text-[12px] text-[#AFA9EC]">twin_owner_id</span>.
            </p>
            <div className="mt-6 grid gap-3 sm:max-w-md">
              <label className="block">
                <span className="text-[11px] font-medium text-[#5F5E5A]">Display name</span>
                <input
                  value={setupName}
                  onChange={(e) => setSetupName(e.target.value)}
                  autoComplete="name"
                  placeholder="Jordan Kim"
                  className="mt-1 w-full rounded-lg border border-white/[0.08] bg-[#0A0A0F] px-3 py-2.5 text-[14px] text-white placeholder:text-[#5F5E5A] focus:outline-none focus:ring-1 focus:ring-[#534AB7]/50"
                />
              </label>
              <label className="block">
                <span className="text-[11px] font-medium text-[#5F5E5A]">Email (optional)</span>
                <input
                  type="email"
                  value={setupEmail}
                  onChange={(e) => setSetupEmail(e.target.value)}
                  autoComplete="email"
                  placeholder="you@company.com"
                  className="mt-1 w-full rounded-lg border border-white/[0.08] bg-[#0A0A0F] px-3 py-2.5 text-[14px] text-white placeholder:text-[#5F5E5A] focus:outline-none focus:ring-1 focus:ring-[#534AB7]/50"
                />
              </label>
              {setupErr ? (
                <p className="text-[13px] text-red-300/90" role="alert">
                  {setupErr}
                </p>
              ) : null}
              <button
                type="button"
                disabled={setupBusy}
                onClick={() => void onCreateTwinAccount()}
                className="rounded-lg bg-[#534AB7] px-4 py-2.5 text-[14px] font-medium text-white transition hover:bg-[#7F77DD] disabled:opacity-45"
              >
                {setupBusy ? "Creating…" : "Create account & NFC card"}
              </button>
            </div>
            <p className="mt-6 text-[13px] text-[#5F5E5A]">
              <Link href="/onboarding" className="text-[#7F77DD] underline-offset-4 hover:text-[#AFA9EC] hover:underline">
                Product onboarding
              </Link>
              <span className="mx-2 text-[#3F3E3A]" aria-hidden>
                ·
              </span>
              <Link href="/" className="text-[#7F77DD] underline-offset-4 hover:text-[#AFA9EC] hover:underline">
                Home
              </Link>
            </p>
          </section>
        ) : null}

        {sessionOwnerId && sessionOwnerId !== "pending" && loadErr ? (
          <div className="rounded-xl border border-red-500/25 bg-red-500/10 px-4 py-3 text-[13px] text-red-200/90">
            <p className="font-medium">Could not load dashboard</p>
            <p className="mt-1 text-red-200/70">
              {loadErr}. Ensure kg-engine is running at{" "}
              <span className="font-mono text-red-100/90">{getKgEngineUrl()}</span> (e.g.{" "}
              <span className="font-mono">cargo run</span>
              ).
            </p>
          </div>
        ) : null}

        {sessionOwnerId && sessionOwnerId !== "pending" && account ? (
          <>
          <motion.section
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            className="rounded-2xl border border-white/[0.06] bg-[#1A1828]/40 p-4 sm:p-5"
          >
            <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0 flex-1">
                <h2 className="text-xl font-medium tracking-[-0.03em]">{account.display_name}</h2>
                <p className="mt-1 text-[14px] text-[#888780]">{account.tagline}</p>
                <p className="mt-2 break-words text-[12px] text-[#5F5E5A]">
                  NFC tap opens your live twin chat at{" "}
                  <span className="break-all font-mono text-[11px] text-[#AFA9EC] sm:text-[12px]">
                    {account.nfc_public_path ??
                      "your public site URL + card id (set when you program the tag or wire the API to return a path)"}
                  </span>
                  . Visitors talk to answers grounded on your graph—they never browse these documents directly.
                </p>
              </div>
              <span className="shrink-0 rounded-full border border-[#534AB7]/40 bg-[#534AB7]/15 px-3 py-1 text-[11px] font-medium text-[#AFA9EC]">
                @{account.owner_slug}
              </span>
            </div>

            <div className="mt-5 rounded-xl border border-white/[0.06] bg-[#0A0A0F]/50 p-3 sm:p-4">
              <h3 className="text-[11px] font-semibold uppercase tracking-[0.14em] text-[#5F5E5A]">
                Account & reachability
              </h3>
              <p className="mt-1 text-[12px] text-[#888780]">
                Email and phone are stored with your kg-engine profile (this server). Used for alerts, OAuth recovery,
                and future “verify tap” loops—not shown on your public NFC page.
              </p>
              <div className="mt-3 grid gap-3 sm:grid-cols-2">
                <label className="block">
                  <span className="text-[11px] font-medium text-[#5F5E5A]">Email</span>
                  <input
                    type="email"
                    autoComplete="email"
                    value={profileEmail}
                    onChange={(e) => setProfileEmail(e.target.value)}
                    placeholder="you@company.com"
                    className="mt-1 w-full rounded-lg border border-white/[0.08] bg-[#0A0A0F] px-3 py-2 text-[14px] text-white placeholder:text-[#5F5E5A] focus:outline-none focus:ring-1 focus:ring-[#534AB7]/50"
                  />
                </label>
                <label className="block">
                  <span className="text-[11px] font-medium text-[#5F5E5A]">Phone</span>
                  <input
                    type="tel"
                    autoComplete="tel"
                    value={profilePhone}
                    onChange={(e) => setProfilePhone(e.target.value)}
                    placeholder="+1 · mobile"
                    className="mt-1 w-full rounded-lg border border-white/[0.08] bg-[#0A0A0F] px-3 py-2 text-[14px] text-white placeholder:text-[#5F5E5A] focus:outline-none focus:ring-1 focus:ring-[#534AB7]/50"
                  />
                </label>
              </div>
              <button
                type="button"
                disabled={profileSaveBusy}
                onClick={() => void onSaveProfile()}
                className="mt-3 rounded-lg bg-[#534AB7] px-4 py-2 text-[13px] font-medium text-white transition hover:bg-[#7F77DD] disabled:opacity-45"
              >
                {profileSaveBusy ? "Saving…" : "Save contact info"}
              </button>
            </div>
          </motion.section>
          <DashboardAppleWallet
            displayName={account.display_name}
            tagline={account.tagline}
            ownerSlug={account.owner_slug}
          />
          </>
        ) : sessionOwnerId && sessionOwnerId !== "pending" && !loadErr ? (
          <p className="text-[14px] text-[#888780]">Loading…</p>
        ) : null}

        {sessionOwnerId && sessionOwnerId !== "pending" && account ? (
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
          <section className="rounded-2xl border border-white/[0.06] bg-[#1A1828]/30 p-4 sm:p-5">
            <h3 className="text-[11px] font-medium uppercase tracking-[0.14em] text-[#5F5E5A]">Ingested documents</h3>
            <p className="mt-1 text-pretty text-[13px] leading-relaxed text-[#888780]">
              Feed your{" "}
              <strong className="font-medium text-[#B4ADEC]/90">identity graph</strong>
              {": "}PDFs (e.g. resume),
              repos, inbox, and notes all become grounded context—so when someone taps your NFC card and chats with your
              twin, answers reflect <span className="text-[#AFA9EC]">who you actually are</span>. Nothing here is exposed
              as raw uploads to visitors; the twin uses it as private structured memory behind your NFC link.
            </p>

            <div className="mt-4 rounded-xl border border-dashed border-[#534AB7]/35 bg-[#534AB7]/[0.04] px-3 py-3 sm:px-4 sm:py-4">
              <h4 className="text-[12px] font-medium text-[#AFA9EC]">Upload resume or PDF</h4>
              <p className="mt-1 text-[12px] leading-relaxed text-[#888780]">
                PDFs run through kg-engine text extraction and resume-style chunking; the row below appears immediately
                while parsing completes on the server.
              </p>
              <input
                ref={pdfRef}
                type="file"
                accept="application/pdf,.pdf"
                className="sr-only"
                onChange={(ev) => {
                  const file = ev.target.files?.[0];
                  ev.target.value = "";
                  if (!file) return;
                  const title = basenamePdf(file.name);
                  void runQuickIngest(`pdf:${file.name}`, {
                    title,
                    body: `PDF uploaded — ${file.name} (${Math.max(1, Math.round(file.size / 1024))} KB). Text extraction → resume-style nodes happens in kg-engine.`,
                    kind: "pdf",
                  });
                }}
              />
              <button
                type="button"
                disabled={!!loadErr || !account || quickBusy?.startsWith("pdf:")}
                onClick={() => pdfRef.current?.click()}
                className="mt-3 w-full rounded-lg border border-[#534AB7]/40 bg-[#1A1828] py-2.5 text-[13px] font-medium text-[#AFA9EC] transition hover:border-[#534AB7]/60 hover:bg-[#232136] disabled:opacity-45 sm:w-auto sm:px-6"
              >
                {quickBusy?.startsWith("pdf:") ? "Uploading…" : "Choose PDF…"}
              </button>
            </div>

            <ul className="mt-5 space-y-2">
              {setupDocuments.map((d) => (
                <li
                  key={d.id}
                  className="rounded-xl border border-white/[0.05] bg-[#0A0A0F]/60 px-3 py-2.5 text-[13px]"
                >
                  <div className="flex items-start justify-between gap-2">
                    <span className="min-w-0 font-medium text-white">{d.title}</span>
                    <span className="shrink-0 rounded bg-white/[0.06] px-1.5 py-0.5 text-[10px] uppercase text-[#888780]">
                      {d.status}
                    </span>
                  </div>
                  <p className="mt-1 text-[12px] leading-relaxed text-[#888780]">{d.excerpt}</p>
                  <p className="mt-1 text-[10px] text-[#5F5E5A]">{d.kind}</p>
                </li>
              ))}
              {setupDocuments.length === 0 ? (
                <li className="rounded-xl border border-white/[0.05] bg-[#0A0A0F]/40 px-3 py-3 text-[12px] text-[#888780]">
                  No setup documents yet. Add a note or upload a PDF.
                </li>
              ) : null}
            </ul>

            <div className="mt-5 border-t border-white/[0.06] pt-4">
              <h4 className="text-[12px] font-medium text-[#AFA9EC]">Quick note</h4>
              <p className="mt-1 text-[11px] text-[#5F5E5A]">
                Stored as kind <span className="font-mono text-[#888780]">note</span> in your ingest list.
              </p>
              <input
                value={ingestTitle}
                onChange={(e) => setIngestTitle(e.target.value)}
                placeholder="Title"
                className="mt-2 w-full rounded-lg border border-white/[0.08] bg-[#0A0A0F] px-3 py-2 text-[14px] text-white placeholder:text-[#5F5E5A] focus:outline-none focus:ring-1 focus:ring-[#534AB7]/50"
              />
              <textarea
                value={ingestBody}
                onChange={(e) => setIngestBody(e.target.value)}
                placeholder="Paste bio, project blurb, or notes…"
                rows={3}
                className="mt-2 w-full resize-none rounded-lg border border-white/[0.08] bg-[#0A0A0F] px-3 py-2 text-[14px] text-white placeholder:text-[#5F5E5A] focus:outline-none focus:ring-1 focus:ring-[#534AB7]/50"
              />
              <button
                type="button"
                disabled={ingestBusy || !account}
                onClick={() => void onIngest()}
                className="mt-3 w-full rounded-lg bg-[#534AB7] py-2.5 text-[13px] font-medium text-white transition hover:bg-[#7F77DD] disabled:opacity-45"
              >
                {ingestBusy ? "Saving…" : "Add note to twin graph"}
              </button>
            </div>
          </section>

          <section className="flex min-h-[280px] flex-col rounded-2xl border border-white/[0.06] bg-[#1A1828]/30 p-4 sm:min-h-[320px] sm:p-5">
            <h3 className="text-[11px] font-medium uppercase tracking-[0.14em] text-[#5F5E5A]">Your network</h3>
            <p className="mt-1 text-[13px] text-[#888780]">Everyone you&apos;ve connected with — tap a name for their ingested slice.</p>
            <div className="mt-3 min-h-[220px] flex-1 overflow-hidden rounded-xl ring-1 ring-[#534AB7]/20 sm:min-h-[260px]">
              {socialNodes.length > 0 ? (
                <DashboardForceGraph
                  nodes={socialNodes}
                  edges={socialEdges}
                  centerId={ownerGraphCenterId || undefined}
                  onNodeClick={(id) => {
                    if (ownerGraphCenterId && id === ownerGraphCenterId) return;
                    void openConnection(id);
                  }}
                />
              ) : (
                <div className="flex h-[260px] items-center justify-center text-[13px] text-[#5F5E5A]">No graph data</div>
              )}
            </div>
          </section>
        </div>
        ) : null}

        {sessionOwnerId && sessionOwnerId !== "pending" && account ? (
          <section className="rounded-2xl border border-white/[0.06] bg-[#1A1828]/30 p-4 sm:p-5">
            <h3 className="text-[11px] font-medium uppercase tracking-[0.14em] text-[#5F5E5A]">Connections</h3>
            <ul className="mt-3 grid gap-2 sm:grid-cols-2">
              {account.connections.map((c) => (
                <li key={c.id}>
                  <button
                    type="button"
                    onClick={() => void openConnection(c.id)}
                    className={`w-full rounded-xl border px-3 py-3 text-left text-[13px] transition ${
                      selectedConn === c.id
                        ? "border-[#534AB7]/60 bg-[#534AB7]/10"
                        : "border-white/[0.06] bg-[#0A0A0F]/50 hover:border-[#534AB7]/35"
                    }`}
                  >
                    <span className="font-medium text-white">{c.name}</span>
                    <span className="mt-0.5 block text-[12px] text-[#888780]">{c.role}</span>
                    <span className="mt-1 block text-[11px] text-[#5F5E5A]">Met: {c.how_we_met}</span>
                  </button>
                </li>
              ))}
            </ul>

            {selectedConn ? (
              <div className="mt-5 border-t border-white/[0.06] pt-4">
                <h4 className="text-[12px] font-medium text-[#AFA9EC]">Ingested context</h4>
                <p className="mt-1 text-[12px] leading-relaxed text-[#888780]">{selectedConnection?.ingested_summary}</p>
                <div className="mt-3 rounded-xl border border-[#534AB7]/35 bg-gradient-to-r from-[#221f3a] to-[#151325] p-3 sm:p-4">
                  <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-[#5F5E5A]">Ask the twin</p>
                  <p className="mt-1 text-[12px] text-[#AFA9EC]">
                    Get a grounded answer about {selectedConnection?.name} based on your ingested context.
                  </p>
                  <Link
                    href={`/chat?topic=${encodeURIComponent(selectedConnection?.name ?? "")}`}
                    className="mt-3 inline-flex w-full items-center justify-center rounded-lg border border-[#7F77DD]/50 bg-[#534AB7]/20 px-3 py-2.5 text-[13px] font-medium text-[#D0CCFF] transition hover:border-[#AFA9EC]/70 hover:bg-[#534AB7]/30 hover:text-white sm:w-auto"
                  >
                    Ask the twin about {selectedConnection?.name}
                  </Link>
                </div>
              </div>
            ) : null}
          </section>
        ) : null}
      </main>
    </div>
  );
}
