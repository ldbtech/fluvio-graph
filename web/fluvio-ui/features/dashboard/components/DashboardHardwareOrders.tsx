"use client";

import Link from "next/link";
import { motion } from "framer-motion";
import { useCallback, useEffect, useState } from "react";
import {
  deleteHardwareOrder,
  hardwareOrderStatusLabel,
  hardwareOrdersForSession,
  listHardwareOrders,
  subscribeHardwareOrders,
  type HardwareOrder,
  type WifiPreorderShipping,
} from "@/shared/lib/hardwareOrders";
import { WIFI_NFC_PREORDER_ENABLED } from "@/shared/lib/onboardingFlags";

type Props = {
  /** Resolved owner id when logged in; null before hydrate or logged out */
  sessionOwnerId: string | null;
};

function formatShippingBlock(s: WifiPreorderShipping) {
  const contact = [s.email.trim(), s.phone.trim()].filter(Boolean).join(" · ");
  const lines = [
    s.fullName.trim(),
    s.companyName.trim() || null,
    s.addressLine1.trim(),
    s.addressLine2.trim() || null,
    [s.city.trim(), s.region.trim(), s.postalCode.trim()].filter(Boolean).join(", ") || null,
    s.country.trim(),
    contact || null,
  ].filter(Boolean) as string[];
  return lines;
}

/** Legacy NFC rows in localStorage may have empty ship-to. */
function hasShipSnapshot(s: WifiPreorderShipping): boolean {
  return !!(s.fullName.trim() && s.email.trim() && s.addressLine1.trim());
}

function formatWhen(iso: string) {
  try {
    const d = new Date(iso);
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(d);
  } catch {
    return iso;
  }
}

type TrackingLink = { label: string; href: string };

/** Build outbound tracking URLs from saved carrier name and/or common id shapes (UPS 1Z…, USPS IMpb-style). */
function trackingLinks(carrierRaw: string | null | undefined, trackingNumber: string | null | undefined): TrackingLink[] {
  const tn = trackingNumber?.trim() ?? "";
  if (!tn) return [];

  const c = (carrierRaw ?? "").trim().toLowerCase();
  const tnNorm = tn.replace(/\s+/g, "");
  const alpha = tnNorm.replace(/[^0-9A-Za-z]/g, "");
  const out: TrackingLink[] = [];

  const push = (label: string, href: string) => out.push({ label, href });

  const fedex = c.includes("fedex") || c.includes("fdx");
  const usps = c.includes("usps") || c.includes("united states postal") || c.includes("u.s. postal");
  const ups = c.includes("ups") || tnNorm.toUpperCase().startsWith("1Z");
  const dhl = c.includes("dhl");

  const uspsLikelyPrefix =
    /^(92|93|94|420|927|928|930|935|936|937|938|920|921|922|923|924|926|931|932|933|934)/.test(alpha);

  if (fedex) {
    push("Track on FedEx", `https://www.fedex.com/fedextrack/?trknbr=${encodeURIComponent(tnNorm)}`);
  }
  if (ups) {
    push("Track on UPS", `https://www.ups.com/track?tracknum=${encodeURIComponent(tnNorm)}`);
  }
  if (dhl) {
    push(
      "Track on DHL",
      `https://www.dhl.com/global-en/home/tracking/tracking-express.html?tracking-id=${encodeURIComponent(tnNorm)}`,
    );
  }
  if (usps) {
    push("Track on USPS", `https://tools.usps.com/go/TrackConfirmAction?tLabels=${encodeURIComponent(tnNorm)}`);
  }

  /** If carrier omitted, USPS-style numbers still get Tools USPS (conservative prefix + length). */
  if (
    out.length === 0 &&
    !ups &&
    uspsLikelyPrefix &&
    alpha.length >= 20
  ) {
    push("Track on USPS", `https://tools.usps.com/go/TrackConfirmAction?tLabels=${encodeURIComponent(tnNorm)}`);
  }

  const seen = new Set<string>();
  return out.filter((link) => (seen.has(link.href) ? false : (seen.add(link.href), true)));
}

function OrderFulfillmentRibbon() {
  return (
    <div className="mb-8">
      <div className="relative h-[3px] overflow-hidden rounded-full bg-white/[0.06]">
        <motion.span
          className="absolute inset-y-0 w-[34%] rounded-full bg-[#c4b5fd] shadow-[0_0_18px_rgba(167,139,250,0.72)]"
          initial={false}
          animate={{ left: ["-40%", "115%"] }}
          transition={{ repeat: Infinity, duration: 2.1, ease: "linear" }}
        />
      </div>
    </div>
  );
}

