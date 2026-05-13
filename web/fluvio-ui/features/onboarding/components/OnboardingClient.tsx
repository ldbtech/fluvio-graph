"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { getToken, getTwinUserId } from "@/shared/lib/fluvioDashboardApi";
import type { NfcCardDesign, NfcCardThemeId, WifiPreorderShipping } from "@/shared/lib/hardwareOrders";
import { DEFAULT_NFC_ACCENT_HEX, DEFAULT_NFC_THEME_ID } from "@/shared/lib/hardwareOrders";
import { WIFI_NFC_PREORDER_ENABLED } from "@/shared/lib/onboardingFlags";
import { WIFI_PREORDER_SHIPPING_EMPTY, placeNfcCardOrder, placeWifiNfcPreorder } from "@/shared/lib/hardwareOrders";
import { CARD_THEMES, NFC_THEME_IDS_ORDERED, NfcCardFrontPreview, sanitizeAccent } from "@/shared/components/NfcCardDesignPreview";

export type PathKind = "wallet" | "nfc" | "wifi_preorder";

export type OnboardingClientProps = {
  initialPath?: PathKind | null;
};

export type { NfcCardDesign, NfcCardThemeId, WifiPreorderShipping } from "@/shared/lib/hardwareOrders";

const STORAGE_KEY = "fluvio_onboarding_v2";

const WIFI_LAUNCH_LABEL = "August 15, 2026";

const NFC_EMPTY_DESIGN: NfcCardDesign = {
  nameOnCard: "",
  titleRole: "",
  company: "",
  tagline: "",
  emailHint: "",
  logoDataUrl: null,
  themeId: DEFAULT_NFC_THEME_ID,
  accentHex: DEFAULT_NFC_ACCENT_HEX,
};

const NFC_ACCENT_PRESETS = [
  "#a78bfa",
  "#818cf8",
  "#22d3ee",
  "#34d399",
  "#fbbf24",
  "#fb7185",
  "#e4e4e7",
  "#ffffff",
] as const;

function NfcCardBackPreview({ themeId }: { themeId: NfcCardThemeId }) {
  const backBg =
    themeId === "ivory"
      ? "linear-gradient(165deg,#e4e4e7,#a1a1aa)"
      : "linear-gradient(165deg,#14141c 0%,#0a0a0f 55%,#050508 100%)";
  /** Faint manufacturer mark — tiny and low-contrast, not blurred. */
  const markColor = themeId === "ivory" ? "rgba(24,24,27,0.28)" : "rgba(255,255,255,0.2)";

  return (
    <div
      className="relative mx-auto w-full max-w-[340px] overflow-hidden rounded-[1.125rem] shadow-[0_36px_60px_-28px_rgba(0,0,0,0.75)] shadow-black/60"
      style={{ aspectRatio: "1.586 / 1", background: backBg }}
      aria-hidden
    >
      <div
        className="pointer-events-none absolute inset-0 opacity-[0.07]"
        style={{
          backgroundImage: `radial-gradient(circle at 1px 1px, ${themeId === "ivory" ? "#000" : "#fff"} 1px, transparent 0)`,
          backgroundSize: "14px 14px",
        }}
      />
      <div
        className="absolute inset-x-0 bottom-[0.5rem] flex flex-col items-center gap-[0.15rem] px-3 text-center"
        style={{ color: markColor }}
      >
        <p className="text-[5px] font-medium uppercase leading-none tracking-[0.32em] sm:text-[5.5px]">Made in USA</p>
        <p className="font-mono text-[4.5px] font-normal leading-none tracking-[0.02em] sm:text-[5px]">www.fluviome.com/</p>
      </div>
    </div>
  );
}

type Stored = {
  completedAt: string;
  pathKind: PathKind;
  nfcDesign?: NfcCardDesign;
  nfcShipping?: WifiPreorderShipping;
  wifiPreorderShipping?: WifiPreorderShipping;
};

