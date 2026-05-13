"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import { useEffect, useState } from "react";
import {
  fetchAuthMe,
  fetchFluvioAccount,
  type FluvioAccount,
} from "@/shared/lib/fluvioDashboardApi";

function profileInitial(displayName: string) {
  const t = displayName.trim();
  if (!t) return "?";
  const ch = t[0];
  return ch.toUpperCase();
}

type Props = {
  /** Width / padding for the hero card. Place this node directly under the page navbar. */
  className?: string;
};

/** Signed-in identity hero — render immediately below each screen’s top nav (not in root layout). */
export function AuthedProfileHeader({ className }: Props) {
  const [account, setAccount] = useState<FluvioAccount | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const me = await fetchAuthMe();
        if (cancelled || !me) {
          if (!cancelled) setAccount(null);
          return;
        }
        const acc = await fetchFluvioAccount();
        if (!cancelled) setAccount(acc);
      } catch {
        if (!cancelled) setAccount(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (!account) return null;

  return (
    <div className={className ?? "mx-auto w-full max-w-5xl px-5 py-2 sm:px-8"}>
      <motion.section
        initial={{ opacity: 0, y: 4 }}
        animate={{ opacity: 1, y: 0 }}
        className="relative overflow-hidden rounded-[20px] border border-white/[0.08] bg-[linear-gradient(165deg,rgba(139,92,246,0.09)_0%,rgba(9,9,11,0.5)_38%,rgba(9,9,11,0.72)_100%)] shadow-[0_0_0_1px_rgba(255,255,255,0.04)_inset]"
      >
        <div
          className="pointer-events-none absolute -right-20 -top-28 h-64 w-64 rounded-full bg-violet-500/15 blur-3xl"
          aria-hidden
        />
        <div
          className="pointer-events-none absolute -bottom-24 -left-16 h-56 w-56 rounded-full bg-fuchsia-500/10 blur-3xl"
          aria-hidden
        />
        <div className="relative p-5 sm:p-7">
          <div className="flex flex-col gap-5 sm:flex-row sm:items-start sm:gap-7">
            <div
              className="flex h-[3rem] w-[3rem] shrink-0 items-center justify-center rounded-2xl bg-violet-500/[0.18] text-[1.05rem] font-semibold tracking-tight text-violet-100 ring-1 ring-violet-400/25 shadow-[0_12px_40px_-16px_rgba(139,92,246,0.55)] sm:h-[3.25rem] sm:w-[3.25rem] sm:text-[1.15rem]"
              aria-hidden
            >
              {profileInitial(account.display_name)}
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex flex-col gap-2.5 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
                <div className="min-w-0">
                  <h2 className="text-[1.45rem] font-semibold tracking-[-0.035em] text-white sm:text-[1.7rem]">
                    {account.display_name}
                  </h2>
                  {account.tagline ? (
                    <p className="mt-1.5 max-w-2xl text-pretty text-[14px] leading-relaxed text-zinc-400 sm:text-[15px]">
                      {account.tagline}
                    </p>
                  ) : null}
                </div>
                <span className="inline-flex w-fit shrink-0 items-center rounded-full border border-white/[0.1] bg-zinc-950/60 px-3 py-1.5 font-mono text-[12px] font-medium tracking-tight text-zinc-300 ring-1 ring-white/[0.04] sm:text-[13px]">
                  @{account.owner_slug}
                </span>
              </div>
              <p className="mt-4 text-[12px] leading-relaxed text-zinc-500 sm:text-[13px]">
                Email and phone for follow-ups live in{" "}
                <Link
                  href="/settings"
                  className="font-medium text-violet-400 underline-offset-4 hover:underline"
                >
                  Settings
                </Link>
                .
              </p>
            </div>
          </div>
        </div>
      </motion.section>
    </div>
  );
}
