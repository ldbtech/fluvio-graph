import type { Metadata } from "next";
import { Suspense } from "react";
import { TwinWorkspaceClient } from "@/app/components/twin/TwinWorkspaceClient";

export const metadata: Metadata = {
  title: "Ask Ali · Fluvio",
  description: "Chat with Ali's AI twin alongside your connection graph.",
};

export default function ChatPage() {
  return (
    <Suspense
      fallback={
        <div className="flex min-h-dvh items-center justify-center bg-[#0A0A0F] px-6 text-[#888780]">
          Loading…
        </div>
      }
    >
      <TwinWorkspaceClient />
    </Suspense>
  );
}