function OrderDetailModal(props: {
  order: HardwareOrder;
  open: boolean;
  onClose: () => void;
}) {
  const { order: o, open, onClose } = props;
  const links = trackingLinks(o.carrier ?? null, o.trackingNumber ?? null);
  const hasTracking = !!(o.trackingNumber?.trim() || o.carrier?.trim());

  useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener("keydown", onKey);
    };
  }, [open, onClose]);

  if (!open) return null;

  const headline =
    o.kind === "nfc_card"
      ? `Tap card · ${o.design.company.trim() || o.design.nameOnCard}`
      : "Wi‑Fi tap card";

  return (
    <div className="fixed inset-0 z-50 flex items-end justify-center p-4 sm:items-center" role="presentation">
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-[2px]"
        aria-hidden
        onClick={onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="order-modal-title"
        className="relative z-10 w-full max-w-md rounded-[22px] border border-white/[0.08] bg-zinc-950 p-6 shadow-[0_24px_80px_rgba(0,0,0,0.55)] sm:p-8"
      >
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <p className="font-mono text-[11px] text-zinc-600">{o.id}</p>
            <h2 id="order-modal-title" className="mt-1 text-[1.2rem] font-semibold tracking-[-0.03em] text-white">
              {headline}
            </h2>
            <p className="mt-2 text-[14px] text-zinc-500">{formatWhen(o.createdAt)}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="shrink-0 rounded-full p-2 text-zinc-500 transition hover:bg-white/[0.06] hover:text-white"
            aria-label="Close"
          >
            <span aria-hidden className="text-[22px] leading-none">
              ×
            </span>
          </button>
        </div>

        <p className="mt-6 text-[13px] font-medium text-zinc-600">Status</p>
        <p className="mt-2 inline-flex rounded-full border border-violet-500/25 bg-violet-500/10 px-3 py-1.5 text-[13px] font-medium text-violet-200/95">
          {hardwareOrderStatusLabel(o.status)}
        </p>

        {o.kind === "nfc_card" ? (
          <div className="mt-6">
            <p className="text-[13px] font-medium text-zinc-600">Card</p>
            <p className="mt-2 text-[15px] leading-relaxed text-zinc-300">
              <span>{o.design.nameOnCard.trim()}</span>
              {o.design.titleRole.trim() ? ` · ${o.design.titleRole.trim()}` : ""}
              {!hasShipSnapshot(o.shipping) && o.design.emailHint.trim() ? (
                <>
                  {" "}
                  · <span className="text-zinc-500">{o.design.emailHint.trim()}</span>
                </>
              ) : null}
              {o.design.logoDataUrl ? (
                <span className="ml-2 rounded border border-violet-500/20 bg-violet-500/5 px-1.5 py-0.5 text-[10px] font-medium text-violet-300/90">
                  Logo
                </span>
              ) : (
                <span className="ml-2 text-[12px] text-zinc-600">No logo</span>
              )}
            </p>
          </div>
        ) : (
          <p className="mt-6 text-[15px] text-zinc-400">
            Ships <span className="text-zinc-200">{o.etaLabel}</span>
          </p>
        )}

        {formatShippingBlock(o.shipping).length > 0 && (
          <div className="mt-6">
            <p className="text-[13px] font-medium text-zinc-600">Ship to</p>
            <ul className="mt-2 space-y-1 text-[14px] leading-snug text-zinc-400">
              {formatShippingBlock(o.shipping).map((line, i) => (
                <li key={i}>{line}</li>
              ))}
            </ul>
            {o.shipping.notes.trim() ? (
              <p className="mt-3 text-[13px] text-zinc-500">Note · {o.shipping.notes.trim()}</p>
            ) : null}
          </div>
        )}

        {hasTracking ? (
          <div className="mt-6 border-t border-white/[0.06] pt-6">
            <p className="text-[13px] font-medium text-zinc-600">Tracking</p>
            {o.carrier?.trim() ? <p className="mt-2 text-[14px] text-zinc-300">{o.carrier.trim()}</p> : null}
            {o.trackingNumber?.trim() ? (
              <p className="mt-2 font-mono text-[13px] text-violet-300">{o.trackingNumber.trim()}</p>
            ) : (
              !o.carrier?.trim() && (
                <p className="mt-2 text-[13px] text-zinc-500">No tracking number saved on this device yet.</p>
              )
            )}
            {links.length > 0 ? (
              <div className="mt-4 flex flex-col gap-2">
                {links.map((l) => (
                  <a
                    key={l.href}
                    href={l.href}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex justify-center rounded-full border border-violet-500/35 bg-violet-500/10 px-4 py-3 text-[14px] font-medium text-violet-200 transition hover:bg-violet-500/18"
                  >
                    {l.label}
                  </a>
                ))}
              </div>
            ) : o.trackingNumber?.trim() ? (
              <p className="mt-3 text-[13px] text-zinc-500">
                Carrier not recognized—we only show USPS, FedEx, UPS, or DHL deep links here. Copy the code above or
                paste it into your carrier’s site.
              </p>
            ) : null}
          </div>
        ) : (
          <p className="mt-6 border-t border-white/[0.06] pt-6 text-[14px] leading-relaxed text-zinc-500">
            Tracking and carrier links appear here once we attach them to this order—same bright line on the dashboard
            will quietly keep pulsing meanwhile.
          </p>
        )}

        <div className="mt-8 flex flex-col gap-2 sm:flex-row sm:justify-between sm:gap-3">
          <button
            type="button"
            onClick={() => {
              if (
                !confirm(
                  "Remove this order from this device? It only lives in your browser storage—this cannot be undone here.",
                )
              )
                return;
              deleteHardwareOrder(o.id);
              onClose();
            }}
            className="rounded-full px-2 py-2.5 text-[14px] font-medium text-zinc-500 underline-offset-4 transition hover:bg-white/[0.04] hover:text-red-400/90 hover:underline"
          >
            Remove from this device
          </button>
          <button
            type="button"
            autoFocus
            onClick={onClose}
            className="rounded-full bg-white px-5 py-3 text-[15px] font-semibold text-zinc-950 transition hover:bg-zinc-100"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}

export function DashboardHardwareOrders({ sessionOwnerId }: Props) {
  const [orders, setOrders] = useState<HardwareOrder[]>([]);
  const [modalOrderId, setModalOrderId] = useState<string | null>(null);

  const refresh = useCallback(() => {
    setOrders(hardwareOrdersForSession(listHardwareOrders(), sessionOwnerId));
  }, [sessionOwnerId]);

  useEffect(() => {
    queueMicrotask(() => refresh());
    return subscribeHardwareOrders(refresh);
  }, [sessionOwnerId, refresh]);

  const modalOrder = modalOrderId ? orders.find((x) => x.id === modalOrderId) ?? null : null;

  if (orders.length === 0) {
    return (
      <section className="rounded-[20px] border border-dashed border-white/[0.08] bg-white/[0.02] px-6 py-8 sm:px-8">
        <h2 className="text-[15px] font-semibold tracking-[-0.02em] text-white">Orders</h2>
        <OrderFulfillmentRibbon />
        <p className="mb-2 max-w-md text-[15px] leading-relaxed text-zinc-500">
          Tap cards you buy here appear here—saved on this device until fulfilment plugs in live status.
        </p>
        <p className="mt-6 flex flex-wrap gap-x-4 gap-y-2 text-[15px] font-medium">
          <Link href="/onboarding?path=nfc" className="text-violet-400 underline-offset-4 hover:text-violet-300 hover:underline">
            NFC card
          </Link>
          <span className="text-zinc-600" aria-hidden>
            ·
          </span>
          {WIFI_NFC_PREORDER_ENABLED ? (
            <Link href="/onboarding?path=wifi" className="text-violet-400 underline-offset-4 hover:text-violet-300 hover:underline">
              Wi‑Fi card
            </Link>
          ) : (
            <span className="text-zinc-600">Wi‑Fi card (soon)</span>
          )}
        </p>
      </section>
    );
  }

  return (
    <section className="rounded-[20px] border border-white/[0.06] bg-white/[0.02] px-6 py-7 sm:px-8 sm:py-8">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-[15px] font-semibold tracking-[-0.02em] text-white">Orders</h2>
        <Link
          href="/onboarding"
          className="text-[14px] font-medium text-violet-400 underline-offset-4 hover:text-violet-300 hover:underline"
        >
          Another order
        </Link>
      </div>

      <OrderFulfillmentRibbon />

      <ul className="space-y-0 divide-y divide-white/[0.06]">
        {orders.map((o) => (
          <li key={o.id} className="grid grid-cols-[1fr_auto] items-center gap-x-4 gap-y-3 py-4 first:pt-0">
            <div className="min-w-0">
              <p className="text-[16px] font-medium tracking-[-0.02em] text-white">
                {o.kind === "nfc_card"
                  ? `Tap card · ${o.design.company.trim() || o.design.nameOnCard}`
                  : "Wi‑Fi tap card"}
              </p>
              <p className="mt-2">
                <span className="inline-flex rounded-full border border-violet-500/20 bg-violet-500/[0.08] px-2.5 py-0.5 text-[11px] font-medium text-violet-200/95">
                  {hardwareOrderStatusLabel(o.status)}
                </span>
              </p>
            </div>
            <button
              type="button"
              onClick={() => setModalOrderId(o.id)}
              className="shrink-0 rounded-full border border-white/[0.1] bg-white/[0.05] px-4 py-2.5 text-[13px] font-medium text-white transition hover:bg-white/[0.1]"
            >
              View status
            </button>
          </li>
        ))}
      </ul>

      {modalOrder ? (
        <OrderDetailModal order={modalOrder} open={!!modalOrder} onClose={() => setModalOrderId(null)} />
      ) : null}
    </section>
  );
}
