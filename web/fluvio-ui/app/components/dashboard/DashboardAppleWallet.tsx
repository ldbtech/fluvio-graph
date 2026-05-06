"use client";

import { useCallback, useState } from "react";
import { FluvioTwinMark } from "@/app/components/twin/FluvioTwinMark";
import { getOwnerId } from "@/lib/fluvioDashboardApi";

type Props = {
  displayName: string;
  tagline:     string;
  ownerSlug:   string;
};

export function DashboardAppleWallet({ displayName, tagline, ownerSlug }: Props) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const addToAppleWallet = useCallback(async () => {
    const id = getOwnerId();
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
    <section className="overflow-hidden rounded-2xl border border-white/[0.07] bg-gradient-to-br from-[#12101c] via-[#0b0b10] to-[#07060c] p-5 sm:p-6">
      <div className="flex flex-col gap-5 lg:flex-row lg:items-stretch lg:justify-between lg:gap-8">
        <div className="relative min-h-[176px] min-w-[min(100%,288px)] flex-1 lg:max-w-md">
          <div
            className="relative mx-auto aspect-[1.586/1] w-full max-w-[286px] overflow-hidden rounded-xl border border-white/[0.1] shadow-[0_36px_80px_-32px_rgba(83,74,183,0.55),inset_0_1px_0_rgba(255,255,255,0.05)] lg:mx-0"
            aria-hidden
          >
            <div className="absolute inset-0 bg-[linear-gradient(145deg,#1a1730_0%,#09090e_52%,#050508_100%)]" />
            <div className="absolute inset-x-8 top-3 h-[1px] bg-gradient-to-r from-transparent via-[#534AB7]/45 to-transparent" />
            <div className="absolute left-5 top-4 flex items-center gap-2">
              <FluvioTwinMark size={36} className="opacity-95" />
              <span className="text-[13px] font-medium tracking-[0.16em] text-white/90">FLUVIO</span>
            </div>
            <div className="absolute bottom-12 left-5 right-5">
              <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[#7369c4]/90">
                Personal twin
              </p>
              <p className="mt-2 truncate text-lg font-medium tracking-tight text-white">{displayName}</p>
              <p className="mt-1 line-clamp-2 text-[12px] leading-snug text-[#9a96b8]">{tagline}</p>
              <p className="mt-4 font-mono text-[10px] text-[#5F5E5A]">
                @{ownerSlug}
              </p>
            </div>
          </div>
          <div className="pointer-events-none absolute -right-4 top-12 hidden h-32 w-32 rounded-full bg-[#534AB7]/22 blur-[40px] sm:block" aria-hidden />
        </div>

        <div className="flex flex-1 flex-col justify-center gap-3 lg:max-w-xl">
          <div>
            <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-[#5F5E5A]">
              Apple Wallet
            </p>
            <h3 className="mt-2 text-[1.15rem] font-medium tracking-[-0.03em] text-white sm:text-lg">
              Black twin card matching your NFC profile
            </h3>
            <p className="mt-2 text-[13px] leading-relaxed text-[#888780]">
              On{" "}
              <span className="text-[#AFA9EC]">
                iPhone or iPad Safari
              </span>
              , tap{" "}
              <span className="font-medium text-white/90">
                Add to Wallet
              </span>
              {" "}
              to install a branded pass—same geometry and violet accents as Fluvio in the browser. Wallet shows your name
              and a QR code that opens your public tap route for visitors.
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
            <p className="text-[11px] leading-snug text-[#5F5E5A] sm:max-w-[14rem]">
              Requires env signing keys (Pass Type ID + WWDR). Also set{" "}
              <span className="font-mono text-[#888780]">WALLET_PASS_URL_SECRET</span> and{" "}
              <span className="font-mono text-[#888780]">NEXT_PUBLIC_APP_URL</span>
              {" "}for QR links.
            </p>
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
