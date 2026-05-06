import type { Metadata } from "next";
import { OnboardingClient } from "@/app/components/onboarding/OnboardingClient";
import type { CardKind } from "@/app/components/onboarding/OnboardingClient";

export const metadata: Metadata = {
  title: "Fluvio · Set up your card",
  description: "Set up NFC or Wi‑Fi Fluvio cards and link them to your account and dashboard.",
};

function parseCardParam(raw: string | string[] | undefined): CardKind | null {
  const v = Array.isArray(raw) ? raw[0] : raw;
  return v === "nfc" || v === "wifi" ? v : null;
}

export default async function OnboardingPage({
  searchParams,
}: {
  searchParams: Promise<{ card?: string | string[] | undefined }>;
}) {
  const sp = await searchParams;
  const initialCardKind = parseCardParam(sp.card);
  const cardKey = Array.isArray(sp.card) ? sp.card.join(",") : sp.card ?? "";

  return <OnboardingClient key={cardKey} initialCardKind={initialCardKind} />;
}
