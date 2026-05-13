import type { Metadata } from "next";
import { OnboardingClient } from "@/features/onboarding/components/OnboardingClient";
import type { PathKind } from "@/features/onboarding/components/OnboardingClient";
import { WIFI_NFC_PREORDER_ENABLED } from "@/shared/lib/onboardingFlags";

export const metadata: Metadata = {
  title: "Set up · FluvioMe",
  description: WIFI_NFC_PREORDER_ENABLED
    ? "Add your FluvioMe pass to Apple Wallet—or design your NFC card—plus Wi‑Fi NFC pre‑order (August 15, 2026)."
    : "Add your FluvioMe pass to Apple Wallet—or design your NFC tap card. Wi‑Fi NFC variant coming soon.",
};

function parsePathKind(raw: string | undefined): PathKind | null {
  const v = raw?.trim().toLowerCase();
  if (v === "wallet" || v === "apple") return "wallet";
  if (v === "nfc") return "nfc";
  if (WIFI_NFC_PREORDER_ENABLED && (v === "wifi" || v === "wifi_preorder")) return "wifi_preorder";
  return null;
}

export default async function OnboardingPage({
  searchParams,
}: {
  searchParams: Promise<{ path?: string | string[]; card?: string | string[] }>;
}) {
  const sp = await searchParams;

  const pathRaw = Array.isArray(sp.path) ? sp.path[0] : sp.path;
  const cardRaw = Array.isArray(sp.card) ? sp.card[0] : sp.card;
  const initialPath = parsePathKind(pathRaw) ?? parsePathKind(cardRaw);

  const qpKey =
    typeof pathRaw === "string" ? pathRaw : typeof cardRaw === "string" ? cardRaw ?? "" : "";

  return <OnboardingClient key={qpKey || "default"} initialPath={initialPath} />;
}
