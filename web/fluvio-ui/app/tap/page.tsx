import type { Metadata } from "next";
import { Suspense } from "react";
import { NfcTapLanding } from "@/app/components/twin/NfcTapLanding";

export const metadata: Metadata = {
  title: "Fluvio · Demo card tap",
  description:
    "Simulate tapping someone's Fluvio NFC card—in production this screen opens from a hardware tap URL.",
};

export default function TapDemoPage() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-dvh items-center justify-center bg-[#0A0A0F] text-[#888780]">Loading tap…</div>
      }
    >
      <NfcTapLanding />
    </Suspense>
  );
}
