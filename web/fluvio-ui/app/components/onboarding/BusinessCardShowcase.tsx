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
      <path
        d="M24 14v6M20 17h8"
        stroke="currentColor"
        strokeWidth="1.1"
        strokeLinecap="round"
        opacity="0.7"
      />
      <path
        d="M18 28c2-3 10-3 12 0"
        stroke="currentColor"
        strokeWidth="1.15"
        strokeLinecap="round"
        opacity="0.55"
      />
      <path
        d="M16 24c3.5-5 12.5-5 16 0"
        stroke="currentColor"
        strokeWidth="1.05"
        strokeLinecap="round"
        opacity="0.35"
      />
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

function WifiGlyph({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 48 48" className={className} fill="none" aria-hidden>
      <path
        d="M10 22c8-9 20-9 28 0"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
        opacity="0.35"
      />
      <path
        d="M14 26c6-6.5 14-6.5 20 0"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
        opacity="0.55"
      />
      <path
        d="M18 30c4-4 8-4 12 0"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
        opacity="0.75"
      />
      <circle cx="24" cy="36" r="2" fill="currentColor" opacity="0.9" />
    </svg>
  );
}

const cardBase =
  "relative flex h-full flex-col overflow-hidden rounded-[1.35rem] border p-6 sm:p-8 " +
  "border-sky-500/12 bg-slate-950/50 shadow-[inset_0_1px_0_rgba(56,189,248,0.07),0_24px_48px_-28px_rgba(0,0,0,0.65)] " +
  "transition-colors hover:border-sky-400/22";

