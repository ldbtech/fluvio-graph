"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useRef } from "react";
import { motion } from "framer-motion";
import { FluvioTwinMark } from "./FluvioTwinMark";
import { NfcRipple } from "./NfcRipple";
import { resetTwinChatBootstrap } from "@/lib/twinChatSession";
import { tapCard } from "@/lib/fluvioDashboardApi";

const BG = "#0A0A0F";

export function NfcTapLanding() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const linkedCardId = searchParams.get("card")?.trim() ?? "";

  useEffect(() => {
    if (!linkedCardId) return;
    void tapCard(linkedCardId).catch(() => {
      /* still allow entering chat demo */
    });
  }, [linkedCardId]);

  const goChat = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    resetTwinChatBootstrap();
    router.push("/chat");
  }, [router]);

  useEffect(() => {
    timerRef.current = setTimeout(() => {
      goChat();
    }, 2000);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [goChat]);

  return (
    <div
      className="relative flex min-h-dvh cursor-pointer flex-col items-center justify-center px-5 pb-28 pt-[max(1.25rem,env(safe-area-inset-top))] text-center sm:px-6 sm:pb-24"
      style={{ backgroundColor: BG }}
      onClick={goChat}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          goChat();
        }
      }}
    >
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        <NfcRipple />
      </div>

      <motion.div
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
        className="relative z-10 flex max-w-md flex-col items-center"
      >
        <div className="relative mb-6 flex h-[4.85rem] w-[4.85rem] items-center justify-center sm:mb-8 sm:h-28 sm:w-28">
          <div className="absolute inset-0 flex items-center justify-center opacity-40 blur-xl">
            <div className="size-[4.25rem] rounded-full bg-[#534AB7]/50 sm:size-24" />
          </div>
          <FluvioTwinMark size={64} className="relative sm:size-[72px]" />
        </div>
        <h1 className="text-balance px-2 text-[1.28rem] font-medium leading-snug tracking-[-0.03em] text-white sm:text-2xl">
          {linkedCardId ? "You opened a Fluvio card" : "You just tapped a demo card"}
        </h1>
        <p className="mt-3 text-[15px] text-[#888780]">Ask me anything</p>
        <p className="mt-8 text-[11px] tracking-wide text-[#5F5E5A]">Tap anywhere to continue</p>
      </motion.div>

      <p className="pointer-events-auto absolute bottom-0 left-0 right-0 z-20 border-t border-white/[0.04] bg-[#0A0A0F]/80 px-3 pb-[max(0.875rem,env(safe-area-inset-bottom))] pt-3 backdrop-blur-md">
        <span className="mx-auto flex max-w-md flex-wrap items-center justify-center gap-x-1 gap-y-0 sm:flex-nowrap sm:gap-x-3">
          <Link
            href="/"
            className="min-h-[2.75rem] content-center px-3 py-2 text-[13px] leading-tight text-[#5F5E5A] underline-offset-4 active:bg-white/[0.06] hover:text-[#AFA9EC] hover:underline sm:min-h-0 sm:text-[12px]"
          >
            Home
          </Link>
          <span className="hidden text-[#3F3E3A] sm:inline" aria-hidden>
            ·
          </span>
          <Link
            href="/dashboard"
            className="min-h-[2.75rem] content-center px-3 py-2 text-[13px] leading-tight text-[#7F77DD] underline-offset-4 active:bg-white/[0.06] hover:text-[#AFA9EC] hover:underline sm:min-h-0 sm:text-[12px]"
          >
            Dashboard
          </Link>
          <span className="hidden text-[#3F3E3A] sm:inline" aria-hidden>
            ·
          </span>
          <Link
            href="/onboarding"
            className="min-h-[2.75rem] content-center px-3 py-2 text-[13px] leading-tight text-[#7F77DD] underline-offset-4 active:bg-white/[0.06] hover:text-[#AFA9EC] hover:underline sm:min-h-0 sm:text-[12px]"
          >
            Set up
          </Link>
          <span className="hidden text-[#3F3E3A] sm:inline" aria-hidden>
            ·
          </span>
          <Link
            href="/product"
            className="min-h-[2.75rem] content-center px-3 py-2 text-[13px] leading-tight text-[#5F5E5A] underline-offset-4 active:bg-white/[0.06] hover:text-[#888780] hover:underline sm:min-h-0 sm:text-[12px]"
          >
            Product
          </Link>
        </span>
      </p>
    </div>
  );
}
