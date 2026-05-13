import { readFileSync } from "node:fs";
import crypto from "node:crypto";
import { join } from "node:path";
import { PKPass } from "passkit-generator";
import { getKgEngineUrl } from "@/shared/lib/constants";

export const runtime = "nodejs";

type TwinMeAccount = {
  display_name?: string;
  tagline?: string;
  owner_slug?: string;
  nfc_card_id?: string | null;
};

function loadSignerCertificates() {
  const wwdr = process.env.APPLE_PASS_WWDR_PATH;
  const cert = process.env.APPLE_PASS_SIGNER_CERT_PATH;
  const key = process.env.APPLE_PASS_SIGNER_KEY_PATH;
  if (!wwdr || !cert || !key) return null;
  return {
    wwdr: readFileSync(wwdr),
    signerCert: readFileSync(cert),
    signerKey: readFileSync(key),
    signerKeyPassphrase: process.env.APPLE_PASS_KEY_PASSPHRASE ?? "",
  };
}

function verifySignedPayload(encoded: string, sig: string, secret: string): string | null {
  try {
    const payload = Buffer.from(encoded, "base64url").toString("utf8");
    const expectedHex = crypto.createHmac("sha256", secret).update(payload).digest("hex");
    const a = Buffer.from(sig, "hex");
    const b = Buffer.from(expectedHex, "hex");
    if (a.length !== b.length || a.length === 0) return null;
    if (!crypto.timingSafeEqual(a, b)) return null;

    const [ownerId, expStr] = payload.split(":");
    const exp = Number(expStr);
    if (!ownerId || Number.isFinite(exp) === false || Date.now() / 1000 > exp) return null;
    return ownerId;
  } catch {
    return null;
  }
}

function kgBaseUrl(): string {
  return getKgEngineUrl();
}

function appPublicOrigin(req: Request): string {
  const fromEnv =
    process.env.NEXT_PUBLIC_APP_URL ??
    (process.env.VERCEL_URL ? `https://${process.env.VERCEL_URL}` : null);
  if (fromEnv) return fromEnv.replace(/\/$/, "");
  try {
    return new URL((req as Request).url ?? "", "http://localhost:3000").origin;
  } catch {
    return "http://localhost:3000";
  }
}

export async function GET(req: Request): Promise<Response> {
  const secret = process.env.WALLET_PASS_URL_SECRET;
  if (!secret) return new Response("Wallet URL signing not configured", { status: 503 });

  const { searchParams } = new URL(req.url);
  const p = searchParams.get("p");
  const s = searchParams.get("s");
  if (!p || !s) return new Response("Bad request", { status: 400 });

  const ownerId = verifySignedPayload(p, s, secret);
  if (!ownerId) return new Response("Invalid or expired link", { status: 403 });

  const certificates = loadSignerCertificates();
  const passTypeId = process.env.APPLE_PASS_TYPE_ID;
  const teamId = process.env.APPLE_PASS_TEAM_ID;
  if (!certificates || !passTypeId || !teamId) {
    return new Response("Apple Wallet signing not configured on server", { status: 503 });
  }

  const modelPath = join(process.cwd(), "wallet", "fluvio-card.pass");
  const kg = kgBaseUrl();
  const twinRes = await fetch(`${kg}/twin/me`, {
    headers: { "X-Owner-ID": ownerId },
    cache: "no-store",
  });
  if (!twinRes.ok) return new Response("Account not found", { status: 404 });

  const account = (await twinRes.json()) as TwinMeAccount;
  const origin = appPublicOrigin(req);
  const cardId =
    typeof account.nfc_card_id === "string" && account.nfc_card_id.length > 0 ? account.nfc_card_id.trim() : null;
  const tapUrl = cardId ? `${origin}/tap?card=${encodeURIComponent(cardId)}` : `${origin}/tap`;

  const blankLabel = "\u2060"; // invisible, avoids loud NAME/LABEL headers in Wallet UI
  const displayName = (account.display_name ?? "").trim() || "You";
  const taglineRaw = (account.tagline ?? "").trim();

  /** Match onboarding card rhythm: headline + subtitle; no trailing em dash placeholders. */
  const secondaryRows: Array<{ key: string; label: string; value: string }> = [];
  if (taglineRaw) {
    secondaryRows.push({ key: "tagline", label: blankLabel, value: taglineRaw });
  }
  const slug = (account.owner_slug ?? "").trim();
  if (slug) {
    secondaryRows.push({ key: "handle", label: blankLabel, value: `@${slug}` });
  }

  const generic = {
    primaryFields: [
      {
        key: "name",
        label: blankLabel,
        value: displayName,
      },
    ],
    secondaryFields: secondaryRows.slice(0, 2),
    auxiliaryFields: [] as Array<{ key: string; label: string; value: string }>,
  };

  const pass = await PKPass.from(
    {
      model: modelPath,
      certificates,
    },
    // `generic` overrides are valid here; OverridablePassProps omits them from its type alias.
    {
      serialNumber: ownerId,
      passTypeIdentifier: passTypeId,
      teamIdentifier: teamId,
      description: `FluvioMe · ${displayName}`,
      organizationName: "FluvioMe",
      logoText: "",
      foregroundColor: "rgb(237,237,239)",
      backgroundColor: "rgb(10,10,15)",
      labelColor: "rgb(148,146,169)",
      suppressStripShine: true,
      generic,
    } as never,
  );

  pass.setBarcodes({
    format: "PKBarcodeFormatQR",
    message: tapUrl,
    messageEncoding: "iso-8859-1",
  });

  const buffer = pass.getAsBuffer();
  const fileSlug = slug || "card";
  return new Response(Buffer.from(buffer), {
    headers: {
      "Content-Type": "application/vnd.apple.pkpass",
      "Content-Disposition": `attachment; filename="fluviome-${fileSlug}.pkpass"`,
      "Cache-Control": "no-store",
    },
  });
}
