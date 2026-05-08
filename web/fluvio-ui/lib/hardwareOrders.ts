/**
 * FluvioMe hardware orders (NFC tap cards, Wi‑Fi NFC pre‑orders).
 * Stored in localStorage until a fulfilment backend exists—same-origin only.
 */

/** Preset card faces for print + preview (stored on each order). */
export type NfcCardThemeId = "carbon" | "midnight" | "wine" | "forest" | "navy" | "ivory";

export type NfcCardDesign = {
  nameOnCard: string;
  titleRole: string;
  company: string;
  tagline: string;
  emailHint: string;
  logoDataUrl: string | null;
  themeId: NfcCardThemeId;
  /** Highlight for role line, hairline, logo frame — `#RRGGBB`. */
  accentHex: string;
};

const NFC_THEME_IDS: NfcCardThemeId[] = ["carbon", "midnight", "wine", "forest", "navy", "ivory"];

export const DEFAULT_NFC_THEME_ID: NfcCardThemeId = "carbon";
export const DEFAULT_NFC_ACCENT_HEX = "#a78bfa";

function parseThemeId(raw: unknown): NfcCardThemeId {
  return typeof raw === "string" && NFC_THEME_IDS.includes(raw as NfcCardThemeId)
    ? (raw as NfcCardThemeId)
    : DEFAULT_NFC_THEME_ID;
}

function parseAccentHex(raw: unknown): string {
  if (typeof raw === "string" && /^#[0-9A-Fa-f]{6}$/.test(raw)) return raw;
  return DEFAULT_NFC_ACCENT_HEX;
}

/** Ship-to for Wi‑Fi NFC pre-orders (same fulfilment path as NFC cards). */
export type WifiPreorderShipping = {
  fullName: string;
  email: string;
  phone: string;
  companyName: string;
  addressLine1: string;
  addressLine2: string;
  city: string;
  region: string;
  postalCode: string;
  country: string;
  notes: string;
};

export type HardwareOrderStatus =
  | "submitted"
  | "in_review"
  | "in_production"
  | "shipped"
  | "delivered"
  | "cancelled";

const STORAGE_KEY = "fluvio_hardware_orders_v1";

export type NfcCardOrder = {
  id: string;
  kind: "nfc_card";
  createdAt: string;
  updatedAt: string;
  status: HardwareOrderStatus;
  ownerId: string | null;
  design: NfcCardDesign;
  /** Full ship-to captured at checkout (legacy rows may omit). */
  shipping: WifiPreorderShipping;
  carrier?: string | null;
  trackingNumber?: string | null;
};

export type WifiNfcPreorderOrder = {
  id: string;
  kind: "wifi_nfc_preorder";
  createdAt: string;
  updatedAt: string;
  status: HardwareOrderStatus;
  ownerId: string | null;
  /** e.g. "August 15, 2026" shown in onboarding */
  etaLabel: string;
  shipping: WifiPreorderShipping;
  carrier?: string | null;
  trackingNumber?: string | null;
};

export type HardwareOrder = NfcCardOrder | WifiNfcPreorderOrder;

const CHANNEL = "fluvio-hardware-orders-changed";

export const WIFI_PREORDER_SHIPPING_EMPTY: WifiPreorderShipping = {
  fullName: "",
  email: "",
  phone: "",
  companyName: "",
  addressLine1: "",
  addressLine2: "",
  city: "",
  region: "",
  postalCode: "",
  country: "",
  notes: "",
};

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null;
}

