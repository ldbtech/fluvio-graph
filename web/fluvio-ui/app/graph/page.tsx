import type { Metadata } from "next";
import { Suspense } from "react";
import { TwinWorkspaceClient } from "@/app/components/twin/TwinWorkspaceClient";

export const metadata: Metadata = {
  title: "My Network · Fluvio",
  description: "Your tap connections — twin chat grounded in shared Surreal data.",
};

export default function GraphPage() {
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