export function BusinessCardShowcase({ sectionId = "business-cards", showOnboardingCta = true }: ShowcaseProps) {
  return (
    <section
      id={sectionId}
      className="scroll-mt-24 border-t border-sky-500/[0.08] bg-gradient-to-b from-violet-950/25 via-[#070a12]/90 to-[#070a12] py-14 sm:py-28"
    >
      <div className="mx-auto max-w-6xl px-[max(1.25rem,env(safe-area-inset-left))] pb-4 pr-[max(1.25rem,env(safe-area-inset-right))] sm:px-8">
        <div className="mx-auto max-w-2xl text-center">
          <h2 className="text-[11px] font-medium uppercase tracking-[0.16em] text-violet-300/80">Hardware &amp; wallet</h2>
          <p className="mt-3 text-2xl font-medium tracking-[-0.03em] text-white sm:text-[1.85rem] sm:leading-snug">
            Fluvio business cards
          </p>
          <p className="mt-4 text-pretty text-[15px] leading-relaxed text-slate-400 sm:text-[16px]">
            Tap into your digital twin: NFC for phone-first moments; Wi‑Fi for cart-to-cart or cart-to-phone handoffs; and
            a wallet pass you add to <span className="text-slate-300">Apple Wallet</span> or{" "}
            <span className="text-slate-300">Google Wallet</span> so it sits next to your{" "}
            <span className="text-slate-300">Apple Pay</span> and <span className="text-slate-300">Google Pay</span>{" "}
            cards—one tap opens your twin from the wallet people already open every day.
          </p>
          <p className="mx-auto mt-4 max-w-xl text-pretty text-[14px] leading-relaxed text-slate-500 sm:text-[15px]">
            <span className="font-medium text-slate-400">No cart or ship required to start.</span> Create your twin in
            onboarding, generate the pass, and add it to the wallet app—you are not checking out with Apple Pay or Google
            Pay to “buy” the twin; the pass simply lives in that same wallet alongside payment cards. NFC cards and Wi‑Fi
            retail carts are optional when you want physical tap or in-aisle workflows.
          </p>
        </div>

        <div className="mx-auto mt-12 grid max-w-5xl gap-5 sm:mt-14 md:grid-cols-3 md:gap-6">
          <motion.article
            initial={{ opacity: 0, y: 14 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-40px" }}
            transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
            className={cardBase}
          >
            <div
              className="pointer-events-none absolute -right-8 -top-8 size-36 rounded-full bg-sky-500/12 blur-3xl"
              aria-hidden
            />
            <div className="mb-5 flex items-start justify-between gap-3">
              <span className="inline-flex items-center gap-2 rounded-full border border-sky-400/20 bg-sky-500/10 px-3 py-1 text-[11px] font-medium uppercase tracking-[0.12em] text-sky-200/90">
                NFC · optional ship
              </span>
              <NfcGlyph className="size-12 shrink-0 text-sky-300/90" />
            </div>
            <h3 className="text-lg font-medium tracking-[-0.02em] text-white">Phone tap</h3>
            <p className="mt-2 flex-1 text-[14px] leading-[1.65] text-slate-400">
              Standard NFC business card. One tap opens your twin on the guest&apos;s phone—ideal for conferences,
              introductions, and leave-behinds.
            </p>
            <p className="mt-4 text-[12px] leading-relaxed text-slate-500">
              Order when you want a leave-behind card; wallet-only works without it. The tag routes guests into your hosted
              experience at tap time.
            </p>
          </motion.article>

          <motion.article
            initial={{ opacity: 0, y: 14 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-40px" }}
            transition={{ duration: 0.45, delay: 0.06, ease: [0.22, 1, 0.36, 1] }}
            className={cardBase}
          >
            <div
              className="pointer-events-none absolute -right-6 -top-10 size-40 rounded-full bg-violet-500/15 blur-3xl"
              aria-hidden
            />
            <div className="mb-5 flex items-start justify-between gap-3">
              <span className="inline-flex items-center gap-2 rounded-full border border-violet-400/25 bg-violet-500/12 px-3 py-1 text-[11px] font-medium uppercase tracking-[0.12em] text-violet-100/90">
                Wi‑Fi · retail
              </span>
              <WifiGlyph className="size-12 shrink-0 text-violet-300/90" />
            </div>
            <h3 className="text-lg font-medium tracking-[-0.02em] text-white">Cart to cart · cart to phone</h3>
            <p className="mt-2 flex-1 text-[14px] leading-[1.65] text-slate-400">
              Wi‑Fi aware carts sync context in the aisle: hand off a session from cart to cart, or beam a summary to a
              shopper&apos;s phone when the moment calls for it.
            </p>
            <p className="mt-4 text-[12px] leading-relaxed text-slate-500">
              Separate retail rollout—not required for personal twin creation.
            </p>
          </motion.article>

          <motion.article
            initial={{ opacity: 0, y: 14 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-40px" }}
            transition={{ duration: 0.45, delay: 0.12, ease: [0.22, 1, 0.36, 1] }}
            className={cardBase}
          >
            <div
              className="pointer-events-none absolute -right-8 -top-10 size-40 rounded-full bg-emerald-500/12 blur-3xl"
              aria-hidden
            />
            <div className="mb-5 flex items-start justify-between gap-3">
              <span className="inline-flex items-center gap-2 rounded-full border border-emerald-400/25 bg-emerald-500/10 px-3 py-1 text-[11px] font-medium uppercase tracking-[0.12em] text-emerald-100/90">
                Wallet · start here
              </span>
              <WalletGlyph className="size-12 shrink-0 text-emerald-300/90" />
            </div>
            <h3 className="text-lg font-medium tracking-[-0.02em] text-white">Apple Wallet · Google Wallet</h3>
            <p className="mt-2 flex-1 text-[14px] leading-[1.65] text-slate-400">
              Add the same digital card as a pass in <span className="text-slate-300">Apple Wallet</span> or{" "}
              <span className="text-slate-300">Google Wallet</span>. It sits next to{" "}
              <span className="text-slate-300">Apple Pay</span> and <span className="text-slate-300">Google Pay</span>{" "}
              cards—so your twin is a thumb-reach away at the register, the gate, or the green room.
            </p>
            <p className="mt-4 text-[12px] leading-relaxed text-slate-500">
              Add from onboarding—no NFC order or shopping cart checkout. Uses the Wallet app your phone already has.
            </p>
          </motion.article>
        </div>

        <motion.div
          initial={{ opacity: 0, y: 10 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ duration: 0.4, delay: 0.1 }}
          className="mx-auto mt-10 flex max-w-xl flex-col items-center justify-center gap-3 sm:mt-12 sm:flex-row sm:gap-4"
        >
          {showOnboardingCta ? (
            <Link
              href="/onboarding"
              className="inline-flex h-11 w-full items-center justify-center rounded-full bg-gradient-to-r from-sky-500 to-violet-500 px-8 text-[14px] font-medium text-white shadow-[0_0_32px_-8px_rgba(56,189,248,0.45)] transition hover:brightness-110 sm:w-auto"
            >
              Twin &amp; wallet setup
            </Link>
          ) : null}
          <a
            href="mailto:hello@fluvio.example?subject=Fluvio%20business%20cards"
            className="inline-flex h-11 w-full items-center justify-center rounded-full border border-slate-600/50 bg-slate-950/60 px-8 text-[14px] font-medium text-slate-200 transition hover:border-sky-400/30 hover:bg-slate-900/80 sm:w-auto"
          >
            Request pricing
          </a>
        </motion.div>
        <p className="mx-auto mt-6 max-w-lg text-center text-[12px] leading-relaxed text-slate-600">
          Onboarding covers twin profile, add-to-wallet, and optional hardware pairing. Your sources and docs are managed in
          the Fluvio dashboard with kg-engine.
        </p>
      </div>
    </section>
  );
}
