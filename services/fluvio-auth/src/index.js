/**
 * fluvioMe Enterprise Token Coprocessor
 *
 * This service is the ENTERPRISE gate for fluvioMe.
 * It is NOT part of the community/OSS runtime — it only starts when
 * FLUVIOME_ENTERPRISE_TOKEN is present in the environment.
 *
 * Role:
 *   - Validates the enterprise JWT issued by fluviome.com
 *   - Injects x-fluviome-tier (starter | growth | enterprise) into every request
 *   - Guards enterprise-only features: collaboration, SSO, audit logs, multi-tenant
 *   - Runs as an Apollo Router coprocessor on port 4002
 *
 * License: BUSL-1.1  (requires commercial license for any for-profit use)
 * Change Date: 4 years from first tagged release → Apache-2.0
 *
 * Community mode (non-commercial — students, researchers, non-profits):
 *   Do NOT start this service. Free and unlimited, forever. No trial period,
 *   no 24-hour limits, no expiry. Apollo Router at :4001 is the direct entry
 *   point. Pass x-user-id from your own auth layer (or omit for anonymous use).
 *
 * Enterprise mode:
 *   1. Register at https://fluviome.com → get your FLUVIOME_ENTERPRISE_TOKEN
 *   2. Add FLUVIOME_ENTERPRISE_TOKEN=<token> to your .env
 *   3. Start this service (it auto-starts via dev.sh when the env var is set)
 *   4. Uncomment the coprocessor block in services/fluvio-gateway/router.yaml
 */

"use strict";

const express = require("express");
const jwt = require("jsonwebtoken");
require("dotenv").config({ path: "../../.env" });

const app = express();
app.use(express.json());

const PORT = process.env.FLUVIOME_ENTERPRISE_COPROCESSOR_PORT || 4002;
const ENTERPRISE_TOKEN = process.env.FLUVIOME_ENTERPRISE_TOKEN;
const FLUVIOME_PUBLIC_KEY = process.env.FLUVIOME_PUBLIC_KEY || "";

// Fail fast — this service should never start without an enterprise token
if (!ENTERPRISE_TOKEN) {
  console.error(
    "[fluvioMe Enterprise] FLUVIOME_ENTERPRISE_TOKEN is not set.\n" +
    "This service is only needed for enterprise deployments.\n" +
    "Community users: run the engine without this service.\n" +
    "Register at https://fluviome.com to get your enterprise token."
  );
  process.exit(1);
}

app.use((req, res, next) => {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET,POST,OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "content-type");
  if (req.method === "OPTIONS") return res.sendStatus(200);
  next();
});

app.get("/health", (_req, res) => {
  res.json({ status: "ok", mode: "enterprise" });
});

/**
 * Apollo Router coprocessor endpoint.
 * Called by the router on every incoming request (router.request stage).
 *
 * Validates x-fluviome-token header and injects:
 *   x-fluviome-tier: starter | growth | enterprise
 *   x-fluviome-org:  organization slug from the token
 */
app.post("/", (req, res) => {
  const body = req.body;
  const stage = body?.stage;

  if (stage !== "RouterRequest") {
    return res.json(body); // pass through non-request stages unchanged
  }

  const headers = body?.control?.break?.headers || body?.headers || {};
  const token = headers["x-fluviome-token"] || headers["X-Fluviome-Token"];

  if (!token) {
    // No enterprise token → reject enterprise-only operations, pass through community ones
    return res.json({
      ...body,
      headers: {
        ...headers,
        "x-fluviome-tier": "community",
      },
    });
  }

  try {
    let decoded;
    if (FLUVIOME_PUBLIC_KEY) {
      decoded = jwt.verify(token, FLUVIOME_PUBLIC_KEY, { algorithms: ["RS256"] });
    } else {
      // Dev: decode without signature verification (enterprise token not yet fully set up)
      decoded = jwt.decode(token);
      if (!decoded) throw new Error("Invalid token format");
    }

    const tier = decoded.tier || "starter";
    const org = decoded.org || decoded.sub || "unknown";

    return res.json({
      ...body,
      headers: {
        ...headers,
        "x-fluviome-tier": tier,
        "x-fluviome-org": org,
      },
    });
  } catch (err) {
    console.error("[fluvioMe Enterprise] Token verification failed:", err.message);
    return res.status(401).json({
      errors: [{ message: "Invalid enterprise token", extensions: { code: "ENTERPRISE_TOKEN_INVALID" } }],
    });
  }
});

app.listen(PORT, "0.0.0.0", () => {
  console.log(`[fluvioMe Enterprise] Coprocessor running on :${PORT}`);
  console.log(`[fluvioMe Enterprise] Token validation: ${FLUVIOME_PUBLIC_KEY ? "RS256 signature" : "decode-only (dev)"}`);
});
