"use client";

import Link from "next/link";
import { motion } from "framer-motion";

type ShowcaseProps = {
  /** Anchor id for in-page links */
  sectionId?: string;
  /** When true, primary CTA opens onboarding instead of inline scroll-only */
  showOnboardingCta?: boolean;
};

function NfcGlyph({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 48 48" className={className} fill="none" aria-hidden>
      <rect x="14" y="8" width="20" height="32" rx="3" stroke="currentColor" strokeWidth="1.25" opacity="0.85" />
      <path d="M24 14v6M20 17h8" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" opacity="0.7" />
      <path d="M18 28c2-3 10-3 12 0" stroke="currentColor" strokeWidth="1.15" strokeLinecap="round" opacity="0.55" />
      <path d="M16 24c3.5-5 12.5-5 16 0" stroke="currentColor" strokeWidth="1.05" strokeLinecap="round" opacity="0.35" />
    </svg>
  );
}

function WalletGlyph({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 48 48" className={className} fill="none" aria-hidden>
      <rect x="8" y="14" width="32" height="26" rx="4" stroke="currentColor" strokeWidth="1.25" opacity="0.88" />
      <path d="M8 22h32" stroke="currentColor" strokeWidth="1.05" opacity="0.45" />
      <rect x="26" y="28" width="12" height="8" rx="2" stroke="currentColor" strokeWidth="1.05" opacity="0.55" />
      <path d="M14 30h6" stroke="currentColor" strokeWidth="1" strokeLinecap="round" opacity="0.35" />
    </svg>
  );
}

const cardBase =
  "relative flex h-full flex-col overflow-hidden rounded-2xl border border-white/[0.07] bg-zinc-900/35 p-7 sm:p-9 " +
  "transition-colors hover:border-white/[0.12] lg:rounded-3xl";

export function BusinessCardShowcase({ sectionId = "business-cards", showOnboardingCta = true }: ShowcaseProps) {
  return (
    <section id={sectionId} className="scroll-mt-24 border-t border-white/[0.06] py-20 sm:py-28">
      <div className="mx-auto max-w-5xl px-5 sm:px-10">
        <div className="mx-auto max-w-[34rem] text-center">
          <h2 className="text-[1.875rem] font-semibold tracking-[-0.04em] text-white sm:text-[2.125rem]">
            Two ways to hand off you.
          </h2>
          <p className="mx-auto mt-5 text-[17px] leading-relaxed text-zinc-400">
            A pass in Wallet. Or a plastic card they tap. Both route to FluvioMe.
          </p>
        </div>

        <div className="mx-auto mt-14 grid max-w-4xl gap-6 lg:grid-cols-2 lg:gap-8">
          <motion.article
            initial={{ opacity: 0, y: 12 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-40px" }}
            transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
            className={cardBase}
          >
            <WalletGlyph className="mb-6 size-11 text-white/50" />
            <h3 className="text-[1.35rem] font-semibold tracking-[-0.03em] text-white">Wallet pass</h3>
            <p className="mt-4 flex-1 text-[16px] leading-relaxed text-zinc-400">
              Lives next to Apple Pay and Google Pay. They add you like any other pass—then open you with one swipe.
            </p>
          </motion.article>

          <motion.article
            initial={{ opacity: 0, y: 12 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-40px" }}
            transition={{ duration: 0.4, delay: 0.05, ease: [0.22, 1, 0.36, 1] }}
            className={cardBase}
          >
            <NfcGlyph className="mb-6 size-11 text-white/50" />
            <h3 className="text-[1.35rem] font-semibold tracking-[-0.03em] text-white">NFC card</h3>
            <p className="mt-4 flex-1 text-[16px] leading-relaxed text-zinc-400">
              For handshakes and coffee meetings. Phone to card—same FluvioMe as Wallet, zero explanation.
            </p>
          </motion.article>
        </div>

        {showOnboardingCta ? (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.35, delay: 0.08 }}
            className="mt-14 flex justify-center"
          >
            <Link
              href="/onboarding"
              className="inline-flex h-12 w-full max-w-xs items-center justify-center rounded-full bg-white text-[15px] font-semibold text-zinc-950 transition hover:bg-zinc-100 sm:w-auto sm:min-w-[14rem]"
            >
              Get started
            </Link>
          </motion.div>
        ) : null}
      </div>
    </section>
  );
}
