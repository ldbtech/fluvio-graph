"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";

export type CardKind = "nfc" | "wifi";

export type OnboardingClientProps = {
  initialCardKind?: CardKind | null;
};

const STORAGE_KEY = "fluvio_onboarding_v1";

type Stored = { completedAt: string; cardKind: CardKind };

function writeStored(cardKind: CardKind) {
  const payload: Stored = { completedAt: new Date().toISOString(), cardKind };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
}

const steps = ["Welcome", "Your card", "Pair", "Ready"] as const;

type StepIndex = 0 | 1 | 2 | 3;

export function OnboardingClient({ initialCardKind = null }: OnboardingClientProps) {
  const router = useRouter();

  const [step, setStep] = useState<StepIndex>(0);
  const [kind, setKind] = useState<CardKind | null>(initialCardKind ?? null);
  const [pairBusy, setPairBusy] = useState(false);

  const goNext = useCallback(() => {
    setStep((s) => (s < 3 ? ((s + 1) as StepIndex) : s));
  }, []);

  const goBack = useCallback(() => {
    setStep((s) => (s > 0 ? ((s - 1) as StepIndex) : s));
  }, []);

  const completePairing = useCallback(() => {
    if (!kind) return;
    setPairBusy(true);
    window.setTimeout(() => {
      setPairBusy(false);
      writeStored(kind);
      setStep(3);
    }, 1600);
  }, [kind]);

  const chooseCard = useCallback((k: CardKind) => {
    setKind(k);
    setStep(2);
  }, []);

  return (
    <div className="relative min-h-dvh bg-[#070a12] text-slate-100 pb-[max(1.5rem,env(safe-area-inset-bottom))]">
      <div
        className="pointer-events-none fixed inset-0 -z-10 bg-[radial-gradient(ellipse_80%_50%_at_50%_-10%,rgba(139,92,246,0.14),transparent_55%),radial-gradient(ellipse_60%_40%_at_100%_20%,rgba(56,189,248,0.08),transparent_50%),radial-gradient(ellipse_50%_35%_at_0%_80%,rgba(14,165,233,0.06),transparent_45%)]"
        aria-hidden
      />

      <header className="sticky top-0 z-30 border-b border-white/[0.06] bg-[#070a12]/92 backdrop-blur-xl pt-[max(0.25rem,env(safe-area-inset-top))]">
        <div className="mx-auto flex h-[3.35rem] max-w-lg items-center justify-between gap-2 px-3 sm:h-14 sm:px-6">
          <button
            type="button"
            onClick={() => (step === 0 ? router.push("/product") : goBack())}
            className="min-h-11 min-w-[3.25rem] rounded-lg px-2 text-[13px] font-medium text-slate-400 transition hover:bg-white/[0.05] hover:text-sky-300 active:bg-white/[0.07]"
          >
            {step === 0 ? "Close" : "Back"}
          </button>
          <span className="min-w-0 truncate text-center text-[11px] font-medium tracking-wide text-slate-500 sm:text-[12px]">
            Account setup
          </span>
          <Link
            href="/product"
            className="flex min-h-11 min-w-[3.25rem] items-center justify-end rounded-lg px-2 text-[13px] font-medium text-violet-300/90 transition hover:bg-white/[0.05] hover:text-violet-200 active:bg-white/[0.07]"
          >
            Home
          </Link>
        </div>
      </header>

      <div className="mx-auto max-w-lg px-4 pb-8 pt-6 sm:px-6 sm:pb-16 sm:pt-10">
        <nav className="mb-10 flex justify-center gap-1.5" aria-label="Progress">
          {steps.map((label, i) => {
            const active = i === step;
            const done = i < step;
            return (
              <div key={label} className="flex flex-1 flex-col items-center gap-1.5">
                <div
                  className={
                    "flex h-7 w-full max-w-[4.5rem] items-center justify-center rounded-full text-[10px] font-semibold uppercase tracking-[0.08em] " +
                    (active
                      ? "bg-gradient-to-r from-sky-500/90 to-violet-500/90 text-white shadow-[0_0_16px_-4px_rgba(56,189,248,0.4)]"
                      : done
                        ? "bg-slate-800 text-slate-300"
                        : "bg-slate-900/80 text-slate-600")
                  }
                >
                  {i + 1}
                </div>
                <span className={"hidden text-[10px] sm:block " + (active ? "text-sky-200/90" : "text-slate-600")}>
                  {label}
                </span>
              </div>
            );
          })}
        </nav>

        <AnimatePresence mode="wait">
          {step === 0 ? (
            <motion.div
              key="s0"
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.35 }}
              className="text-center"
            >
              <div className="mx-auto mb-6 flex h-16 w-16 items-center justify-center rounded-2xl border border-sky-500/20 bg-sky-500/10 shadow-[0_0_40px_-12px_rgba(56,189,248,0.5)]">
                <span className="text-2xl" aria-hidden>
                  ◈
                </span>
              </div>
              <h1 className="text-balance text-2xl font-medium tracking-[-0.03em] text-white sm:text-[1.65rem]">
                Your digital twin starts with a card
              </h1>
              <p className="mx-auto mt-4 max-w-md text-pretty text-[15px] leading-relaxed text-slate-400">
                Pick your card type and complete a one-tap handshake. Then add{' '}
                <strong className="font-medium text-slate-300">email &amp; phone</strong> plus documents in{' '}
                <Link href="/dashboard" className="text-violet-300 underline-offset-2 hover:underline">
                  your dashboard
                </Link>{' '}
                so your NFC twin stays grounded on the real you.
              </p>
              <button
                type="button"
                onClick={goNext}
                className="mt-10 h-12 w-full rounded-full bg-sky-500 text-[15px] font-medium text-white shadow-[0_0_28px_-6px_rgba(14,165,233,0.5)] transition hover:bg-sky-400 sm:max-w-xs sm:mx-auto sm:flex sm:justify-center"
              >
                Continue
              </button>
              <p className="mt-6 text-[12px] text-slate-600">
                Pairing below only saves this device preference. Your profile lives on kg-engine. Optional:{" "}
                <Link href="/onboarding?card=nfc" className="text-sky-400/90 underline-offset-2 hover:underline">
                  NFC deep link
                </Link>
                ,{" "}
                <Link href="/onboarding?card=wifi" className="text-violet-300/90 underline-offset-2 hover:underline">
                  Wi‑Fi deep link
                </Link>
                .
              </p>
            </motion.div>
          ) : null}

          {step === 1 ? (
            <motion.div
              key="s1"
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.35 }}
            >
              <h1 className="text-center text-xl font-medium tracking-[-0.03em] text-white sm:text-[1.4rem]">
                Which card are you setting up?
              </h1>
              <p className="mx-auto mt-3 max-w-md text-center text-[14px] leading-relaxed text-slate-500">
                Choose the hardware you plan to carry—you can swap or add sources later from the dashboard.
              </p>

              <div className="mt-8 grid gap-3">
                <button
                  type="button"
                  onClick={() => chooseCard("nfc")}
                  className={
                    "group flex w-full flex-col rounded-2xl border p-5 text-left transition " +
                    (kind === "nfc"
                      ? "border-sky-400/40 bg-sky-500/10"
                      : "border-slate-700/60 bg-slate-950/50 hover:border-sky-500/25")
                  }
                >
                  <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-sky-300/90">NFC</span>
                  <span className="mt-1 text-[16px] font-medium text-white">Business card → phone</span>
                  <span className="mt-2 text-[13px] leading-relaxed text-slate-400">
                    Tap your card to someone&apos;s phone; your twin opens instantly.
                  </span>
                </button>

                <button
                  type="button"
                  onClick={() => chooseCard("wifi")}
                  className={
                    "group flex w-full flex-col rounded-2xl border p-5 text-left transition " +
                    (kind === "wifi"
                      ? "border-violet-400/40 bg-violet-500/10"
                      : "border-slate-700/60 bg-slate-950/50 hover:border-violet-500/25")
                  }
                >
                  <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-violet-200/90">
                    Wi‑Fi
                  </span>
                  <span className="mt-1 text-[16px] font-medium text-white">Cart ↔ cart · cart → phone</span>
                  <span className="mt-2 text-[13px] leading-relaxed text-slate-400">
                    Hand off context across carts or push a digest to a shopper&apos;s device on the floor.
                  </span>
                </button>
              </div>

              <p className="mt-6 text-center text-[12px] text-slate-600">
                Deep link with a preselected type:{" "}
                <Link href="/onboarding?card=nfc" className="text-sky-400/90 underline-offset-2 hover:underline">
                  ?card=nfc
                </Link>
                {" · "}
                <Link href="/onboarding?card=wifi" className="text-violet-300/90 underline-offset-2 hover:underline">
                  ?card=wifi
                </Link>
              </p>
            </motion.div>
          ) : null}

          {step === 2 ? (
            <motion.div
              key="s2"
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.35 }}
              className="text-center"
            >
              {!kind ? (
                <>
                  <h1 className="text-xl font-medium tracking-[-0.03em] text-white sm:text-[1.4rem]">
                    Pick a card first
                  </h1>
                  <p className="mx-auto mt-3 max-w-md text-[14px] leading-relaxed text-slate-500">
                    Choose NFC or Wi‑Fi on the previous step (or open a{" "}
                    <Link href="/onboarding?card=nfc" className="text-sky-400/90 underline-offset-2 hover:underline">
                      deep link
                    </Link>
                    ).
                  </p>
                  <button
                    type="button"
                    onClick={() => setStep(1)}
                    className="mt-8 h-11 w-full rounded-full border border-slate-600/60 bg-slate-900/60 text-[14px] font-medium text-slate-200 transition hover:border-sky-400/30 sm:max-w-xs sm:mx-auto sm:flex sm:justify-center"
                  >
                    Go to card choice
                  </button>
                </>
              ) : (
                <>
                  <h1 className="text-xl font-medium tracking-[-0.03em] text-white sm:text-[1.4rem]">Pair your card</h1>
                  <p className="mx-auto mt-3 max-w-md text-[14px] leading-relaxed text-slate-500">
                    Full verification issues short-lived tokens on the server. This step only records your card type on
                    this device so onboarding can resume.
                  </p>

                  <div className="relative mx-auto mt-10 flex h-44 max-w-xs items-center justify-center">
                    <motion.div
                      className="absolute inset-0 rounded-full bg-sky-500/5"
                      animate={{ scale: pairBusy ? [1, 1.15, 1] : 1, opacity: pairBusy ? [0.4, 0.15, 0.4] : 0.25 }}
                      transition={{ duration: 1.5, repeat: pairBusy ? Infinity : 0, ease: "easeInOut" }}
                    />
                    <div className="relative flex flex-col items-center gap-3">
                      <div
                        className={
                          "flex h-20 w-20 items-center justify-center rounded-2xl border text-sm font-semibold uppercase tracking-widest " +
                          (kind === "wifi"
                            ? "border-violet-400/35 bg-violet-500/15 text-violet-100"
                            : "border-sky-400/35 bg-sky-500/15 text-sky-100")
                        }
                      >
                        {kind === "wifi" ? "Wi‑Fi" : "NFC"}
                      </div>
                      <span className="text-[12px] text-slate-500">{pairBusy ? "Pairing…" : "Ready to pair"}</span>
                    </div>
                  </div>

                  <button
                    type="button"
                    disabled={pairBusy}
                    onClick={completePairing}
                    className="mt-4 h-12 w-full rounded-full bg-gradient-to-r from-sky-500 to-violet-600 text-[15px] font-medium text-white transition enabled:hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-40 sm:max-w-xs sm:mx-auto sm:flex sm:justify-center"
                  >
                    Confirm tap / handshake
                  </button>
                  <button
                    type="button"
                    onClick={goBack}
                    className="mt-3 text-[13px] text-slate-500 underline-offset-2 hover:text-slate-400 hover:underline"
                  >
                    Change card type
                  </button>
                </>
              )}
            </motion.div>
          ) : null}

          {step === 3 ? (
            <motion.div
              key="s3"
              initial={{ opacity: 0, scale: 0.98 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
              className="text-center"
            >
              <div className="mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-full bg-emerald-500/15 text-emerald-300">
                <svg viewBox="0 0 24 24" className="h-7 w-7" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M5 13l4 4L19 7" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </div>
              <h1 className="text-xl font-medium tracking-[-0.03em] text-white sm:text-[1.45rem]">
                Card linked on this device
              </h1>
              <p className="mx-auto mt-3 max-w-md text-[14px] leading-relaxed text-slate-400">
                We saved your{" "}
                <span className="text-white">{kind === "wifi" ? "Wi‑Fi cart" : "NFC card"}</span> choice under{" "}
                <code className="rounded bg-slate-800/80 px-1.5 py-0.5 text-[12px] text-sky-200/90">{STORAGE_KEY}</code>{" "}
                on this browser. Finish your account—email, phone, PDFs—in the dashboard so your twin is accurate.
              </p>
              <div className="mx-auto mt-10 flex w-full max-w-sm flex-col gap-3">
                <Link
                  href="/dashboard"
                  className="inline-flex h-12 items-center justify-center rounded-full bg-sky-500 text-[15px] font-medium text-white transition hover:bg-sky-400"
                >
                  Open dashboard · email & phone
                </Link>
                <Link
                  href="/chat"
                  className="inline-flex h-12 items-center justify-center rounded-full border border-violet-400/30 bg-violet-500/10 text-[15px] font-medium text-violet-100 transition hover:bg-violet-500/20"
                >
                  Open twin chat
                </Link>
                <Link
                  href="/product#business-cards"
                  className="text-[13px] font-medium text-slate-500 underline-offset-2 hover:text-slate-400 hover:underline"
                >
                  Back to business cards
                </Link>
              </div>
            </motion.div>
          ) : null}
        </AnimatePresence>
      </div>
    </div>
  );
}
