import type { Metadata } from "next";
import { LandingPage } from "@/app/components/landing/LandingPage";

export const metadata: Metadata = {
  title: "Fluvio · Create your digital twin",
  description:
    "Teach a personal AI twin from your mail, repos, and files. Start with an Apple Wallet or Google Wallet pass—no checkout or hardware required. Optional NFC and retail flows when you want them.",
};

export default function ProductPage() {
  return <LandingPage />;
}