function parseShipping(raw: unknown): WifiPreorderShipping {
  if (!isRecord(raw)) return { ...WIFI_PREORDER_SHIPPING_EMPTY };
  const s = WIFI_PREORDER_SHIPPING_EMPTY;
  return {
    fullName: typeof raw.fullName === "string" ? raw.fullName : s.fullName,
    email: typeof raw.email === "string" ? raw.email : s.email,
    phone: typeof raw.phone === "string" ? raw.phone : s.phone,
    companyName: typeof raw.companyName === "string" ? raw.companyName : s.companyName,
    addressLine1: typeof raw.addressLine1 === "string" ? raw.addressLine1 : s.addressLine1,
    addressLine2: typeof raw.addressLine2 === "string" ? raw.addressLine2 : s.addressLine2,
    city: typeof raw.city === "string" ? raw.city : s.city,
    region: typeof raw.region === "string" ? raw.region : s.region,
    postalCode: typeof raw.postalCode === "string" ? raw.postalCode : s.postalCode,
    country: typeof raw.country === "string" ? raw.country : s.country,
    notes: typeof raw.notes === "string" ? raw.notes : s.notes,
  };
}

function parseOrder(raw: unknown): HardwareOrder | null {
  if (!isRecord(raw)) return null;
  const id = typeof raw.id === "string" ? raw.id : null;
  const kind = raw.kind;
  const createdAt = typeof raw.createdAt === "string" ? raw.createdAt : null;
  const updatedAt = typeof raw.updatedAt === "string" ? raw.updatedAt : createdAt;
  const status = raw.status as HardwareOrderStatus;
  const ownerId = raw.ownerId === null || typeof raw.ownerId === "string" ? raw.ownerId : null;
  if (!id || !createdAt || !updatedAt) return null;

  const validStatus: HardwareOrderStatus[] = [
    "submitted",
    "in_review",
    "in_production",
    "shipped",
    "delivered",
    "cancelled",
  ];
  const st = validStatus.includes(status) ? status : "submitted";
  const carrier = raw.carrier === null || typeof raw.carrier === "string" ? raw.carrier : null;
  const trackingNumber =
    raw.trackingNumber === null || typeof raw.trackingNumber === "string" ? raw.trackingNumber : null;

  if (kind === "wifi_nfc_preorder") {
    const etaLabel = typeof raw.etaLabel === "string" ? raw.etaLabel : "";
    const shipping = "shipping" in raw ? parseShipping(raw.shipping) : { ...WIFI_PREORDER_SHIPPING_EMPTY };
    return {
      id,
      kind: "wifi_nfc_preorder",
      createdAt,
      updatedAt,
      status: st,
      ownerId,
      etaLabel,
      shipping,
      carrier: carrier ?? undefined,
      trackingNumber: trackingNumber ?? undefined,
    };
  }

  if (kind === "nfc_card" && isRecord(raw.design)) {
    const d = raw.design;
    const design: NfcCardDesign = {
      nameOnCard: typeof d.nameOnCard === "string" ? d.nameOnCard : "",
      titleRole: typeof d.titleRole === "string" ? d.titleRole : "",
      company: typeof d.company === "string" ? d.company : "",
      tagline: typeof d.tagline === "string" ? d.tagline : "",
      emailHint: typeof d.emailHint === "string" ? d.emailHint : "",
      logoDataUrl: d.logoDataUrl === null || typeof d.logoDataUrl === "string" ? d.logoDataUrl : null,
      themeId: parseThemeId(d.themeId),
      accentHex: parseAccentHex(d.accentHex),
    };
    const shipping = "shipping" in raw ? parseShipping(raw.shipping) : { ...WIFI_PREORDER_SHIPPING_EMPTY };
    return {
      id,
      kind: "nfc_card",
      createdAt,
      updatedAt,
      status: st,
      ownerId,
      design,
      shipping,
      carrier: carrier ?? undefined,
      trackingNumber: trackingNumber ?? undefined,
    };
  }

  return null;
}

function readAllUnsafe(): HardwareOrder[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
    if (!Array.isArray(raw)) return [];
    return raw.map(parseOrder).filter((x): x is HardwareOrder => x !== null);
  } catch {
    return [];
  }
}

function writeAll(orders: HardwareOrder[]) {
  if (typeof window === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(orders));
  window.dispatchEvent(new Event(CHANNEL));
}

export function listHardwareOrders(): HardwareOrder[] {
  return readAllUnsafe();
}

