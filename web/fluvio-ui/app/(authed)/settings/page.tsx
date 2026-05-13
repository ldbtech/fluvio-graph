import type { Metadata } from "next";
import AuthRequiredGate from "@/app/components/AuthRequiredGate";
import { SettingsClient } from "@/features/settings/components/SettingsClient";

export const metadata: Metadata = {
  title: "Settings · FluvioMe",
  description: "Control who sees your graph, manage account and NFC, and privacy zones.",
};

export default function SettingsPage() {
  return (
    <AuthRequiredGate>
      <SettingsClient />
    </AuthRequiredGate>
  );
}

