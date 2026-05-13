"use client";

import type { NfcCardDesign, NfcCardThemeId } from "@/shared/lib/hardwareOrders";
import { DEFAULT_NFC_ACCENT_HEX } from "@/shared/lib/hardwareOrders";

type CardThemeStyle = {
  label: string;
  swatch: string;
  bg: string;
  logoBg: string;
  nameColor: string;
  companyColor: string;
  taglineColor: string;
  logoPlaceholderClass: string;
};

export const CARD_THEMES: Record<NfcCardThemeId, CardThemeStyle> = {
  carbon: {
    label: "Carbon",
    swatch: "linear-gradient(145deg,#27272a,#09090b)",
    bg: "linear-gradient(155deg,#1c1c22 0%,#0f0f12 42%,#080809 100%)",
    logoBg: "rgba(255,255,255,0.04)",
    nameColor: "#fafafa",
    companyColor: "#d4d4d8",
    taglineColor: "#a1a1aa",
    logoPlaceholderClass: "text-zinc-600",
  },
  midnight: {
    label: "Midnight",
    swatch: "linear-gradient(145deg,#1e3a5f,#0c1220)",
    bg: "linear-gradient(155deg,#152238 0%,#0c1424 48%,#070b14 100%)",
    logoBg: "rgba(147,197,253,0.06)",
    nameColor: "#f8fafc",
    companyColor: "#cbd5e1",
    taglineColor: "#94a3b8",
    logoPlaceholderClass: "text-slate-500",
  },
  wine: {
    label: "Wine",
    swatch: "linear-gradient(145deg,#7f1d1d,#1c0a0c)",
    bg: "linear-gradient(155deg,#3d151d 0%,#1f0a10 45%,#0f0508 100%)",
    logoBg: "rgba(254,202,202,0.06)",
    nameColor: "#fef2f2",
    companyColor: "#fecaca",
    taglineColor: "#fca5a5",
    logoPlaceholderClass: "text-red-300/50",
  },
  forest: {
    label: "Forest",
    swatch: "linear-gradient(145deg,#14532d,#07150c)",
    bg: "linear-gradient(155deg,#1a2e1f 0%,#0f1a12 45%,#080f0a 100%)",
    logoBg: "rgba(187,247,208,0.06)",
    nameColor: "#f0fdf4",
    companyColor: "#bbf7d0",
    taglineColor: "#86efac",
    logoPlaceholderClass: "text-emerald-400/40",
  },
  navy: {
    label: "Navy",
    swatch: "linear-gradient(145deg,#1e3a8a,#0a0f1f)",
    bg: "linear-gradient(155deg,#172554 0%,#0f172a 48%,#070d1a 100%)",
    logoBg: "rgba(191,219,254,0.06)",
    nameColor: "#f8fafc",
    companyColor: "#bfdbfe",
    taglineColor: "#93c5fd",
    logoPlaceholderClass: "text-blue-300/45",
  },
  ivory: {
    label: "Ivory",
    swatch: "linear-gradient(145deg,#fafafa,#d4d4d8)",
    bg: "linear-gradient(160deg,#fafafa 0%,#e4e4e7 42%,#d4d4d8 100%)",
    logoBg: "rgba(24,24,27,0.06)",
    nameColor: "#18181b",
    companyColor: "#3f3f46",
    taglineColor: "#52525b",
    logoPlaceholderClass: "text-zinc-500",
  },
};

export const NFC_THEME_IDS_ORDERED = Object.keys(CARD_THEMES) as NfcCardThemeId[];

export function sanitizeAccent(hex: string): string {
  return /^#[0-9A-Fa-f]{6}$/.test(hex) ? hex : DEFAULT_NFC_ACCENT_HEX;
}

export type NfcCardFrontPreviewProps = {
  design: NfcCardDesign;
  /** Bottom “Tap · FluvioMe / Wallet” strip (dashboard Wallet preview). */
  walletFooter?: boolean;
};

export function NfcCardFrontPreview({ design, walletFooter }: NfcCardFrontPreviewProps) {
  const t = CARD_THEMES[design.themeId] ?? CARD_THEMES.carbon;
  const accent = sanitizeAccent(design.accentHex);
  const name = design.nameOnCard.trim() || "Your name";
  const role = design.titleRole.trim();
  const company = design.company.trim() || "Company";
  const tagline = design.tagline.trim();
  const hairline = `linear-gradient(90deg, transparent, ${accent}55, transparent)`;

  return (
    <div
      className="relative mx-auto w-full max-w-[340px] overflow-hidden rounded-[1.125rem] shadow-[0_36px_60px_-28px_rgba(0,0,0,0.85)] shadow-black/70"
      style={{
        aspectRatio: "1.586 / 1",
        background: t.bg,
        borderWidth: 1,
        borderStyle: "solid",
        borderColor: design.themeId === "ivory" ? "rgba(24,24,27,0.14)" : `${accent}44`,
      }}
      aria-hidden
    >
      <div className="absolute inset-x-9 top-3 h-px" style={{ background: hairline }} />
      <div className="absolute left-[1.125rem] top-[2.125rem] right-[1.125rem] flex gap-4">
        <div
          className="flex size-[3.375rem] shrink-0 overflow-hidden rounded-[0.625rem] shadow-inner shadow-black/30"
          style={{
            borderWidth: 1,
            borderStyle: "solid",
            borderColor: `${accent}66`,
            background: t.logoBg,
          }}
        >
          {design.logoDataUrl ? (
            <img src={design.logoDataUrl} alt="" className="h-full w-full object-cover" />
          ) : (
            <div
              className={`flex h-full w-full items-center justify-center text-[9px] font-semibold uppercase tracking-[0.06em] ${t.logoPlaceholderClass}`}
            >
              Logo
            </div>
          )}
        </div>
        <div className="min-w-0 flex-1 text-left">
          <p className="truncate text-[1.0625rem] font-semibold leading-tight tracking-[-0.02em]" style={{ color: t.nameColor }}>
            {name}
          </p>
          {role ? (
            <p className="mt-1 truncate text-[11px] font-medium uppercase tracking-[0.1em]" style={{ color: accent }}>
              {role}
            </p>
          ) : (
            <p className="mt-1 text-[11px]" style={{ color: accent, opacity: 0.55 }}>
              Role
            </p>
          )}
          <p className="mt-3 truncate text-[13px] font-medium" style={{ color: t.companyColor }}>
            {company}
          </p>
          {tagline ? (
            <p className="mt-2 line-clamp-2 text-[11px] leading-snug" style={{ color: t.taglineColor }}>
              {tagline}
            </p>
          ) : (
            <p className="mt-2 text-[11px]" style={{ color: t.taglineColor, opacity: 0.65 }}>
              Short tagline
            </p>
          )}
        </div>
      </div>
      {walletFooter ? (
        <div className="absolute bottom-[1rem] left-[1.125rem] right-[1.125rem] flex items-end justify-between gap-2 border-t border-white/[0.06] pt-[0.75rem]">
          <span className="font-mono text-[9px] tracking-wide text-zinc-600">Tap · FluvioMe</span>
          <span className="text-[9px] font-semibold uppercase tracking-[0.14em] text-zinc-500">Wallet</span>
        </div>
      ) : null}
    </div>
  );
}