function writeStored(
  pathKind: PathKind,
  extras?: {
    nfcDesign?: NfcCardDesign;
    nfcShipping?: WifiPreorderShipping;
    wifiPreorderShipping?: WifiPreorderShipping;
  },
) {
  const payload: Stored = {
    completedAt: new Date().toISOString(),
    pathKind,
    ...(extras?.nfcDesign ? { nfcDesign: extras.nfcDesign } : {}),
    ...(extras?.nfcShipping ? { nfcShipping: extras.nfcShipping } : {}),
    ...(extras?.wifiPreorderShipping ? { wifiPreorderShipping: extras.wifiPreorderShipping } : {}),
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
}

/** Keeps uploads under ~500KB-ish for localStorage reliability. */
async function resizeImageFileToJpegDataUrl(file: File, maxSide = 280): Promise<string> {
  const bitmap = await createImageBitmap(file);
  const ratio = Math.min(1, maxSide / Math.max(bitmap.width, bitmap.height));
  const w = Math.max(1, Math.round(bitmap.width * ratio));
  const h = Math.max(1, Math.round(bitmap.height * ratio));
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("canvas");
  ctx.drawImage(bitmap, 0, 0, w, h);
  bitmap.close();
  return canvas.toDataURL("image/jpeg", 0.82);
}

function WifiShippingPreview({ shipping }: { shipping: WifiPreorderShipping }) {
  const cityLine = [shipping.city.trim(), shipping.region.trim(), shipping.postalCode.trim()].filter(Boolean).join(", ");

  return (
    <div
      className="rounded-2xl border border-white/[0.1] bg-zinc-950/50 p-5 text-left shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]"
      aria-hidden
    >
      <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Ship to</p>
      <div className="mt-4 space-y-1.5 text-[13px] leading-snug text-zinc-300">
        <p className="font-medium text-white">{shipping.fullName.trim() || "Full name"}</p>
        {shipping.companyName.trim() ? <p className="text-zinc-400">{shipping.companyName.trim()}</p> : null}
        <p>{shipping.addressLine1.trim() || "Street address"}</p>
        {shipping.addressLine2.trim() ? <p>{shipping.addressLine2.trim()}</p> : null}
        <p>{cityLine || "City, region, postal code"}</p>
        <p>{shipping.country.trim() || "Country"}</p>
      </div>
      <div className="mt-5 space-y-1 border-t border-white/[0.06] pt-4 text-[12px] text-zinc-500">
        <p>{shipping.email.trim() || "Email"}</p>
        <p>{shipping.phone.trim() || "Phone"}</p>
      </div>
      {shipping.notes.trim() ? (
        <p className="mt-4 text-[12px] leading-relaxed text-zinc-600">Note · {shipping.notes.trim()}</p>
      ) : null}
    </div>
  );
}

function FluvioSetupMark() {
  return (
    <span
      className="relative inline-flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-md border border-violet-500/25 bg-violet-500/[0.1] sm:h-7 sm:w-7"
      aria-hidden
    >
      <svg viewBox="0 0 24 24" className="relative h-4 w-4 text-violet-200/95 sm:h-[1.125rem] sm:w-[1.125rem]" fill="none">
        <path d="M12 3.5 L4.5 8.2 L4.5 15.8 L12 20.5 L19.5 15.8 L19.5 8.2 Z" stroke="currentColor" strokeWidth="1.1" opacity="0.65" />
        <circle cx="12" cy="7.7" r="1.5" className="fill-violet-100" />
        <circle cx="8.3" cy="14.7" r="1.35" className="fill-violet-200/90" />
        <circle cx="15.7" cy="14.7" r="1.35" className="fill-violet-200/80" />
        <path d="M12 9.2 L8.3 13.3 M12 9.2 L15.7 13.3 M8.3 14.7 L15.7 14.7" stroke="currentColor" strokeWidth="1.05" opacity="0.8" />
      </svg>
    </span>
  );
}

const steps = ["Welcome", "Choose", "Finish", "Ready"] as const;

type StepIndex = 0 | 1 | 2 | 3;

function AppleWalletGlyph({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor" aria-hidden>
      <path d="M17.57 14.746c-.03 2.958 2.596 3.957 2.626 3.974-.024.068-.387 1.299-1.39 2.518-.804.968-1.666 2.054-3.036 2.072-1.327.018-1.765-.764-3.465-.764-1.7 0-2.259.743-3.486.783-1.404.036-2.489-1.383-3.297-2.348-1.83-2.344-3.239-6.596-1.354-9.478.922-1.537 2.568-2.493 4.382-2.518 1.314-.026 2.619.867 3.466.867.836 0 2.49-1.057 4.207-.922.738.029 2.834.297 4.207 2.274-.108.069-2.515 1.459-2.542 4.744zM15.554 6.852c-.72-.849-2.086-1.504-3.15-1.518 0 0-.16 1.78 1.012 3.086 1.14 1.272 3.068 1.064 3.068 1.064.004-1.12-.446-2.285-1.93-3.632z" />
    </svg>
  );
}

const inputClass =
  "mt-1.5 w-full rounded-xl border border-white/[0.1] bg-black/35 px-[0.9375rem] py-3 text-[15px] text-white placeholder:text-zinc-600 focus:border-white/[0.22] focus:outline-none focus:ring-0";

function trimShipTo(s: WifiPreorderShipping): WifiPreorderShipping {
  return {
    fullName: s.fullName.trim(),
    email: s.email.trim(),
    phone: s.phone.trim(),
    companyName: s.companyName.trim(),
    addressLine1: s.addressLine1.trim(),
    addressLine2: s.addressLine2.trim(),
    city: s.city.trim(),
    region: s.region.trim(),
    postalCode: s.postalCode.trim(),
    country: s.country.trim(),
    notes: s.notes.trim(),
  };
}

function validateShipTo(s: WifiPreorderShipping): string | null {
  const email = s.email.trim();
  if (
    !s.fullName.trim() ||
    !email ||
    !s.phone.trim() ||
    !s.addressLine1.trim() ||
    !s.city.trim() ||
    !s.postalCode.trim() ||
    !s.country.trim()
  ) {
    return "Fill every required ship field (name, email, phone, address, city, postal code, country).";
  }
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
    return "Use a valid email.";
  }
  return null;
}

