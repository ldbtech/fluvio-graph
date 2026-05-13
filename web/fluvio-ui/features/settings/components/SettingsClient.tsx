"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { AuthedProfileHeader } from "@/app/components/AuthedProfileHeader";
import {
  fetchFluvioAccount,
  postFluvioAccountProfile,
} from "@/shared/lib/fluvioDashboardApi";

type Zone = 1 | 2 | 3;

type MockConnection = {
  id: string;
  name: string;
  email?: string;
  zone: Zone;
  canSeeWorkspace: boolean;
  canSeeNotes: boolean;
  canSeeUploads: boolean;
};

export function SettingsClient() {
  const [connections, setConnections] = useState<MockConnection[]>([
    {
      id: "demo-1",
      name: "Example contact",
      email: "friend@example.com",
      zone: 2,
      canSeeWorkspace: true,
      canSeeNotes: true,
      canSeeUploads: false,
    },
  ]);

  const [deleteConfirm, setDeleteConfirm] = useState("");
  const [nfcCode, setNfcCode] = useState("");
  const [activeTab, setActiveTab] = useState<"privacy" | "account" | "nfc">("privacy");

  const [contactEmail, setContactEmail] = useState("");
  const [contactPhone, setContactPhone] = useState("");
  const [contactLoading, setContactLoading] = useState(true);
  const [contactLoadErr, setContactLoadErr] = useState<string | null>(null);
  const [contactSaveBusy, setContactSaveBusy] = useState(false);
  const [contactSaveErr, setContactSaveErr] = useState<string | null>(null);
  const [contactSaveOk, setContactSaveOk] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      setContactLoading(true);
      setContactLoadErr(null);
      try {
        const acc = await fetchFluvioAccount();
        if (cancelled) return;
        setContactEmail(acc.email ?? "");
        setContactPhone(acc.phone ?? "");
      } catch (e) {
        if (!cancelled) {
          setContactLoadErr(e instanceof Error ? e.message : "Could not load account");
        }
      } finally {
        if (!cancelled) setContactLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const onSaveContact = useCallback(async () => {
    setContactSaveBusy(true);
    setContactSaveErr(null);
    setContactSaveOk(false);
    try {
      await postFluvioAccountProfile({
        email: contactEmail.trim(),
        phone: contactPhone.trim(),
      });
      setContactSaveOk(true);
    } catch (e) {
      setContactSaveErr(e instanceof Error ? e.message : "Could not save");
    } finally {
      setContactSaveBusy(false);
    }
  }, [contactEmail, contactPhone]);

  useEffect(() => {
    setContactSaveOk(false);
  }, [contactEmail, contactPhone]);

  const updateConnection = (id: string, patch: Partial<MockConnection>) => {
    setConnections((prev) =>
      prev.map((c) => (c.id === id ? { ...c, ...patch } : c))
    );
  };

  const zoneConfig: Record<Zone, { dot: string; label: string; pill: string }> = {
    1: {
      dot: "bg-zinc-400",
      label: "Only me",
      pill: "border-zinc-700/60 bg-zinc-800/60 text-zinc-300",
    },
    2: {
      dot: "bg-emerald-400",
      label: "Trusted",
      pill: "border-emerald-800/50 bg-emerald-950/60 text-emerald-300",
    },
    3: {
      dot: "bg-violet-400",
      label: "Extended",
      pill: "border-violet-800/50 bg-violet-950/60 text-violet-300",
    },
  };

  const tabs = [
    { id: "privacy" as const, label: "Privacy & zones" },
    { id: "account" as const, label: "Account" },
    { id: "nfc" as const, label: "NFC card" },
  ];

  const inputClass =
    "w-full rounded-xl border border-zinc-800 bg-zinc-950/80 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 outline-none transition-colors focus:border-zinc-600 focus:bg-zinc-950";

  return (
    <div className="min-h-screen bg-[#080808] text-zinc-100 antialiased">
      {/* Top shimmer line */}
      <div className="pointer-events-none fixed inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      <div className="relative mx-auto w-full max-w-[680px] px-5 pb-32 pt-14 sm:px-6">

        {/* ── Header ── */}
        <div className="mb-10 flex items-start justify-between gap-4">
          <div>
            <Link
              href="/dashboard"
              className="mb-3 inline-flex items-center gap-1.5 text-[11px] font-medium text-zinc-600 transition-colors hover:text-zinc-300"
            >
              <svg width="11" height="11" viewBox="0 0 11 11" fill="none">
                <path d="M7 1.5L3 5.5L7 9.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              Dashboard
            </Link>
            <h1 className="text-[1.85rem] font-semibold tracking-[-0.045em] text-white">
              Settings
            </h1>
            <p className="mt-1 text-sm text-zinc-500">
              Privacy, account, and NFC management.
            </p>
          </div>
          <div className="mt-1 flex shrink-0 items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900/70 px-3 py-1.5 text-[11px] font-medium text-zinc-500">
            <span className="h-1.5 w-1.5 rounded-full bg-amber-400" />
            Preview
          </div>
        </div>

        <div className="mb-8 border-b border-zinc-800/50 pb-8">
          <AuthedProfileHeader className="w-full max-w-full px-0 py-0" />
        </div>

        {/* ── Tab bar ── */}
        <div className="mb-7 flex items-center gap-0.5 rounded-xl border border-zinc-800/50 bg-zinc-900/30 p-1">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex-1 rounded-[10px] px-4 py-2 text-[13px] font-medium transition-all duration-150 ${
                activeTab === tab.id
                  ? "bg-white/[0.07] text-white"
                  : "text-zinc-500 hover:text-zinc-300"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* ═══════════════════════════
            TAB: Privacy & Zones
        ═══════════════════════════ */}
        {activeTab === "privacy" && (
          <div className="space-y-3">

            {/* Zone legend */}
            <div className="grid grid-cols-3 gap-2 mb-5">
              {([1, 2, 3] as Zone[]).map((z) => (
                <div
                  key={z}
                  className="flex items-center gap-2.5 rounded-xl border border-zinc-800/50 bg-zinc-900/30 px-3.5 py-3"
                >
                  <span className={`h-2 w-2 shrink-0 rounded-full ${zoneConfig[z].dot}`} />
                  <div>
                    <p className="text-[12px] font-semibold text-zinc-200">Zone {z}</p>
                    <p className="text-[11px] text-zinc-600">{zoneConfig[z].label}</p>
                  </div>
                </div>
              ))}
            </div>

            {/* Connections list */}
            {connections.length === 0 ? (
              <div className="flex flex-col items-center gap-3 rounded-2xl border border-dashed border-zinc-800 py-14 text-center">
                <div className="flex h-9 w-9 items-center justify-center rounded-full border border-zinc-800 bg-zinc-900 text-zinc-500">
                  <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
                    <path d="M7 2v10M2 7h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                  </svg>
                </div>
                <div>
                  <p className="text-sm font-medium text-zinc-300">No connections yet</p>
                  <p className="mt-0.5 text-xs text-zinc-600">Tap an NFC card to add someone.</p>
                </div>
              </div>
            ) : (
              connections.map((c) => (
                <div
                  key={c.id}
                  className="rounded-2xl border border-zinc-800/50 bg-zinc-900/25 p-5 transition-all hover:border-zinc-700/60 hover:bg-zinc-900/40"
                >
                  {/* Contact row */}
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-3">
                      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-[11px] font-semibold text-zinc-200">
                        {c.name.slice(0, 2).toUpperCase()}
                      </div>
                      <div>
                        <p className="text-[13px] font-semibold leading-none text-zinc-100">{c.name}</p>
                        {c.email && (
                          <p className="mt-0.5 text-[11px] text-zinc-500">{c.email}</p>
                        )}
                      </div>
                    </div>

                    {/* Zone picker */}
                    <select
                      className={`rounded-full border px-3 py-1 text-[11px] font-semibold outline-none cursor-pointer transition-colors ${zoneConfig[c.zone].pill}`}
                      value={c.zone}
                      onChange={(e) =>
                        updateConnection(c.id, { zone: Number(e.target.value) as Zone })
                      }
                    >
                      <option value={1}>Zone 1 · Only me</option>
                      <option value={2}>Zone 2 · Trusted</option>
                      <option value={3}>Zone 3 · Extended</option>
                    </select>
                  </div>

                  {/* Divider */}
                  <div className="my-4 h-px bg-zinc-800/60" />

                  {/* Permission toggles */}
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="mr-1 text-[11px] text-zinc-600">Visibility:</span>
                    {(
                      [
                        { key: "canSeeWorkspace" as keyof MockConnection, label: "Workspace" },
                        { key: "canSeeNotes" as keyof MockConnection, label: "Notes" },
                        { key: "canSeeUploads" as keyof MockConnection, label: "Uploads" },
                      ]
                    ).map(({ key, label }) => {
                      const on = c[key] as boolean;
                      return (
                        <button
                          key={key}
                          onClick={() => updateConnection(c.id, { [key]: !on })}
                          className={`rounded-full border px-3 py-1 text-[11px] font-medium transition-all duration-150 ${
                            on
                              ? "border-emerald-800/50 bg-emerald-950/60 text-emerald-300"
                              : "border-zinc-800/60 bg-transparent text-zinc-600 hover:text-zinc-400 hover:border-zinc-700"
                          }`}
                        >
                          {on && (
                            <span className="mr-1">✓</span>
                          )}
                          {label}
                        </button>
                      );
                    })}
                  </div>

                  {/* Footer */}
                  <div className="mt-4 flex items-center justify-end">
                    <button
                      disabled
                      className="rounded-lg border border-zinc-800 bg-zinc-900/50 px-3 py-1.5 text-[11px] font-medium text-zinc-600 cursor-not-allowed"
                    >
                      Save — coming soon
                    </button>
                  </div>
                </div>
              ))
            )}

            {/* Add connection CTA */}
            <button className="flex w-full items-center justify-center gap-2 rounded-2xl border border-dashed border-zinc-800 py-3.5 text-xs font-medium text-zinc-600 transition-all hover:border-zinc-700 hover:text-zinc-400">
              <svg width="13" height="13" viewBox="0 0 13 13" fill="none">
                <path d="M6.5 1.5v10M1.5 6.5h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
              Add connection
            </button>
          </div>
        )}

        {/* ═══════════════════════════
            TAB: Account
        ═══════════════════════════ */}
        {activeTab === "account" && (
          <div className="space-y-4">
            <div className="rounded-2xl border border-zinc-800/50 bg-zinc-900/25 p-5">
              <p className="mb-1 text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
                Contact · private
              </p>
              <p className="mb-4 text-[13px] leading-relaxed text-zinc-500">
                Used for follow-ups and handoffs. Not shown on your public tap page.
              </p>
              {contactLoading ? (
                <p className="text-sm text-zinc-500">Loading…</p>
              ) : contactLoadErr ? (
                <p className="rounded-xl border border-red-500/25 bg-red-500/[0.08] px-3 py-2.5 text-[13px] text-red-300/95">
                  {contactLoadErr}
                </p>
              ) : (
                <>
                  <div className="space-y-3">
                    <div>
                      <label className="mb-1.5 block text-xs font-medium text-zinc-400">Email</label>
                      <input
                        type="email"
                        autoComplete="email"
                        className={inputClass}
                        placeholder="you@company.com"
                        value={contactEmail}
                        onChange={(e) => setContactEmail(e.target.value)}
                      />
                    </div>
                    <div>
                      <label className="mb-1.5 block text-xs font-medium text-zinc-400">Phone</label>
                      <input
                        type="tel"
                        autoComplete="tel"
                        className={inputClass}
                        placeholder="+1 …"
                        value={contactPhone}
                        onChange={(e) => setContactPhone(e.target.value)}
                      />
                    </div>
                  </div>
                  <div className="mt-4 flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-3">
                    <button
                      type="button"
                      disabled={contactSaveBusy}
                      onClick={() => void onSaveContact()}
                      className="rounded-xl border border-violet-600/50 bg-violet-600/90 px-4 py-2.5 text-xs font-semibold text-white transition hover:bg-violet-500 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                      {contactSaveBusy ? "Saving…" : "Save contact"}
                    </button>
                    {contactSaveOk ? (
                      <span className="text-[11px] font-medium text-emerald-400/95">Saved.</span>
                    ) : null}
                  </div>
                  {contactSaveErr ? (
                    <p className="mt-3 text-[12px] text-red-400/95">{contactSaveErr}</p>
                  ) : null}
                </>
              )}
            </div>

            {/* Active sessions */}
            <div className="rounded-2xl border border-zinc-800/50 bg-zinc-900/25 p-5">
              <p className="mb-4 text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
                Sessions
              </p>
              <div className="space-y-2">
                {[
                  { label: "MacBook Pro · Chrome", meta: "Montréal, CA · Active now", current: true },
                  { label: "iPhone 15 · Safari", meta: "Montréal, CA · 2h ago", current: false },
                ].map((s) => (
                  <div
                    key={s.label}
                    className="flex items-center justify-between rounded-xl border border-zinc-800/40 bg-zinc-950/40 px-4 py-3"
                  >
                    <div>
                      <p className="text-[13px] font-medium text-zinc-200">{s.label}</p>
                      <p className="text-[11px] text-zinc-600">{s.meta}</p>
                    </div>
                    {s.current ? (
                      <span className="rounded-full border border-emerald-800/40 bg-emerald-950/50 px-2.5 py-0.5 text-[10px] font-semibold text-emerald-400">
                        This device
                      </span>
                    ) : (
                      <button className="text-[11px] font-medium text-zinc-600 transition-colors hover:text-red-400">
                        Revoke
                      </button>
                    )}
                  </div>
                ))}
              </div>
            </div>

            {/* Danger zone */}
            <div className="rounded-2xl border border-red-950/50 bg-[#0d0505] p-5">
              <p className="mb-1 text-[10px] font-semibold uppercase tracking-widest text-red-800">
                Danger zone
              </p>
              <p className="mb-4 text-xs text-zinc-600">
                Permanently removes your graph, uploads, and NFC links. Cannot be undone.
              </p>
              <div className="space-y-2.5">
                <div>
                  <label className="mb-1.5 block text-xs text-zinc-600">
                    Type <code className="font-mono text-red-700/80">delete my account</code> to confirm
                  </label>
                  <input
                    type="text"
                    className="w-full rounded-xl border border-red-950/60 bg-zinc-950/80 px-4 py-2.5 text-sm text-red-200 placeholder:text-red-950 outline-none focus:border-red-900/70 transition-colors"
                    placeholder="delete my account"
                    value={deleteConfirm}
                    onChange={(e) => setDeleteConfirm(e.target.value)}
                  />
                </div>
                <button
                  disabled={deleteConfirm.trim() !== "delete my account"}
                  className={`w-full rounded-xl border py-2.5 text-xs font-semibold transition-all ${
                    deleteConfirm.trim() === "delete my account"
                      ? "border-red-800/60 bg-red-950/70 text-red-300 hover:bg-red-900/60"
                      : "border-zinc-800/40 bg-transparent text-zinc-700 cursor-not-allowed"
                  }`}
                >
                  Delete account permanently
                </button>
              </div>
            </div>
          </div>
        )}

        {/* ═══════════════════════════
            TAB: NFC Card
        ═══════════════════════════ */}
        {activeTab === "nfc" && (
          <div className="space-y-4">

            {/* Card visual */}
            <div className="relative overflow-hidden rounded-2xl border border-zinc-800/50 bg-zinc-900/30 p-8">
              {/* Decorative rings */}
              <div className="pointer-events-none absolute -right-12 -top-12 h-40 w-40 rounded-full border border-white/[0.04]" />
              <div className="pointer-events-none absolute -right-6 -top-6 h-24 w-24 rounded-full border border-white/[0.03]" />
              <div className="pointer-events-none absolute bottom-0 left-0 h-32 w-32 -translate-x-1/2 translate-y-1/2 rounded-full border border-white/[0.02]" />

              <div className="relative flex flex-col gap-6">
                <div className="flex items-center justify-between">
                  {/* NFC icon */}
                  <div className="flex h-10 w-10 items-center justify-center rounded-full border border-zinc-700/60 bg-zinc-800/60">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                      <path d="M8.5 8.5C9.8 7.2 11.3 6.5 12 6.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
                      <path d="M6 6C8.1 3.9 10.1 3 12 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
                      <path d="M15.5 8.5C14.2 7.2 12.7 6.5 12 6.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
                      <path d="M18 6C15.9 3.9 13.9 3 12 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
                      <circle cx="12" cy="14" r="3" stroke="currentColor" strokeWidth="1.5"/>
                      <path d="M12 17v4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
                    </svg>
                  </div>
                  <span className="text-[10px] font-semibold tracking-[0.2em] uppercase text-zinc-700">
                    Fluvio · NFC
                  </span>
                </div>
                <div>
                  <p className="text-lg font-semibold tracking-tight text-zinc-100">
                    Your identity card
                  </p>
                  <p className="mt-1 text-xs text-zinc-600">
                    Tap to exchange graphs with anyone.
                  </p>
                </div>
              </div>
            </div>

            {/* Activate card */}
            <div className="rounded-2xl border border-zinc-800/50 bg-zinc-900/25 p-5">
              <p className="mb-1 text-sm font-semibold text-zinc-100">Activate a card</p>
              <p className="mb-4 text-xs text-zinc-600">
                Scan or paste the UID printed on your card to link it to this account.
              </p>
              <div className="space-y-2.5">
                <input
                  type="text"
                  className={`${inputClass} font-mono tracking-wider`}
                  placeholder="04:A1:B2:C3:D4:E5:F6"
                  value={nfcCode}
                  onChange={(e) => setNfcCode(e.target.value)}
                />
                <button
                  disabled
                  className="w-full rounded-xl border border-zinc-800/50 bg-zinc-900/50 py-2.5 text-xs font-semibold text-zinc-600 cursor-not-allowed opacity-60"
                >
                  Activate card — coming soon
                </button>
              </div>
            </div>

            {/* How it works */}
            <div className="rounded-2xl border border-zinc-800/50 bg-zinc-900/25 p-5">
              <p className="mb-5 text-[10px] font-semibold uppercase tracking-widest text-zinc-600">
                How it works
              </p>
              <div className="space-y-5">
                {[
                  {
                    n: "01",
                    title: "Tap cards together",
                    body: "Two Fluvio users tap NFC cards. Both devices receive each other's UID instantly.",
                  },
                  {
                    n: "02",
                    title: "Assign a zone",
                    body: "Set zone 1, 2, or 3 to control exactly what this new contact can see from your graph.",
                  },
                  {
                    n: "03",
                    title: "Graphs connect",
                    body: "Shared nodes surface in both graphs, scoped to the zone permissions you set.",
                  },
                ].map((step) => (
                  <div key={step.n} className="flex items-start gap-4">
                    <span className="shrink-0 w-7 text-[11px] font-semibold tabular-nums text-zinc-700">
                      {step.n}
                    </span>
                    <div>
                      <p className="text-[13px] font-semibold text-zinc-200">{step.title}</p>
                      <p className="mt-0.5 text-xs leading-relaxed text-zinc-600">{step.body}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}