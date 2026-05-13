"use client";

import { useEffect, useState, type ReactNode } from "react";
import { usePathname, useRouter } from "next/navigation";
import { fetchAuthMe } from "@/shared/lib/fluvioDashboardApi";

type AuthStatus = "checking" | "authorized" | "unauthorized";

export default function AuthRequiredGate({ children }: { children: ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const [authStatus, setAuthStatus] = useState<AuthStatus>("checking");

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      try {
        const me = await fetchAuthMe();
        if (cancelled) return;
        if (me) {
          setAuthStatus("authorized");
          return;
        }
      } catch {
        if (cancelled) return;
      }
      setAuthStatus("unauthorized");
      const next = pathname ? `?next=${encodeURIComponent(pathname)}` : "";
      router.replace(`/dashboard${next}`);
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [pathname, router]);

  if (authStatus !== "authorized") {
    return (
      <main className="flex min-h-screen items-center justify-center bg-zinc-950 px-6 text-center text-zinc-400">
        <p className="text-sm">Checking your session…</p>
      </main>
    );
  }
  return <>{children}</>;
}
