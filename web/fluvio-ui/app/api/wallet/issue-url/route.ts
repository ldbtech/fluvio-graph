import crypto from "crypto";
import { NextResponse } from "next/server";

export const runtime = "nodejs";

/**
 * Signs a short-lived GET URL for `/api/wallet/pass` (Safari cannot send `X-Owner-ID`).
 * Requires `WALLET_PASS_URL_SECRET` in env (min 16 chars recommended).
 */
export async function POST(req: Request) {
  const ownerId = req.headers.get("x-owner-id")?.trim();
  if (!ownerId) return NextResponse.json({ error: "Missing X-Owner-ID" }, { status: 401 });

  const secret = process.env.WALLET_PASS_URL_SECRET;
  if (!secret) return NextResponse.json({ error: "WALLET_PASS_URL_SECRET is not configured" }, { status: 503 });

  const exp = Math.floor(Date.now() / 1000) + 15 * 60;
  const payload = `${ownerId}:${exp}`;
  const sig = crypto.createHmac("sha256", secret).update(payload).digest("hex");

  const origin = new URL(req.url).origin;
  const u = new URL("/api/wallet/pass", origin);
  u.searchParams.set("p", Buffer.from(payload).toString("base64url"));
  u.searchParams.set("s", sig);

  return NextResponse.json({
    pkpassUrl: u.toString(),
    signingConfigured:
      !!(
        process.env.APPLE_PASS_WWDR_PATH &&
        process.env.APPLE_PASS_SIGNER_CERT_PATH &&
        process.env.APPLE_PASS_SIGNER_KEY_PATH &&
        process.env.APPLE_PASS_TYPE_ID &&
        process.env.APPLE_PASS_TEAM_ID
      ),
  });
}
