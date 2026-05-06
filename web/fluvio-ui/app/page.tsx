import type { Metadata } from "next";
import { LandingPage } from "@/app/components/landing/LandingPage";

export const metadata: Metadata = {
  title: "Fluvio · Personal AI twin",
  description:
    "Teach a digital twin from mail, repos, and files—then share it via wallet passes or NFC when you're ready. Create your twin or explore the product.",
};

export default function Home() {
  return <LandingPage />;
}
