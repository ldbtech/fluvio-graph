import type { Metadata } from "next";
import { DashboardClient } from "@/features/dashboard/components/DashboardClient";

export const metadata: Metadata = {
  title: "Overview · FluvioMe",
  description: "Your profile, Wallet pass, orders, and the people you've met.",
};

export default function DashboardPage() {
  return <DashboardClient />;
}
