"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { motion } from "framer-motion";
import { DashboardAppleWallet } from "./DashboardAppleWallet";
import { DashboardHardwareOrders } from "./DashboardHardwareOrders";
import { DashboardForceGraph } from "./DashboardForceGraph";
import { DashboardPersonalGraph } from "./DashboardPersonalGraph";
import { getKgEngineUrl } from "@/lib/constants";
import {
  fetchAuthMe,
  fetchFluvioAccount,
  fetchFluvioSocialGraph,
  getTwinUserId,
  logoutAuthSession,
  postAuthRequest,
  postAuthVerify,
  postFluvioAccountProfile,
  postFluvioIngest,
  toGraphNodesEdges,
  type FluvioAccount,
} from "@/lib/fluvioDashboardApi";
import type { GraphEdge, GraphNode } from "@/lib/types";

function isSetupDocumentKind(kind: string): boolean {
  const normalized = kind.trim().toLowerCase();
  return normalized === "note" || normalized === "pdf";
}

type AuthStatus = "loading" | "signed_out" | "signed_in";

export function DashboardClient() {
  const [authStatus, setAuthStatus] = useState<AuthStatus>("loading");
  const [account, setAccount] = useState<FluvioAccount | null>(null);
  const [socialNodes, setSocialNodes] = useState<GraphNode[]>([]);
  const [socialEdges, setSocialEdges] = useState<GraphEdge[]>([]);
  const [selectedConn, setSelectedConn] = useState<string | null>(null);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [ingestTitle, setIngestTitle] = useState("");
  const [ingestBody, setIngestBody] = useState("");
  const [ingestBusy, setIngestBusy] = useState(false);
  const [profileEmail, setProfileEmail] = useState("");
  const [profilePhone, setProfilePhone] = useState("");
  const [profileSaveBusy, setProfileSaveBusy] = useState(false);

  const [loginEmail, setLoginEmail] = useState("");
  const [loginName, setLoginName] = useState("");
  const [loginCode, setLoginCode] = useState("");
  const [loginStep, setLoginStep] = useState<"email" | "code">("email");
  const [loginErr, setLoginErr] = useState<string | null>(null);
  const [loginBusy, setLoginBusy] = useState(false);
  const [demoCodeHint, setDemoCodeHint] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setAuthStatus("loading");
    setLoadErr(null);
    const ac = new AbortController();
    const t = window.setTimeout(() => ac.abort(), 25_000);
    try {
      const me = await fetchAuthMe(ac.signal);
      if (!me) {
        setAccount(null);
        setSocialNodes([]);
        setSocialEdges([]);
        setAuthStatus("signed_out");
        return;
      }
      const [acc, soc] = await Promise.all([
        fetchFluvioAccount(ac.signal),
        fetchFluvioSocialGraph(ac.signal),
      ]);
      setAccount(acc);
      const g = toGraphNodesEdges(soc);
      setSocialNodes(g.nodes);
      setSocialEdges(g.edges);
      setAuthStatus("signed_in");
    } catch (e) {
      const aborted =
        (e instanceof DOMException && e.name === "AbortError") ||
        (e instanceof Error && e.name === "AbortError");
      setLoadErr(
        aborted
          ? `No response from ${getKgEngineUrl()}. Start the backend, then refresh.`
          : e instanceof Error
            ? e.message
            : "Something went wrong loading this page.",
      );
      setAccount(null);
      setSocialNodes([]);
      setSocialEdges([]);
      setAuthStatus("signed_out");
    } finally {
      window.clearTimeout(t);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

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
  const sessionOwnerId =
    authStatus === "signed_in" ? (account?.user_id ?? getTwinUserId()) : null;

  const onRequestLoginCode = useCallback(async () => {
    const email = loginEmail.trim().toLowerCase();
    if (!email.includes("@")) {
      setLoginErr("Enter a valid email.");
      return;
    }
    setLoginBusy(true);
    setLoginErr(null);
    setDemoCodeHint(null);
    try {
      const r = await postAuthRequest({
        email,
        name: loginName.trim() || undefined,
      });
      setLoginStep("code");
      if (r.code) setDemoCodeHint(r.code);
    } catch (e) {
      setLoginErr(e instanceof Error ? e.message : "Could not send code");
    } finally {
      setLoginBusy(false);
    }
  }, [loginEmail, loginName]);

  const onVerifyLogin = useCallback(async () => {
    const email = loginEmail.trim().toLowerCase();
    const code = loginCode.trim();
    if (!email.includes("@") || !code) {
      setLoginErr("Email and code are required.");
      return;
    }
    setLoginBusy(true);
    setLoginErr(null);
    try {
      await postAuthVerify(email, code);
      setLoginCode("");
      setDemoCodeHint(null);
      await reload();
    } catch (e) {
      setLoginErr(e instanceof Error ? e.message : "Invalid or expired code");
    } finally {
      setLoginBusy(false);
    }
  }, [loginEmail, loginCode, reload]);

  const onSignOut = useCallback(async () => {
    await logoutAuthSession();
    setLoginStep("email");
    setLoginErr(null);
    setDemoCodeHint(null);
    await reload();
  }, [reload]);

  return (
    <div className="min-h-[100dvh] bg-zinc-950 text-white antialiased">
      <header className="sticky top-0 z-20 border-b border-white/[0.06] bg-zinc-950/80 backdrop-blur-xl pt-[max(0.75rem,env(safe-area-inset-top))]">
        <div className="mx-auto flex max-w-5xl flex-col gap-4 px-5 pb-4 pt-2 sm:flex-row sm:items-end sm:justify-between sm:px-8">
          <div className="min-w-0">
            <p className="text-[13px] font-medium text-zinc-500">FluvioMe</p>
            <h1 className="mt-0.5 text-[1.75rem] font-semibold tracking-[-0.03em] text-white sm:text-[2rem]">Overview</h1>
          </div>
          <nav className="scrollbar-none flex shrink-0 flex-wrap items-center gap-1 sm:justify-end sm:gap-0 sm:pb-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            <Link
              href="/"
              className="whitespace-nowrap rounded-full px-3 py-2 text-[15px] text-zinc-400 transition hover:bg-white/[0.06] hover:text-white"
            >
              Home
            </Link>
            <Link
              href="/chat"
              className="whitespace-nowrap rounded-full px-3 py-2 text-[15px] text-zinc-400 transition hover:bg-white/[0.06] hover:text-white"
            >
              Chat
            </Link>
            <Link
              href="/graph"
              className="whitespace-nowrap rounded-full px-3 py-2 text-[15px] text-zinc-400 transition hover:bg-white/[0.06] hover:text-white"
            >
              Map
            </Link>
            <Link
              href="/onboarding"
              className="whitespace-nowrap rounded-full px-3 py-2 text-[15px] font-medium text-violet-400 transition hover:bg-violet-500/10 hover:text-violet-300"
            >
              Cards
            </Link>
            {authStatus === "signed_in" ? (
              <button
                type="button"
                onClick={() => void onSignOut()}
                className="whitespace-nowrap rounded-full px-3 py-2 text-[15px] text-zinc-400 transition hover:bg-white/[0.06] hover:text-white"
              >
                Sign out
              </button>
            ) : null}
          </nav>
        </div>
      </header>

      <main className="mx-auto max-w-5xl space-y-10 px-5 pb-[max(2.5rem,env(safe-area-inset-bottom))] pt-10 sm:px-8">
        <DashboardHardwareOrders sessionOwnerId={sessionOwnerId} />

        {authStatus === "signed_in" ? (
          <DashboardPersonalGraph
            locked={!account}
            onDone={() => void reload()}
            onError={(msg) => setLoadErr(msg)}
          />
        ) : null}

        {authStatus === "loading" ? (
          <p className="text-[15px] text-zinc-500">Loading…</p>
        ) : authStatus === "signed_out" ? (
          <section className="rounded-[20px] border border-white/[0.06] bg-white/[0.02] p-8 sm:p-10">
            <h2 className="text-2xl font-semibold tracking-[-0.03em] text-white sm:text-[1.75rem]">Sign in</h2>
            <p className="mt-3 max-w-md text-pretty text-[15px] leading-relaxed text-zinc-500">
              We’ll email you a one-time code. New here? We create your account when you verify your email.
            </p>
            <div className="mt-8 grid max-w-md gap-4">
              <label className="block">
                <span className="text-[13px] font-medium text-zinc-500">Email</span>
                <input
                  type="email"
                  value={loginEmail}
                  onChange={(e) => setLoginEmail(e.target.value)}
                  autoComplete="email"
                  disabled={loginStep === "code"}
                  placeholder="you@email.com"
                  className="mt-1.5 w-full rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[16px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/30 disabled:opacity-50"
                />
              </label>
              {loginStep === "email" ? (
                <label className="block">
                  <span className="text-[13px] font-medium text-zinc-500">Name · optional (first-time only)</span>
                  <input
                    value={loginName}
                    onChange={(e) => setLoginName(e.target.value)}
                    autoComplete="name"
                    placeholder="Jordan Kim"
                    className="mt-1.5 w-full rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[16px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/30"
                  />
                </label>
              ) : null}
              {loginStep === "code" ? (
                <label className="block">
                  <span className="text-[13px] font-medium text-zinc-500">Code from email</span>
                  <input
                    value={loginCode}
                    onChange={(e) => setLoginCode(e.target.value)}
                    inputMode="numeric"
                    autoComplete="one-time-code"
                    placeholder="123456"
                    className="mt-1.5 w-full rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[16px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/30"
                  />
                </label>
              ) : null}
              {demoCodeHint ? (
                <p className="text-[14px] text-amber-200/90" role="status">
                  Demo mode — your code: <span className="font-mono font-semibold">{demoCodeHint}</span>
                </p>
              ) : null}
              {loginErr ? (
                <p className="text-[14px] text-red-400/90" role="alert">
                  {loginErr}
                </p>
              ) : null}
              {loginStep === "email" ? (
                <button
                  type="button"
                  disabled={loginBusy}
                  onClick={() => void onRequestLoginCode()}
                  className="mt-1 rounded-full bg-white px-5 py-3.5 text-[16px] font-semibold text-zinc-950 transition hover:bg-zinc-100 disabled:opacity-40"
                >
                  {loginBusy ? "Sending…" : "Send login code"}
                </button>
              ) : (
                <div className="flex flex-wrap gap-3">
                  <button
                    type="button"
                    disabled={loginBusy}
                    onClick={() => void onVerifyLogin()}
                    className="rounded-full bg-white px-5 py-3.5 text-[16px] font-semibold text-zinc-950 transition hover:bg-zinc-100 disabled:opacity-40"
                  >
                    {loginBusy ? "Signing in…" : "Sign in"}
                  </button>
                  <button
                    type="button"
                    disabled={loginBusy}
                    onClick={() => {
                      setLoginStep("email");
                      setLoginCode("");
                      setLoginErr(null);
                      setDemoCodeHint(null);
                    }}
                    className="rounded-full border border-white/[0.12] px-5 py-3.5 text-[16px] font-medium text-zinc-300 transition hover:bg-white/[0.06]"
                  >
                    Edit email
                  </button>
                </div>
              )}
            </div>
            <p className="mt-8 text-[15px] text-zinc-600">
              <Link href="/onboarding" className="font-medium text-violet-400 underline-offset-4 hover:text-violet-300 hover:underline">
                Order a tap card
              </Link>
              <span className="mx-2 text-zinc-700" aria-hidden>
                ·
              </span>
              <Link href="/" className="text-zinc-500 underline-offset-4 hover:text-zinc-400 hover:underline">
                FluvioMe home
              </Link>
            </p>
          </section>
        ) : null}

        {authStatus === "signed_in" && loadErr ? (
          <div className="rounded-[16px] border border-red-500/20 bg-red-500/[0.07] px-5 py-4 text-[15px] text-red-200/95">
            <p className="font-medium text-red-100">Can’t load your profile</p>
            <p className="mt-2 leading-relaxed text-red-200/75">{loadErr}</p>
            <p className="mt-2 font-mono text-[12px] text-red-300/70">{getKgEngineUrl()}</p>
          </div>
        ) : null}

        {authStatus === "signed_in" && account ? (
          <>
          <motion.section
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            className="rounded-[20px] border border-white/[0.06] bg-white/[0.02] p-8 sm:p-9"
          >
            <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
              <div className="min-w-0 flex-1">
                <h2 className="text-[1.65rem] font-semibold tracking-[-0.03em] text-white">{account.display_name}</h2>
                {account.tagline ? <p className="mt-2 text-[16px] text-zinc-500">{account.tagline}</p> : null}
                <p className="mt-4 text-[14px] leading-relaxed text-zinc-600">
                  Your tap opens chat for visitors at{" "}
                  <span className="break-all font-mono text-[13px] text-zinc-400">
                    {account.nfc_public_path ?? "— set when your tag is linked"}
                  </span>
                </p>
              </div>
              <span className="shrink-0 rounded-full border border-white/[0.08] bg-white/[0.04] px-3.5 py-1.5 text-[13px] font-medium text-zinc-400">
                @{account.owner_slug}
              </span>
            </div>

            <div className="mt-8 border-t border-white/[0.06] pt-8">
              <h3 className="text-[17px] font-semibold tracking-[-0.02em] text-white">Reach you</h3>
              <p className="mt-1 max-w-lg text-[15px] leading-relaxed text-zinc-500">
                Private. Not shown on your tap page.
              </p>
              <div className="mt-5 grid gap-4 sm:grid-cols-2">
                <label className="block">
                  <span className="text-[13px] font-medium text-zinc-500">Email</span>
                  <input
                    type="email"
                    autoComplete="email"
                    value={profileEmail}
                    onChange={(e) => setProfileEmail(e.target.value)}
                    placeholder="you@company.com"
                    className="mt-1.5 w-full rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[16px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/30"
                  />
                </label>
                <label className="block">
                  <span className="text-[13px] font-medium text-zinc-500">Phone</span>
                  <input
                    type="tel"
                    autoComplete="tel"
                    value={profilePhone}
                    onChange={(e) => setProfilePhone(e.target.value)}
                    placeholder="+1 …"
                    className="mt-1.5 w-full rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[16px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/30"
                  />
                </label>
              </div>
              <button
                type="button"
                disabled={profileSaveBusy}
                onClick={() => void onSaveProfile()}
                className="mt-5 rounded-full border border-white/[0.12] bg-transparent px-5 py-2.5 text-[15px] font-medium text-white transition hover:bg-white/[0.06] disabled:opacity-40"
              >
                {profileSaveBusy ? "Saving…" : "Save"}
              </button>
            </div>
          </motion.section>
          <DashboardAppleWallet
            displayName={account.display_name}
            tagline={account.tagline}
            ownerSlug={account.owner_slug}
          />
          </>
        ) : authStatus === "signed_in" && !loadErr ? (
          <p className="text-[15px] text-zinc-500">Loading…</p>
        ) : null}

        {authStatus === "signed_in" && account ? (
        <div className="grid grid-cols-1 gap-8 lg:grid-cols-2">
          <section
            id="dashboard-note"
            className="scroll-mt-28 rounded-[20px] border border-white/[0.06] bg-white/[0.02] p-6 sm:p-8"
          >
            <h3 className="text-[17px] font-semibold tracking-[-0.02em] text-white">What you’ve shared</h3>
            <p className="mt-2 max-w-sm text-[15px] leading-relaxed text-zinc-500">
              Notes you save here attach to your tap profile. Heavy PDFs and video belong in Personal graph near the top of this page.
            </p>

            <ul className="mt-6 space-y-2">
              {setupDocuments.map((d) => (
                <li
                  key={d.id}
                  className="rounded-xl border border-white/[0.06] bg-zinc-950/50 px-4 py-3.5"
                >
                  <div className="flex items-start justify-between gap-2">
                    <span className="min-w-0 text-[15px] font-medium text-white">{d.title}</span>
                    <span className="shrink-0 rounded-full bg-white/[0.06] px-2 py-0.5 text-[11px] font-medium text-zinc-500">
                      {d.status}
                    </span>
                  </div>
                  <p className="mt-1 text-[14px] leading-relaxed text-zinc-500">{d.excerpt}</p>
                </li>
              ))}
              {setupDocuments.length === 0 ? (
                <li className="rounded-xl border border-white/[0.05] bg-zinc-950/30 px-4 py-4 text-[15px] text-zinc-600">
                  Nothing here yet.
                </li>
              ) : null}
            </ul>

            <div className="mt-8 border-t border-white/[0.06] pt-8">
              <h4 className="text-[15px] font-medium text-white">Note</h4>
              <input
                value={ingestTitle}
                onChange={(e) => setIngestTitle(e.target.value)}
                placeholder="Title"
                className="mt-3 w-full rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[16px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/30"
              />
              <textarea
                value={ingestBody}
                onChange={(e) => setIngestBody(e.target.value)}
                placeholder="Write something your avatar should know…"
                rows={3}
                className="mt-3 w-full resize-none rounded-xl border border-white/[0.08] bg-zinc-950 px-4 py-3 text-[16px] text-white placeholder:text-zinc-600 focus:outline-none focus:ring-2 focus:ring-violet-500/30"
              />
              <button
                type="button"
                disabled={ingestBusy || !account}
                onClick={() => void onIngest()}
                className="mt-4 w-full rounded-full bg-white py-3.5 text-[16px] font-semibold text-zinc-950 transition hover:bg-zinc-100 disabled:opacity-40"
              >
                {ingestBusy ? "Saving…" : "Save note"}
              </button>
            </div>
          </section>

          <section className="flex min-h-[280px] flex-col rounded-[20px] border border-white/[0.06] bg-white/[0.02] p-6 sm:min-h-[320px] sm:p-8">
            <h3 className="text-[17px] font-semibold tracking-[-0.02em] text-white">Your circle</h3>
            <p className="mt-2 text-[15px] text-zinc-500">Tap someone below the map to see more.</p>
            <div className="mt-4 min-h-[220px] flex-1 overflow-hidden rounded-2xl ring-1 ring-white/[0.06] sm:min-h-[260px]">
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
                <div className="flex h-[260px] items-center justify-center text-[15px] text-zinc-600">No one yet</div>
              )}
            </div>
          </section>
        </div>
        ) : null}

        {authStatus === "signed_in" && account ? (
          <section className="rounded-[20px] border border-white/[0.06] bg-white/[0.02] p-6 sm:p-8">
            <h3 className="text-[17px] font-semibold tracking-[-0.02em] text-white">People</h3>
            <ul className="mt-5 grid gap-3 sm:grid-cols-2">
              {account.connections.map((c) => (
                <li key={c.id}>
                  <button
                    type="button"
                    onClick={() => void openConnection(c.id)}
                    className={`w-full rounded-2xl border px-4 py-4 text-left transition ${
                      selectedConn === c.id
                        ? "border-violet-500/40 bg-violet-500/[0.08]"
                        : "border-white/[0.06] bg-zinc-950/40 hover:border-white/[0.12]"
                    }`}
                  >
                    <span className="text-[16px] font-medium text-white">{c.name}</span>
                    <span className="mt-0.5 block text-[14px] text-zinc-500">{c.role}</span>
                    <span className="mt-1.5 block text-[13px] text-zinc-600">{c.how_we_met}</span>
                  </button>
                </li>
              ))}
            </ul>

            {selectedConn ? (
              <div className="mt-8 border-t border-white/[0.06] pt-8">
                <h4 className="text-[15px] font-medium text-white">About {selectedConnection?.name}</h4>
                <p className="mt-2 text-[15px] leading-relaxed text-zinc-500">{selectedConnection?.ingested_summary}</p>
                <div className="mt-5 rounded-2xl border border-white/[0.08] bg-zinc-950/60 p-5 sm:p-6">
                  <p className="text-[15px] text-zinc-400">
                    Ask your avatar anything you saved about them.
                  </p>
                  <Link
                    href={`/chat?topic=${encodeURIComponent(selectedConnection?.name ?? "")}`}
                    className="mt-4 inline-flex w-full items-center justify-center rounded-full bg-white py-3.5 text-[16px] font-semibold text-zinc-950 transition hover:bg-zinc-100 sm:w-auto sm:px-8"
                  >
                    Open chat
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
