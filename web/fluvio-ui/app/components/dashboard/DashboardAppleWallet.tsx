"use client";

import { useCallback, useState } from "react";
import { FluvioTwinMark } from "@/app/components/twin/FluvioTwinMark";
import { getTwinUserId } from "@/lib/fluvioDashboardApi";

type Props = {
  displayName: string;
  tagline:     string;
  ownerSlug:   string;
};

export function DashboardAppleWallet({ displayName, tagline, ownerSlug }: Props) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

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
        pkpassUrl?:           string;
        signingConfigured?:  boolean;
        error?:               string;
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

  return (
    <section className="overflow-hidden rounded-[20px] border border-white/[0.06] bg-white/[0.02] p-6 sm:p-8">
      <div className="flex flex-col gap-8 lg:flex-row lg:items-stretch lg:justify-between lg:gap-10">
        <div className="relative min-h-[176px] min-w-[min(100%,340px)] flex-1 lg:max-w-md">
          {/* Same footprint as onboarding NFC preview: logo tile + imprint, one subtle footer line */}
          <div
            className="relative mx-auto w-full max-w-[340px] overflow-hidden rounded-[1.125rem] border border-white/[0.12] bg-[linear-gradient(155deg,#1c1c22_0%,#0f0f12_42%,#080809_100%)] shadow-[0_36px_60px_-28px_rgba(0,0,0,0.85)] shadow-black/70 lg:mx-0"
            style={{ aspectRatio: "1.586 / 1" }}
            aria-hidden
          >
            <div className="absolute inset-x-9 top-3 h-px bg-gradient-to-r from-transparent via-white/[0.12] to-transparent" />
            <div className="absolute left-[1.125rem] top-[2.125rem] right-[1.125rem] flex gap-4">
              <div className="flex size-[3.375rem] shrink-0 items-center justify-center overflow-hidden rounded-[0.625rem] border border-white/[0.1] bg-white/[0.04] shadow-inner shadow-black/30">
                <FluvioTwinMark size={28} className="opacity-[0.92]" />
              </div>
              <div className="min-w-0 flex-1 text-left">
                <p className="truncate text-[1.0625rem] font-semibold leading-tight tracking-[-0.02em] text-white">
                  {displayName.trim() || "Your name"}
                </p>
                <p className="mt-3 truncate text-[13px] font-medium text-zinc-300">@{ownerSlug.trim() || "you"}</p>
                {tagline.trim() ? (
                  <p className="mt-2 line-clamp-2 text-[11px] leading-snug text-zinc-500">{tagline.trim()}</p>
                ) : (
                  <p className="mt-2 text-[11px] text-zinc-600">Short tagline</p>
                )}
              </div>
            </div>
            <div className="absolute bottom-[1rem] left-[1.125rem] right-[1.125rem] flex items-end justify-between gap-2 border-t border-white/[0.06] pt-[0.75rem]">
              <span className="font-mono text-[9px] tracking-wide text-zinc-600">Tap · FluvioMe</span>
              <span className="text-[9px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Wallet</span>
            </div>
          </div>
        </div>

        <div className="flex flex-1 flex-col justify-center gap-3 lg:max-w-xl">
          <div>
            <p className="text-[13px] font-medium text-zinc-500">Apple Wallet</p>
            <h3 className="mt-1 text-xl font-semibold tracking-[-0.03em] text-white sm:text-[1.35rem]">Your pass</h3>
            <p className="mt-3 text-[15px] leading-relaxed text-zinc-500">
              On iPhone or iPad, open this page in Safari and tap below. Your name and a QR code go in Wallet—same look as your
              tap card online.
            </p>
          </div>

          {err ? (
            <p className="rounded-lg border border-amber-500/35 bg-amber-500/[0.08] px-3 py-2 text-[12px] leading-snug text-amber-200/95">
              {err}
            </p>
          ) : null}

          <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
            <button
              type="button"
              disabled={busy}
              onClick={() => void addToAppleWallet()}
              className="inline-flex min-h-12 items-center justify-center gap-2.5 rounded-lg bg-black px-5 shadow-[inset_0_1px_0_rgba(255,255,255,0.18)] ring-1 ring-white/14 transition hover:ring-[#534AB7]/45 active:bg-zinc-950 disabled:opacity-45"
              aria-label="Add to Apple Wallet"
            >
              <AppleGlyph className="h-7 w-7 shrink-0 text-white" />
              <span className="text-[15px] font-medium tracking-[0.01em] text-white">
                {busy ? "Preparing…" : "Add to Apple Wallet"}
              </span>
            </button>
            <details className="sm:max-w-xs">
              <summary className="cursor-pointer text-[13px] text-zinc-600 underline-offset-2 hover:text-zinc-500">
                Developer setup
              </summary>
              <p className="mt-2 text-[12px] leading-snug text-zinc-600">
                Apple Pass signing and env vars (Pass Type ID, WWDR, <span className="font-mono text-zinc-500">WALLET_PASS_URL_SECRET</span>
                , <span className="font-mono text-zinc-500">NEXT_PUBLIC_APP_URL</span>) must be set on the server.
              </p>
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
