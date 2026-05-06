import type { Metadata } from "next";
import { DashboardClient } from "@/app/components/dashboard/DashboardClient";

export const metadata: Metadata = {
  title: "Dashboard · Fluvio",
  description: "Your Fluvio profile, ingested documents, NFC connections, and twin graph settings.",
};

export default function DashboardPage() {
  return <DashboardClient />;
}