function HardwareShippingInputs({
  idPrefix,
  shipping,
  setShipping,
}: {
  idPrefix: string;
  shipping: WifiPreorderShipping;
  setShipping: Dispatch<SetStateAction<WifiPreorderShipping>>;
}) {
  return (
    <div className="space-y-5">
      <div>
        <label htmlFor={`${idPrefix}-name`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
          Full name<span className="text-red-400/90">*</span>
        </label>
        <input
          id={`${idPrefix}-name`}
          value={shipping.fullName}
          onChange={(e) => setShipping((p) => ({ ...p, fullName: e.target.value }))}
          placeholder="Jordan Lee"
          className={inputClass}
          autoComplete="name"
        />
      </div>

      <div>
        <label htmlFor={`${idPrefix}-email`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
          Email<span className="text-red-400/90">*</span>
        </label>
        <input
          id={`${idPrefix}-email`}
          type="email"
          inputMode="email"
          value={shipping.email}
          onChange={(e) => setShipping((p) => ({ ...p, email: e.target.value }))}
          placeholder="you@company.com"
          className={inputClass}
          autoComplete="email"
        />
      </div>

      <div>
        <label htmlFor={`${idPrefix}-phone`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
          Phone<span className="text-red-400/90">*</span>
        </label>
        <input
          id={`${idPrefix}-phone`}
          type="tel"
          inputMode="tel"
          value={shipping.phone}
          onChange={(e) => setShipping((p) => ({ ...p, phone: e.target.value }))}
          placeholder="+1 …"
          className={inputClass}
          autoComplete="tel"
        />
      </div>

      <div>
        <label htmlFor={`${idPrefix}-company`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
          Company (optional)
        </label>
        <input
          id={`${idPrefix}-company`}
          value={shipping.companyName}
          onChange={(e) => setShipping((p) => ({ ...p, companyName: e.target.value }))}
          placeholder="Atlas Labs"
          className={inputClass}
          autoComplete="organization"
        />
      </div>

      <div>
        <label htmlFor={`${idPrefix}-line1`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
          Address line 1<span className="text-red-400/90">*</span>
        </label>
        <input
          id={`${idPrefix}-line1`}
          value={shipping.addressLine1}
          onChange={(e) => setShipping((p) => ({ ...p, addressLine1: e.target.value }))}
          placeholder="Street, building, suite"
          className={inputClass}
          autoComplete="address-line1"
        />
      </div>

      <div>
        <label htmlFor={`${idPrefix}-line2`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
          Address line 2
        </label>
        <input
          id={`${idPrefix}-line2`}
          value={shipping.addressLine2}
          onChange={(e) => setShipping((p) => ({ ...p, addressLine2: e.target.value }))}
          placeholder="Apt, floor, c/o"
          className={inputClass}
          autoComplete="address-line2"
        />
      </div>

      <div className="grid gap-5 sm:grid-cols-2">
        <div>
          <label htmlFor={`${idPrefix}-city`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
            City<span className="text-red-400/90">*</span>
          </label>
          <input
            id={`${idPrefix}-city`}
            value={shipping.city}
            onChange={(e) => setShipping((p) => ({ ...p, city: e.target.value }))}
            placeholder="City"
            className={inputClass}
            autoComplete="address-level2"
          />
        </div>
        <div>
          <label htmlFor={`${idPrefix}-region`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
            Region / state
          </label>
          <input
            id={`${idPrefix}-region`}
            value={shipping.region}
            onChange={(e) => setShipping((p) => ({ ...p, region: e.target.value }))}
            placeholder="CA"
            className={inputClass}
            autoComplete="address-level1"
          />
        </div>
      </div>

      <div className="grid gap-5 sm:grid-cols-2">
        <div>
          <label htmlFor={`${idPrefix}-postal`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
            Postal code<span className="text-red-400/90">*</span>
          </label>
          <input
            id={`${idPrefix}-postal`}
            value={shipping.postalCode}
            onChange={(e) => setShipping((p) => ({ ...p, postalCode: e.target.value }))}
            placeholder="ZIP / postal"
            className={inputClass}
            autoComplete="postal-code"
          />
        </div>
        <div>
          <label htmlFor={`${idPrefix}-country`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
            Country<span className="text-red-400/90">*</span>
          </label>
          <input
            id={`${idPrefix}-country`}
            value={shipping.country}
            onChange={(e) => setShipping((p) => ({ ...p, country: e.target.value }))}
            placeholder="United States"
            className={inputClass}
            autoComplete="country-name"
          />
        </div>
      </div>

      <div>
        <label htmlFor={`${idPrefix}-notes`} className="text-[13px] font-semibold tracking-wide text-zinc-400">
          Delivery notes (optional)
        </label>
        <textarea
          id={`${idPrefix}-notes`}
          value={shipping.notes}
          onChange={(e) => setShipping((p) => ({ ...p, notes: e.target.value }))}
          placeholder="Gate code, preferred carrier, PO number…"
          rows={3}
          className={`${inputClass} min-h-[5.25rem] resize-y`}
        />
      </div>
    </div>
  );
}

export function OnboardingClient({ initialPath = null }: OnboardingClientProps) {
  const router = useRouter();

  const [step, setStep] = useState<StepIndex>(() => (initialPath ? 2 : 0));
  const [pathKind, setPathKind] = useState<PathKind | null>(() => initialPath ?? null);
  const [nfcDesign, setNfcDesign] = useState<NfcCardDesign>(NFC_EMPTY_DESIGN);
  const [nfcLogoErr, setNfcLogoErr] = useState<string | null>(null);
  const [nfcSubmitErr, setNfcSubmitErr] = useState<string | null>(null);
  const [nfcShipping, setNfcShipping] = useState<WifiPreorderShipping>(() => ({ ...WIFI_PREORDER_SHIPPING_EMPTY }));
  const [wifiShipping, setWifiShipping] = useState<WifiPreorderShipping>(() => ({ ...WIFI_PREORDER_SHIPPING_EMPTY }));
  const [wifiSubmitErr, setWifiSubmitErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [walletErr, setWalletErr] = useState<string | null>(null);
  const [placedOrder, setPlacedOrder] = useState<{
    id: string;
    kind: "nfc_card" | "wifi_nfc_preorder";
  } | null>(null);

  const [hasOwnerId, setHasOwnerId] = useState(false);

  useEffect(() => {
    setHasOwnerId(!!(getToken() || getTwinUserId()));
  }, [step]);

  const goNext = useCallback(() => {
    setStep((s) => (s < 3 ? ((s + 1) as StepIndex) : s));
  }, []);

  const goBack = useCallback(() => {
    setStep((s) => (s > 0 ? ((s - 1) as StepIndex) : s));
  }, []);

  const choosePath = useCallback((k: PathKind) => {
    setPathKind(k);
    setWalletErr(null);
    setNfcSubmitErr(null);
    setWifiSubmitErr(null);
    setNfcLogoErr(null);
    setPlacedOrder(null);
    if (k !== "nfc") {
      setNfcDesign(NFC_EMPTY_DESIGN);
      setNfcShipping({ ...WIFI_PREORDER_SHIPPING_EMPTY });
    }
    if (k !== "wifi_preorder") setWifiShipping({ ...WIFI_PREORDER_SHIPPING_EMPTY });
    setStep(2);
  }, []);

  const onNfcLogo = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    e.target.value = "";
    if (!f) return;
    if (!f.type.startsWith("image/")) {
      setNfcLogoErr("Use an image file (PNG, JPG, WebP, etc.).");
      return;
    }
    if (f.size > 8 * 1024 * 1024) {
      setNfcLogoErr("File too large — try under 8 MB.");
      return;
    }
    setNfcLogoErr(null);
    try {
      const url = await resizeImageFileToJpegDataUrl(f);
      setNfcDesign((p) => ({ ...p, logoDataUrl: url }));
    } catch {
      setNfcLogoErr("Could not read image. Try another file.");
    }
  }, []);

  const addToAppleWallet = useCallback(async () => {
    const id = getTwinUserId();
    if (!id) return;
    setBusy(true);
    setWalletErr(null);
    try {
      const r = await fetch("/api/wallet/issue-url", { method: "POST", headers: { "X-Owner-ID": id } });
      const body = (await r.json()) as {
        pkpassUrl?: string;
        signingConfigured?: boolean;
        error?: string;
      };
      if (!r.ok || !body.pkpassUrl) {
        setWalletErr(body.error ?? `Could not prepare pass (${r.status})`);
        return;
      }
      if (!body.signingConfigured) {
        setWalletErr("Apple pass signing isn’t configured on this server yet. Use the dashboard for status.");
        return;
      }
      window.location.assign(body.pkpassUrl);
    } catch {
      setWalletErr("Network error while requesting the pass.");
    } finally {
      setBusy(false);
    }
  }, []);

  const completeFinish = useCallback(() => {
    if (!pathKind) return;

    if (pathKind === "nfc") {
      const name = nfcDesign.nameOnCard.trim();
      const company = nfcDesign.company.trim();
      if (!name || !company) {
        setNfcSubmitErr("Add your name and company—those print on every card.");
        return;
      }
      const shipErr = validateShipTo(nfcShipping);
      if (shipErr) {
        setNfcSubmitErr(shipErr);
        return;
      }
      setNfcSubmitErr(null);
    }

    if (pathKind === "wifi_preorder") {
      const shipErr = validateShipTo(wifiShipping);
      if (shipErr) {
        setWifiSubmitErr(shipErr);
        return;
      }
      setWifiSubmitErr(null);
    }

    const ownerSnap = typeof window !== "undefined" ? getTwinUserId() : null;

    setBusy(true);
    window.setTimeout(() => {
      setBusy(false);

      if (pathKind === "nfc") {
        const ship = trimShipTo(nfcShipping);
        const designOut: NfcCardDesign = { ...nfcDesign, emailHint: ship.email };
        const o = placeNfcCardOrder(designOut, ownerSnap, ship);
        setPlacedOrder({ id: o.id, kind: "nfc_card" });
        setNfcDesign(designOut);
        writeStored("nfc", { nfcDesign: designOut, nfcShipping: ship });
      } else if (pathKind === "wifi_preorder") {
        const ship = trimShipTo(wifiShipping);
        const o = placeWifiNfcPreorder(ownerSnap, WIFI_LAUNCH_LABEL, ship);
        setPlacedOrder({ id: o.id, kind: "wifi_nfc_preorder" });
        writeStored("wifi_preorder", { wifiPreorderShipping: ship });
      } else {
        writeStored(pathKind);
        setPlacedOrder(null);
      }
      setStep(3);
    }, pathKind === "wifi_preorder" ? 900 : pathKind === "nfc" ? 1100 : 1250);
  }, [pathKind, nfcDesign, nfcShipping, wifiShipping]);

  const contentWide =
    step === 2 && (pathKind === "nfc" || (WIFI_NFC_PREORDER_ENABLED && pathKind === "wifi_preorder"));

  return (
    <div className="relative min-h-dvh bg-[#09090b] pb-[max(1.5rem,env(safe-area-inset-bottom))] text-zinc-100 antialiased">
      <div
        className="pointer-events-none fixed inset-0 -z-10 bg-[radial-gradient(ellipse_80%_50%_at_50%_-20%,rgba(255,255,255,0.05),transparent_52%)]"
        aria-hidden
      />

      <header className="sticky top-0 z-30 border-b border-white/[0.06] bg-[#09090b]/85 backdrop-blur-2xl pt-[max(0.35rem,env(safe-area-inset-top))]">
        <div className="mx-auto grid h-14 max-w-5xl grid-cols-[1fr_auto_1fr] items-center gap-2 px-4 sm:h-[3.5rem] sm:px-8">
          <button
            type="button"
            onClick={() => (step === 0 ? router.push("/") : goBack())}
            className="min-h-11 justify-self-start rounded-lg px-2 text-[15px] font-medium text-white/55 transition hover:bg-white/[0.05] hover:text-white active:bg-white/[0.07]"
          >
            {step === 0 ? "Close" : "Back"}
          </button>
          <div className="flex min-w-0 max-w-full items-center justify-center gap-2 justify-self-center">
            <FluvioSetupMark />
            <span className="truncate text-[11px] font-medium tracking-[0.12em] text-zinc-500 sm:text-[12px] sm:text-zinc-600">
              FLUVIOME SETUP
            </span>
          </div>
          <Link
            href="/"
            className="flex min-h-11 items-center justify-self-end rounded-lg px-2 text-[15px] font-medium text-white/70 transition hover:bg-white/[0.05] hover:text-white active:bg-white/[0.07]"
          >
            Home
          </Link>
        </div>
      </header>

      <div className={`mx-auto px-5 pb-14 pt-8 sm:px-8 sm:pb-24 sm:pt-12 ${contentWide ? "max-w-5xl" : "max-w-lg"}`}>
        <nav className="mb-12 flex justify-center gap-2 sm:gap-4" aria-label="Progress">
          {steps.map((label, i) => {
            const active = i === step;
            const done = i < step;
            return (
              <div key={label} className="flex flex-1 flex-col items-center gap-2">
                <div
                  className={`flex h-8 w-full max-w-[5rem] items-center justify-center rounded-full text-[11px] font-semibold uppercase tracking-[0.1em] ${
                    active
                      ? "bg-white text-zinc-950"
                      : done
                        ? "bg-white/[0.1] text-zinc-300"
                        : "bg-white/[0.04] text-zinc-600"
                  }`}
                >
                  {i + 1}
                </div>
                <span className={"hidden text-[10px] sm:block " + (active ? "text-zinc-300" : "text-zinc-600")}>{label}</span>
              </div>
            );
          })}
        </nav>

        <AnimatePresence mode="wait">
          {step === 0 ? (
            <motion.div
              key="s0"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.28 }}
              className="text-center"
            >
              <p className="text-[13px] font-semibold uppercase tracking-[0.16em] text-zinc-500">FluvioMe</p>
              <h1 className="mt-4 text-balance text-[1.875rem] font-semibold tracking-[-0.04em] text-white sm:text-[2rem]">
                Put yourself in Wallet.
              </h1>
              <p className="mx-auto mt-5 max-w-md text-[17px] leading-relaxed text-zinc-400">
                {WIFI_NFC_PREORDER_ENABLED ? (
                  <>
                    Next, pick how you’ll show up: Apple Wallet today, NFC plastic you design here, or the new Wi‑Fi NFC card—we’ll
                    line it up.
                  </>
                ) : (
                  <>
                    Next, pick how you’ll show up: Apple Wallet today or NFC plastic you design here. Wi‑Fi NFC card—available
                    soon (around {WIFI_LAUNCH_LABEL}).
                  </>
                )}
              </p>
              <button
                type="button"
                onClick={goNext}
                className="mx-auto mt-12 flex h-12 w-full max-w-xs items-center justify-center rounded-full bg-white text-[15px] font-semibold text-zinc-950 transition hover:bg-zinc-100 active:bg-zinc-200"
              >
                Continue
              </button>
            </motion.div>
          ) : null}

          {step === 1 ? (
            <motion.div
              key="s1"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.28 }}
            >
              <h1 className="text-center text-xl font-semibold tracking-[-0.03em] text-white sm:text-[1.375rem]">
                How should they open you?
              </h1>
              <p className="mx-auto mt-3 max-w-md text-center text-[15px] leading-relaxed text-zinc-500">
                Pick one to start—we’re not forcing an order tonight.
              </p>

              <div className="mt-10 flex flex-col gap-3">
                <button
                  type="button"
                  onClick={() => choosePath("wallet")}
                  className={`flex w-full flex-col rounded-2xl border px-6 py-5 text-left transition sm:rounded-3xl sm:py-6 ${
                    pathKind === "wallet"
                      ? "border-white/25 bg-white/[0.06]"
                      : "border-white/[0.07] bg-zinc-900/40 hover:border-white/15 hover:bg-white/[0.03]"
                  }`}
                >
                  <div className="flex items-start gap-4">
                    <AppleWalletGlyph className="mt-0.5 h-10 w-10 shrink-0 text-white/80" />
                    <div>
                      <span className="text-[13px] font-semibold uppercase tracking-[0.12em] text-zinc-400">Apple Wallet</span>
                      <p className="mt-1 text-[17px] font-semibold tracking-[-0.02em] text-white">Add your pass now</p>
                      <p className="mt-2 text-[14px] leading-relaxed text-zinc-500">
                        Opens in Safari on iPhone—Google Wallet pairing runs from the dashboard on supported devices too.
                      </p>
                    </div>
                  </div>
                </button>

                <button
                  type="button"
                  onClick={() => choosePath("nfc")}
                  className={`flex w-full flex-col rounded-2xl border px-6 py-5 text-left transition sm:rounded-3xl sm:py-6 ${
                    pathKind === "nfc"
                      ? "border-white/25 bg-white/[0.06]"
                      : "border-white/[0.07] bg-zinc-900/40 hover:border-white/15 hover:bg-white/[0.03]"
                  }`}
                >
                  <span className="text-[13px] font-semibold uppercase tracking-[0.12em] text-zinc-400">NFC card</span>
                  <p className="mt-1 text-[17px] font-semibold tracking-[-0.02em] text-white">Design & order a tap card</p>
                  <p className="mt-2 text-[14px] leading-relaxed text-zinc-500">
                    Name, title, logo—your digital business card on plastic. Ships after you confirm.
                  </p>
                </button>

                {WIFI_NFC_PREORDER_ENABLED ? (
                  <button
                    type="button"
                    onClick={() => choosePath("wifi_preorder")}
                    className={`flex w-full flex-col rounded-2xl border px-6 py-5 text-left transition sm:rounded-3xl sm:py-6 ${
                      pathKind === "wifi_preorder"
                        ? "border-white/25 bg-white/[0.06]"
                        : "border-white/[0.07] bg-zinc-900/40 hover:border-white/15 hover:bg-white/[0.03]"
                    }`}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <span className="text-[13px] font-semibold uppercase tracking-[0.12em] text-zinc-400">
                          Wi‑Fi NFC card · New
                        </span>
                        <p className="mt-1 text-[17px] font-semibold tracking-[-0.02em] text-white">Pre-order</p>
                        <p className="mt-2 text-[14px] leading-relaxed text-zinc-500">
                          Ships with richer handoffs for retail aisles. Available {WIFI_LAUNCH_LABEL}.
                        </p>
                      </div>
                      <span className="shrink-0 rounded-full border border-white/[0.1] bg-white/[0.04] px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.1em] text-zinc-400">
                        Pre-order
                      </span>
                    </div>
                  </button>
                ) : (
                  <div
                    role="note"
                    aria-label="Wi‑Fi NFC card coming soon"
                    className="flex w-full cursor-not-allowed flex-col rounded-2xl border border-dashed border-white/[0.1] bg-zinc-950/40 px-6 py-5 text-left opacity-[0.72] sm:rounded-3xl sm:py-6"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <span className="text-[13px] font-semibold uppercase tracking-[0.12em] text-zinc-500">
                          Wi‑Fi NFC card · New
                        </span>
                        <p className="mt-1 text-[17px] font-semibold tracking-[-0.02em] text-zinc-300">Will be available soon</p>
                        <p className="mt-2 text-[14px] leading-relaxed text-zinc-600">
                          We&apos;re finishing the design. Approximate availability: {WIFI_LAUNCH_LABEL}.
                        </p>
                      </div>
                      <span className="shrink-0 rounded-full border border-white/[0.08] bg-white/[0.03] px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.1em] text-zinc-500">
                        Soon
                      </span>
                    </div>
                  </div>
                )}
              </div>

              <p className="mt-10 text-center text-[12px] text-zinc-600">
                Direct links:{" "}
                <Link href="/onboarding?path=wallet" className="text-zinc-400 underline-offset-2 hover:underline">
                  Apple
                </Link>
                {" · "}
                <Link href="/onboarding?path=nfc" className="text-zinc-400 underline-offset-2 hover:underline">
                  NFC order
                </Link>
                {" · "}
                {WIFI_NFC_PREORDER_ENABLED ? (
                  <Link href="/onboarding?path=wifi" className="text-zinc-400 underline-offset-2 hover:underline">
                    Wi‑Fi pre-order
                  </Link>
                ) : (
                  <span className="cursor-not-allowed text-zinc-600">Wi‑Fi card (soon)</span>
                )}
              </p>
            </motion.div>
          ) : null}

          {step === 2 ? (
            <motion.div
              key="s2"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.28 }}
              className={
                pathKind === "nfc" || (WIFI_NFC_PREORDER_ENABLED && pathKind === "wifi_preorder")
                  ? "text-left"
                  : "text-center"
              }
            >
              {!pathKind ? (
                <>
                  <h2 className="text-center text-xl font-semibold text-white">Choose a path</h2>
                  <button
                    type="button"
                    onClick={() => setStep(1)}
                    className="mx-auto mt-10 flex h-11 w-full max-w-xs items-center justify-center rounded-full border border-white/[0.1] bg-white/[0.04] text-[15px] font-medium text-white transition hover:bg-white/[0.08]"
                  >
                    Go back to choices
                  </button>
                </>
              ) : pathKind === "wallet" ? (
                <div className="text-center">
                  <div className="mx-auto mb-8 flex justify-center">
                    <AppleWalletGlyph className="h-16 w-16 text-white/85" />
                  </div>
                  <h2 className="text-xl font-semibold tracking-[-0.03em] text-white sm:text-[1.375rem]">Add to Wallet</h2>
                  <p className="mx-auto mt-4 max-w-md text-[15px] leading-relaxed text-zinc-500">
                    Uses the same encrypted pass installer as dashboard. Already finished twin setup here? Tap below—otherwise hop
                    to the dashboard once and come back.
                  </p>

                  {walletErr ? (
                    <p className="mx-auto mt-6 max-w-md rounded-xl border border-amber-500/25 bg-amber-500/[0.08] px-4 py-3 text-left text-[13px] leading-relaxed text-amber-100/95">
                      {walletErr}
                    </p>
                  ) : null}

                  {hasOwnerId ? (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void addToAppleWallet()}
                      className="mx-auto mt-10 flex min-h-[52px] w-full max-w-xs items-center justify-center gap-2 rounded-full bg-white px-5 text-[15px] font-semibold text-zinc-950 transition hover:bg-zinc-100 disabled:cursor-not-allowed disabled:opacity-40"
                      aria-label="Add to Apple Wallet"
                    >
                      <AppleWalletGlyph className="h-6 w-6 text-zinc-950" />
                      {busy ? "Preparing…" : "Add to Apple Wallet"}
                    </button>
                  ) : (
                    <Link
                      href="/dashboard"
                      className="mx-auto mt-10 flex min-h-[52px] w-full max-w-xs items-center justify-center rounded-full bg-white text-[15px] font-semibold text-zinc-950 transition hover:bg-zinc-100"
                    >
                      Open dashboard · get pass ready
                    </Link>
                  )}

                  <button
                    type="button"
                    onClick={() => void completeFinish()}
                    className="mx-auto mt-4 block text-[14px] font-medium text-white/45 underline underline-offset-4 hover:text-white/65"
                  >
                    Done for now · save my choice on this browser
                  </button>
                  <button
                    type="button"
                    onClick={() => setStep(1)}
                    className="mx-auto mt-6 block text-[13px] text-zinc-500 underline-offset-2 hover:text-zinc-400 hover:underline"
                  >
                    Change path
                  </button>
                </div>
              ) : pathKind === "nfc" ? (
                <div>
                  <h2 className="text-[1.25rem] font-semibold tracking-[-0.03em] text-white sm:text-[1.375rem]">
                    Design your NFC card
                  </h2>
                  <p className="mt-3 max-w-xl text-[15px] leading-relaxed text-zinc-500">
                    This is how your twin shows up after a tap—same story as Wallet, formatted like the business card in their
                    hand.
                  </p>

                  <div className="mt-10 grid gap-12 lg:grid-cols-[minmax(0,1fr)_minmax(260px,360px)] lg:items-start">
                    <fieldset className="min-w-0 space-y-5 border-none p-0">
                      <legend className="sr-only">Card imprint</legend>

                      <div>
                        <label htmlFor="nfc-name" className="text-[13px] font-semibold tracking-wide text-zinc-400">
                          Name on card<span className="text-red-400/90">*</span>
                        </label>
                        <input
                          id="nfc-name"
                          value={nfcDesign.nameOnCard}
                          onChange={(e) => setNfcDesign((p) => ({ ...p, nameOnCard: e.target.value }))}
                          placeholder="Jordan Lee"
                          className={inputClass}
                          autoComplete="name"
                        />
                      </div>

                      <div>
                        <label htmlFor="nfc-title" className="text-[13px] font-semibold tracking-wide text-zinc-400">
                          Title / role
                        </label>
                        <input
                          id="nfc-title"
                          value={nfcDesign.titleRole}
                          onChange={(e) => setNfcDesign((p) => ({ ...p, titleRole: e.target.value }))}
                          placeholder="Founder · Product"
                          className={inputClass}
                        />
                      </div>

                      <div>
                        <label htmlFor="nfc-company" className="text-[13px] font-semibold tracking-wide text-zinc-400">
                          Company<span className="text-red-400/90">*</span>
                        </label>
                        <input
                          id="nfc-company"
                          value={nfcDesign.company}
                          onChange={(e) => setNfcDesign((p) => ({ ...p, company: e.target.value }))}
                          placeholder="Atlas Labs"
                          className={inputClass}
                          autoComplete="organization"
                        />
                      </div>

                      <div>
                        <label htmlFor="nfc-tagline" className="text-[13px] font-semibold tracking-wide text-zinc-400">
                          Tagline
                        </label>
                        <textarea
                          id="nfc-tagline"
                          value={nfcDesign.tagline}
                          onChange={(e) => setNfcDesign((p) => ({ ...p, tagline: e.target.value }))}
                          placeholder="Brief line under your company—what you do in one breath."
                          rows={3}
                          className={`${inputClass} min-h-[5.25rem] resize-y`}
                        />
                      </div>

                      <div>
                        <p className="text-[13px] font-semibold tracking-wide text-zinc-400">Card finish</p>
                        <p className="mt-1 text-[12px] leading-relaxed text-zinc-600">
                          Background for the physical card. Front stays clean—branding lives on the back.
                        </p>
                        <div className="mt-3 flex flex-wrap gap-2">
                          {NFC_THEME_IDS_ORDERED.map((id) => {
                            const meta = CARD_THEMES[id];
                            const active = nfcDesign.themeId === id;
                            return (
                              <button
                                key={id}
                                type="button"
                                onClick={() => setNfcDesign((p) => ({ ...p, themeId: id }))}
                                className={`flex min-w-0 items-center gap-2.5 rounded-xl border px-3 py-2.5 text-left transition ${
                                  active
                                    ? "border-violet-400/50 bg-violet-500/[0.12] text-white"
                                    : "border-white/[0.1] bg-white/[0.03] text-zinc-300 hover:border-white/[0.16] hover:bg-white/[0.06]"
                                }`}
                              >
                                <span
                                  className="size-9 shrink-0 rounded-lg border border-white/10 shadow-inner shadow-black/40"
                                  style={{ background: meta.swatch }}
                                  aria-hidden
                                />
                                <span className="text-[13px] font-semibold">{meta.label}</span>
                              </button>
                            );
                          })}
                        </div>
                      </div>

                      <div>
                        <p className="text-[13px] font-semibold tracking-wide text-zinc-400">Accent color</p>
                        <p className="mt-1 text-[12px] leading-relaxed text-zinc-600">
                          Highlights the role line, hairline, and logo frame on the front.
                        </p>
                        <div className="mt-3 flex flex-wrap items-center gap-2">
                          {NFC_ACCENT_PRESETS.map((hex) => {
                            const active = sanitizeAccent(nfcDesign.accentHex).toLowerCase() === hex.toLowerCase();
                            return (
                              <button
                                key={hex}
                                type="button"
                                title={hex}
                                onClick={() => setNfcDesign((p) => ({ ...p, accentHex: hex }))}
                                className={`size-9 rounded-full border-2 transition ${
                                  active ? "border-white ring-2 ring-violet-400/60 ring-offset-2 ring-offset-zinc-950" : "border-black/20"
                                }`}
                                style={{ backgroundColor: hex }}
                                aria-label={`Accent ${hex}`}
                              />
                            );
                          })}
                          <label className="ml-1 flex cursor-pointer items-center gap-2 rounded-xl border border-white/[0.1] bg-white/[0.04] px-3 py-2 text-[12px] text-zinc-400 transition hover:bg-white/[0.07]">
                            <span className="font-medium text-zinc-500">Custom</span>
                            <input
                              type="color"
                              value={sanitizeAccent(nfcDesign.accentHex)}
                              onChange={(e) => setNfcDesign((p) => ({ ...p, accentHex: e.target.value }))}
                              className="h-8 w-10 cursor-pointer rounded border-0 bg-transparent p-0"
                              aria-label="Pick custom accent"
                            />
                          </label>
                        </div>
                      </div>

                      <div>
                        <p className="text-[13px] font-semibold tracking-wide text-zinc-400">Company logo</p>
                        <p className="mt-1 text-[12px] leading-relaxed text-zinc-600">
                          PNG, JPG, or WebP. Saved with your on-site order and shown in the dashboard while we prep print.
                        </p>
                        <div className="mt-3 flex flex-wrap items-center gap-4">
                          <label className="inline-flex cursor-pointer items-center justify-center rounded-full border border-white/[0.12] bg-white/[0.04] px-5 py-3 text-[14px] font-semibold text-white transition hover:bg-white/[0.08]">
                            Choose file
                            <input type="file" accept="image/*" className="sr-only" onChange={(e) => void onNfcLogo(e)} />
                          </label>
                          {nfcDesign.logoDataUrl ? (
                            <button
                              type="button"
                              className="text-[13px] font-medium text-zinc-500 underline-offset-4 hover:text-zinc-300 hover:underline"
                              onClick={() => setNfcDesign((p) => ({ ...p, logoDataUrl: null }))}
                            >
                              Remove logo
                            </button>
                          ) : null}
                        </div>
                        {nfcLogoErr ? (
                          <p className="mt-2 text-[13px] text-amber-200/95" role="status">
                            {nfcLogoErr}
                          </p>
                        ) : null}
                      </div>

                      <div className="border-t border-white/[0.06] pt-10">
                        <p className="text-[13px] font-semibold tracking-wide text-zinc-300">Where we ship your cards</p>
                        <p className="mt-1 text-[12px] leading-relaxed text-zinc-600">
                          Full address required—same checklist as Wi‑Fi pre-orders. Your ship-to email is stored on the order for
                          confirmation.
                        </p>
                        <div className="mt-6">
                          <HardwareShippingInputs idPrefix="nfc-ship" shipping={nfcShipping} setShipping={setNfcShipping} />
                        </div>
                      </div>
                    </fieldset>

                    <div className="space-y-10 lg:sticky lg:top-[6.75rem]">
                      <div>
                        <p className="mb-5 text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Live preview</p>
                        <div className="space-y-8">
                          <div>
                            <p className="mb-3 text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600">Front</p>
                            <NfcCardFrontPreview design={nfcDesign} />
                          </div>
                          <div>
                            <p className="mb-3 text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600">Back</p>
                            <NfcCardBackPreview themeId={nfcDesign.themeId} />
                          </div>
                        </div>
                      </div>
                      <div>
                        <p className="mb-5 text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Ship to</p>
                        <WifiShippingPreview shipping={nfcShipping} />
                      </div>
                    </div>
                  </div>

                  {nfcSubmitErr ? (
                    <p className="mt-10 text-[14px] text-amber-200/95 lg:max-w-xl" role="alert">
                      {nfcSubmitErr}
                    </p>
                  ) : null}

                  <div className="mt-10 flex flex-col gap-5 border-t border-white/[0.06] pt-10">
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void completeFinish()}
                      className="flex min-h-[3rem] w-full max-w-md items-center justify-center rounded-full bg-white text-[15px] font-semibold text-zinc-950 transition hover:bg-zinc-100 disabled:opacity-40"
                    >
                      {busy ? "Placing…" : "Place order on site"}
                    </button>
                    <p className="max-w-md text-[13px] leading-relaxed text-zinc-600">
                      We store this order on this browser immediately; open{" "}
                      <Link href="/dashboard" className="font-semibold text-zinc-400 underline-offset-4 hover:text-white hover:underline">
                        Dashboard
                      </Link>{" "}
                      anytime to watch status updates (next step is plugging in real fulfilment).
                    </p>
                  </div>

                  <button
                    type="button"
                    onClick={() => setStep(1)}
                    className="mt-10 text-[13px] text-zinc-500 underline-offset-2 hover:text-zinc-400 hover:underline"
                  >
                    Change path
                  </button>
                </div>
              ) : (
                <div>
                  <div className="mb-6 inline-flex rounded-full border border-white/[0.08] bg-white/[0.04] px-4 py-2 text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-400">
                    Wi‑Fi NFC · {WIFI_LAUNCH_LABEL}
                  </div>
                  <h2 className="text-[1.25rem] font-semibold tracking-[-0.03em] text-white sm:text-[1.375rem]">
                    Pre-order · ship-to
                  </h2>
                  <p className="mt-3 max-w-xl text-[15px] leading-relaxed text-zinc-500">
                    Same on-device order queue as NFC cards—add your full address so we can ship when the batch is ready (
                    {WIFI_LAUNCH_LABEL}). Tracking shows in the dashboard when fulfilment posts it.
                  </p>

                  <div className="mt-10 grid gap-12 lg:grid-cols-[minmax(0,1fr)_minmax(260px,360px)] lg:items-start">
                    <fieldset className="min-w-0 border-none p-0">
                      <legend className="sr-only">Shipping address</legend>
                      <HardwareShippingInputs idPrefix="wifi" shipping={wifiShipping} setShipping={setWifiShipping} />
                    </fieldset>

                    <div className="lg:sticky lg:top-[6.75rem]">
                      <p className="mb-5 text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Preview</p>
                      <WifiShippingPreview shipping={wifiShipping} />
                    </div>
                  </div>

                  {wifiSubmitErr ? (
                    <p className="mt-10 text-[14px] text-amber-200/95 lg:max-w-xl" role="alert">
                      {wifiSubmitErr}
                    </p>
                  ) : null}

                  <div className="mt-10 flex flex-col gap-5 border-t border-white/[0.06] pt-10">
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void completeFinish()}
                      className="flex min-h-[3rem] w-full max-w-md items-center justify-center rounded-full bg-white text-[15px] font-semibold text-zinc-950 transition hover:bg-zinc-100 disabled:opacity-40"
                    >
                      {busy ? "Placing…" : "Submit pre-order"}
                    </button>
                    <p className="max-w-md text-[13px] leading-relaxed text-zinc-600">
                      Order id and shipping status appear in{" "}
                      <Link href="/dashboard" className="font-semibold text-zinc-400 underline-offset-4 hover:text-white hover:underline">
                        Dashboard
                      </Link>
                      —same hardware queue as NFC.
                    </p>
                  </div>

                  <button
                    type="button"
                    onClick={() => setStep(1)}
                    className="mt-10 text-[13px] text-zinc-500 underline-offset-2 hover:text-zinc-400 hover:underline"
                  >
                    Change path
                  </button>
                </div>
              )}
            </motion.div>
          ) : null}

          {step === 3 ? (
            <motion.div
              key="s3"
              initial={{ opacity: 0, scale: 0.99 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
              className="text-center"
            >
              <div className="mx-auto mb-6 flex h-14 w-14 items-center justify-center rounded-full border border-white/10 bg-white/[0.05] text-white">
                <svg viewBox="0 0 24 24" className="h-7 w-7" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M5 13l4 4L19 7" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </div>
              <h2 className="text-xl font-semibold tracking-[-0.03em] text-white sm:text-[1.4rem]">
                {pathKind === "wifi_preorder"
                  ? "You’re on the Wi‑Fi NFC list"
                  : pathKind === "nfc"
                    ? "Order placed"
                    : "Wallet path saved"}
              </h2>
              <p className="mx-auto mt-4 max-w-md text-[15px] leading-relaxed text-zinc-500">
                {pathKind === "wifi_preorder" ? (
                  <>
                    You’re queued for the batch shipping around{" "}
                    <span className="text-zinc-300">{WIFI_LAUNCH_LABEL}</span>. Status updates surface in your dashboard—no separate
                    email thread required.
                  </>
                ) : pathKind === "nfc" ? (
                  <>
                    Imprint and logo are committed to fulfilment—the same snapshot below is persisted with order{" "}
                    {placedOrder ? (
                      <code className="rounded-md border border-white/[0.08] bg-white/[0.04] px-1.5 py-0.5 font-mono text-[11px] text-zinc-300">
                        {placedOrder.id}
                      </code>
                    ) : null}
                    . On-device backup:{" "}
                    <code className="rounded-md border border-white/[0.08] bg-white/[0.04] px-1.5 py-0.5 font-mono text-[11px] text-zinc-400">{STORAGE_KEY}</code>
                    .
                  </>
                ) : (
                  <>Wallet install lives in the dashboard until Apple signing is live on your stack.</>
                )}{" "}
                {pathKind === "wallet" ? (
                  <>
                    Saved as{" "}
                    <code className="rounded-md border border-white/[0.08] bg-white/[0.04] px-1.5 py-0.5 font-mono text-[11px] text-zinc-400">{STORAGE_KEY}</code>.
                  </>
                ) : null}
                {pathKind === "wifi_preorder" && placedOrder ? (
                  <span className="mt-3 block font-mono text-[11px] text-zinc-500">Order {placedOrder.id}</span>
                ) : null}
              </p>

              {pathKind === "nfc" ? (
                <div className="mx-auto mt-10 max-w-sm text-left">
                  <div className="space-y-8">
                    <div>
                      <p className="mb-3 text-center text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600">Front</p>
                      <NfcCardFrontPreview design={nfcDesign} />
                    </div>
                    <div>
                      <p className="mb-3 text-center text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-600">Back</p>
                      <NfcCardBackPreview themeId={nfcDesign.themeId} />
                    </div>
                  </div>
                  <div className="mt-8">
                    <p className="mb-3 text-center text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Ship to</p>
                    <WifiShippingPreview shipping={nfcShipping} />
                  </div>
                  <Link
                    href="/dashboard"
                    className="mt-8 inline-flex h-12 w-full items-center justify-center rounded-full border border-white/[0.12] bg-white/[0.06] text-[15px] font-semibold text-white transition hover:bg-white/[0.1]"
                  >
                    View order status
                  </Link>
                </div>
              ) : null}

              {pathKind === "wifi_preorder" ? (
                <div className="mx-auto mt-10 max-w-sm text-left">
                  <WifiShippingPreview shipping={wifiShipping} />
                  <Link
                    href="/dashboard"
                    className="mt-8 inline-flex h-12 w-full items-center justify-center rounded-full border border-white/[0.12] bg-white/[0.06] text-[15px] font-semibold text-white transition hover:bg-white/[0.1]"
                  >
                    View order & tracking
                  </Link>
                </div>
              ) : null}

              <div className="mx-auto mt-12 flex w-full max-w-sm flex-col gap-3">
                <Link
                  href="/dashboard"
                  className="inline-flex h-12 items-center justify-center rounded-full bg-white text-[15px] font-semibold text-zinc-950 transition hover:bg-zinc-100"
                >
                  Open dashboard
                </Link>
                <Link
                  href="/graph"
                  className="inline-flex h-12 items-center justify-center rounded-full border border-white/[0.1] bg-transparent text-[15px] font-medium text-white transition hover:bg-white/[0.05]"
                >
                  My Network
                </Link>
                <Link href="/" className="pt-2 text-[14px] font-medium text-zinc-500 underline-offset-4 hover:text-zinc-400 hover:underline">
                  Back to FluvioMe
                </Link>
              </div>
            </motion.div>
          ) : null}
        </AnimatePresence>
      </div>
    </div>
  );
}