/** Guest orders (no ownerId) stay visible after you sign in on the same browser. */
export function hardwareOrdersForSession(orders: HardwareOrder[], sessionOwnerId: string | null): HardwareOrder[] {
  if (!sessionOwnerId) return orders;
  return orders.filter((o) => o.ownerId == null || o.ownerId === sessionOwnerId);
}

export function hardwareOrderStatusLabel(status: HardwareOrderStatus): string {
  switch (status) {
    case "submitted":
      return "Received";
    case "in_review":
      return "In review";
    case "in_production":
      return "In production";
    case "shipped":
      return "Shipped";
    case "delivered":
      return "Delivered";
    case "cancelled":
      return "Cancelled";
    default:
      return status;
  }
}

function newId(prefix: string) {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
}

export function placeNfcCardOrder(
  design: NfcCardDesign,
  ownerId: string | null,
  shipping: WifiPreorderShipping,
): NfcCardOrder {
  const now = new Date().toISOString();
  const order: NfcCardOrder = {
    id: newId("nfc"),
    kind: "nfc_card",
    createdAt: now,
    updatedAt: now,
    status: "submitted",
    ownerId,
    design: { ...design },
    shipping: { ...shipping },
    carrier: null,
    trackingNumber: null,
  };
  const all = readAllUnsafe();
  all.unshift(order);
  writeAll(all);
  return order;
}

export function placeWifiNfcPreorder(
  ownerId: string | null,
  etaLabel: string,
  shipping: WifiPreorderShipping,
): WifiNfcPreorderOrder {
  const now = new Date().toISOString();
  const order: WifiNfcPreorderOrder = {
    id: newId("wifi"),
    kind: "wifi_nfc_preorder",
    createdAt: now,
    updatedAt: now,
    status: "submitted",
    ownerId,
    etaLabel,
    shipping: { ...shipping },
    carrier: null,
    trackingNumber: null,
  };
  const all = readAllUnsafe();
  all.unshift(order);
  writeAll(all);
  return order;
}

/** Fulfilment / admin: update status and optional carrier tracking (persists to localStorage). */
export function patchHardwareOrderFulfillment(
  orderId: string,
  patch: {
    status?: HardwareOrderStatus;
    carrier?: string | null;
    trackingNumber?: string | null;
  },
): boolean {
  const all = readAllUnsafe();
  const i = all.findIndex((o) => o.id === orderId);
  if (i < 0) return false;
  const o = all[i];
  const now = new Date().toISOString();
  if (patch.status != null) {
    (o as NfcCardOrder | WifiNfcPreorderOrder).status = patch.status;
  }
  if ("carrier" in patch) {
    (o as NfcCardOrder | WifiNfcPreorderOrder).carrier = patch.carrier ?? null;
  }
  if ("trackingNumber" in patch) {
    (o as NfcCardOrder | WifiNfcPreorderOrder).trackingNumber = patch.trackingNumber ?? null;
  }
  (o as NfcCardOrder | WifiNfcPreorderOrder).updatedAt = now;
  all[i] = o;
  writeAll(all);
  return true;
}

/** Remove one order from this browser’s queue (localStorage). */
export function deleteHardwareOrder(orderId: string): boolean {
  const all = readAllUnsafe();
  const next = all.filter((o) => o.id !== orderId);
  if (next.length === all.length) return false;
  writeAll(next);
  return true;
}

/** Subscribe to same-tab updates after placeOrder/writeAll */
export function subscribeHardwareOrders(cb: () => void) {
  if (typeof window === "undefined") return () => {};
  const onCustom = () => cb();
  const onStorage = (e: StorageEvent) => {
    if (e.key === STORAGE_KEY) cb();
  };
  window.addEventListener(CHANNEL, onCustom);
  window.addEventListener("storage", onStorage);
  return () => {
    window.removeEventListener(CHANNEL, onCustom);
    window.removeEventListener("storage", onStorage);
  };
}
