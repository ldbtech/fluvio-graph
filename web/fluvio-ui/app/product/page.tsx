import type { Metadata } from "next";
import { LandingPage } from "@/app/components/landing/LandingPage";

export const metadata: Metadata = {
  title: "FluvioMe",
  description: "Wallet pass or tap card—the same introduction every time.",
};

export default function ProductPage() {
  return <LandingPage />;
}
