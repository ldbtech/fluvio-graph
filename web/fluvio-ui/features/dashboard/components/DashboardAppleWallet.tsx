"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import { NfcCardFrontPreview } from "@/shared/components/NfcCardDesignPreview";
import { resolveSessionNfcCardDesign, subscribeHardwareOrders } from "@/shared/lib/hardwareOrders";
import type { NfcCardDesign } from "@/shared/lib/hardwareOrders";
import { getTwinUserId } from "@/shared/lib/fluvioDashboardApi";

type Props = {
  displayName: string;
  tagline: string;
  /** Physical / logical NFC card UUID — tap URL is `/twin/tap/{nfcCardId}`. */
  nfcCardId?: string | null;
};

function mergeDesignWithProfile(
  design: NfcCardDesign,
  displayName: string,
  tagline: string,
): NfcCardDesign {
  return {
    ...design,
    nameOnCard: design.nameOnCard.trim() || displayName.trim() || "Your name",
    tagline: design.tagline.trim() || tagline.trim(),
  };
}

export function DashboardAppleWallet({ displayName, tagline, nfcCardId }: Props) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState(false);
  const [savedDesign, setSavedDesign] = useState<NfcCardDesign | null>(null);

  const refreshSavedDesign = useCallback(() => {
    setSavedDesign(resolveSessionNfcCardDesign(getTwinUserId()));
  }, []);

  useEffect(() => {
    refreshSavedDesign();
    return subscribeHardwareOrders(refreshSavedDesign);
  }, [refreshSavedDesign]);

  const previewDesign = useMemo(() => {
    if (!savedDesign) return null;
    return mergeDesignWithProfile(savedDesign, displayName, tagline);
  }, [savedDesign, displayName, tagline]);

  const addToAppleWallet = useCallback(async () => {
    const id = getTwinUserId();
    if (!id) return;
    setBusy(true);
    setErr(null);
    try {
      const r = await fetch("/api/wallet/issue-url", {
        method: "POST",
        headers: { "X-Owner-ID": id },
      });
      const body = (await r.json()) as {
        pkpassUrl?: string;
        signingConfigured?: boolean;
        error?: string;
      };
      if (!r.ok || !body.pkpassUrl) {
        const raw = body.error ?? `Could not prepare pass (${r.status})`;
        if (raw.includes("WALLET_PASS_URL_SECRET")) {
          setErr(
            "Add WALLET_PASS_URL_SECRET to web/fluvio-ui/.env.local (e.g. openssl rand -hex 32), restart next dev, then try again.",
          );
        } else {
          setErr(raw);
        }
        return;
      }
      if (!body.signingConfigured) {
        setErr(
          "Apple signing is not configured. Set APPLE_PASS_WWDR_PATH, APPLE_PASS_SIGNER_CERT_PATH, APPLE_PASS_SIGNER_KEY_PATH, APPLE_PASS_TYPE_ID, and APPLE_PASS_TEAM_ID.",
        );
        return;
      }
      window.location.assign(body.pkpassUrl);
    } catch {
      setErr("Network error while requesting the pass.");
    } finally {
      setBusy(false);
    }
  }, []);

  const copyCardId = useCallback(async () => {
    const id = nfcCardId?.trim();
    if (!id) return;
    try {
      await navigator.clipboard.writeText(id);
      setCopiedId(true);
      window.setTimeout(() => setCopiedId(false), 2000);
    } catch {
      try {
        const ta = document.createElement("textarea");
        ta.value = id;
        ta.style.position = "fixed";
        ta.style.left = "-9999px";
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        document.body.removeChild(ta);
        setCopiedId(true);
        window.setTimeout(() => setCopiedId(false), 2000);
      } catch {
        /* ignore */
      }
    }
  }, [nfcCardId]);

  return (
    <section className="relative overflow-hidden rounded-[22px] border border-white/[0.08] bg-[linear-gradient(165deg,rgba(139,92,246,0.08)_0%,rgba(9,9,11,0.55)_40%,rgba(9,9,11,0.72)_100%)] shadow-[0_0_0_1px_rgba(255,255,255,0.04)_inset]">
      <div className="pointer-events-none absolute -right-24 -top-32 h-72 w-72 rounded-full bg-violet-500/12 blur-3xl" aria-hidden />
      <div className="pointer-events-none absolute -bottom-28 -left-20 h-64 w-64 rounded-full bg-indigo-500/10 blur-3xl" aria-hidden />
      <div className="relative p-8 sm:p-10">
        <div className="flex flex-col gap-10 lg:flex-row lg:items-stretch lg:justify-between lg:gap-12">
          <div className="relative flex min-h-[200px] min-w-0 flex-1 flex-col items-center justify-center lg:max-w-md lg:items-start">
            <p className="mb-4 w-full text-left text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
              {previewDesign ? "Your card design" : "Card preview"}
            </p>
            {previewDesign ? (
              <div className="w-full">
                <NfcCardFrontPreview design={previewDesign} walletFooter />
                <p className="mt-4 max-w-[340px] text-left text-[12px] leading-relaxed text-zinc-600">
                  Pulled from your saved NFC order or onboarding snapshot on this device. Wallet still uses Apple’s pass layout;
                  this is how your physical card and dashboard stay aligned.
                </p>
              </div>
            ) : (
              <div
                className="relative flex w-full max-w-[340px] flex-col items-stretch justify-center overflow-hidden rounded-[1.125rem] border border-dashed border-violet-500/25 bg-zinc-950/50 p-6 text-left shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]"
                style={{ aspectRatio: "1.586 / 1" }}
              >
                <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_30%_0%,rgba(139,92,246,0.12),transparent_55%)]" aria-hidden />
                <div className="relative flex flex-1 flex-col justify-center">
                  <p className="text-[13px] font-semibold text-white">No card design saved yet</p>
                  <p className="mt-2 text-[13px] leading-relaxed text-zinc-500">
                    Design your tap card—logo, finish, accent, and imprint—then it shows here and stays with your hardware order
                    on this browser.
                  </p>
                  <Link
                    href="/onboarding?path=nfc"
                    className="mt-5 inline-flex w-fit items-center justify-center rounded-full bg-violet-500 px-5 py-2.5 text-[14px] font-semibold text-white shadow-[0_12px_32px_-12px_rgba(139,92,246,0.55)] transition hover:bg-violet-400"
                  >
                    Design your card
                  </Link>
                </div>
              </div>
            )}
          </div>

          <div className="flex min-w-0 flex-1 flex-col justify-center gap-5 lg:max-w-xl">
            <div>
              <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Apple Wallet</p>
              <h3 className="mt-2 text-[1.35rem] font-semibold tracking-[-0.03em] text-white sm:text-[1.45rem]">Your pass</h3>
              <p className="mt-3 text-[15px] leading-relaxed text-zinc-500">
                On iPhone or iPad, open this page in Safari and tap below. Your name and a QR code go in Wallet—tuned to match your
                FluvioMe tap flow once signing is live.
              </p>
            </div>

            {err ? (
              <p className="rounded-xl border border-amber-500/30 bg-amber-500/[0.08] px-4 py-3 text-[13px] leading-snug text-amber-200/95">
                {err}
              </p>
            ) : null}

            <div className="flex flex-col gap-4 sm:flex-row sm:flex-wrap sm:items-center">
              <button
                type="button"
                disabled={busy}
                onClick={() => void addToAppleWallet()}
                className="inline-flex min-h-12 items-center justify-center gap-2.5 rounded-full bg-black px-6 shadow-[inset_0_1px_0_rgba(255,255,255,0.18)] ring-1 ring-white/14 transition hover:ring-violet-400/40 active:bg-zinc-950 disabled:opacity-45"
                aria-label="Add to Apple Wallet"
              >
                <AppleGlyph className="h-7 w-7 shrink-0 text-white" />
                <span className="text-[15px] font-medium tracking-[0.01em] text-white">
                  {busy ? "Preparing…" : "Add to Apple Wallet"}
                </span>
              </button>
            </div>

            <details className="group rounded-2xl border border-white/[0.07] bg-zinc-950/45 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
              <summary className="cursor-pointer list-none px-4 py-3.5 text-[14px] font-medium text-zinc-300 transition marker:content-none group-open:border-b group-open:border-white/[0.06] hover:text-white [&::-webkit-details-marker]:hidden">
                <span className="inline-flex items-center gap-2">
                  <span className="rounded-md bg-white/[0.06] px-2 py-0.5 font-mono text-[11px] font-semibold uppercase tracking-wider text-violet-300/90">
                    Dev
                  </span>
                  Server setup & NFC id
                </span>
              </summary>
              <div className="space-y-4 px-4 py-4 text-[13px] leading-relaxed text-zinc-500">
                <p>
                  Apple Pass signing and env vars (Pass Type ID, WWDR,{" "}
                  <code className="rounded-md border border-white/[0.08] bg-black/35 px-1.5 py-0.5 font-mono text-[12px] text-zinc-300">
                    WALLET_PASS_URL_SECRET
                  </code>
                  ,{" "}
                  <code className="rounded-md border border-white/[0.08] bg-black/35 px-1.5 py-0.5 font-mono text-[12px] text-zinc-300">
                    NEXT_PUBLIC_APP_URL
                  </code>
                  ) must be set on the server.
                </p>
                {nfcCardId ? (
                  <div className="rounded-xl border border-white/[0.08] bg-black/25 p-4">
                    <p className="text-[11px] font-semibold uppercase tracking-[0.12em] text-zinc-500">NFC card ID</p>
                    <p className="mt-1.5 text-[12px] text-zinc-600">
                      Tap URLs, Wallet routing, and simulator tests use{" "}
                      <code className="font-mono text-[11px] text-zinc-400">/twin/tap/…</code>.
                    </p>
                    <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:items-stretch">
                      <code className="block min-w-0 flex-1 break-all rounded-xl border border-white/[0.08] bg-zinc-950/80 px-3 py-2.5 font-mono text-[11px] leading-relaxed text-zinc-200">
                        {nfcCardId}
                      </code>
                      <button
                        type="button"
                        onClick={() => void copyCardId()}
                        className="shrink-0 rounded-xl border border-white/[0.12] bg-white/[0.08] px-4 py-2.5 text-[13px] font-medium text-zinc-100 transition hover:bg-white/[0.12]"
                      >
                        {copiedId ? "Copied" : "Copy"}
                      </button>
                    </div>
                  </div>
                ) : (
                  <p className="rounded-xl border border-white/[0.06] bg-black/20 px-3 py-2.5 text-[12px] text-zinc-600">
                    Card ID appears after your profile loads from the API (<span className="font-mono text-zinc-500">GET /twin/me</span>
                    ).
                  </p>
                )}
              </div>
            </details>
          </div>
        </div>
      </div>
    </section>
  );
}

function AppleGlyph({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 384 512" aria-hidden>
      <path
        fill="currentColor"
        d="M318.7 268.7c-.2-36.7 16.4-64.4 50-84.8-18.8-26.9-47.2-41.7-84.7-44.6-35.5-2.8-74.3 20.7-88.5 20.7-15 0-49.4-19.7-76.4-19.7C63.3 141.2 4 184.8 4 273.5q0 39.9 14.8 73.9c41.9 103.9 157.9 258.9 217.9 258.9 32.3 0 55.3-10.5 74.2-24.2 1.4-1 2.7-2.1 4-3.2-.4-1.2-1.1-2.4-1.6-3.5-12.3-24.4-15.4-48.3-6.3-72.4 9.3-25.2 28.4-44.3 56.4-57.3-16.3-8.3-35.2-12.8-55.2-12.8zM255.5 56.3c15.2.2 33.2-8.3 45.2-25.1 10.1-14.4 16.9-33.7 15.1-53.4-14.4 1.2-32.1 9.5-42.5 23.8-10.6 14.7-19.5 34.1-17.8 53.7z"
      />
    </svg>
  );
}
