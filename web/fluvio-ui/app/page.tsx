import type { Metadata } from "next";
import { LandingPage } from "@/app/components/landing/LandingPage";

export const metadata: Metadata = {
  title: "FluvioMe - Your introduction in Wallet",
  description: "Wallet or tap. Your AI twin—not another dead contact.",
};

export default function Home() {
  return <LandingPage />;
}
