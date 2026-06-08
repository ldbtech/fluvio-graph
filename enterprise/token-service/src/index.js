"use strict";
/**
 * fluvioMe Token Service
 *
 * Handles:
 *   POST /stripe/webhook          ← Stripe sends subscription events here
 *   GET  /token/:orgId            ← Org fetches their current token
 *   POST /token/verify            ← Verify any token (debug / support)
 *   POST /contact                 ← Contact / sales enquiry form
 *   GET  /health
 *
 * Flow:
 *   1. Org signs up on fluviome.com → Stripe Checkout session created
 *   2. Stripe fires customer.subscription.created → we issue a JWT → email it
 *   3. Org sets FLUVIOME_ENTERPRISE_TOKEN=<jwt> in their .env → enterprise features unlock
 *   4. Stripe fires customer.subscription.updated → re-issue token with new tier
 *   5. Stripe fires customer.subscription.deleted → token expires naturally (not revoked)
 *
 * License: BUSL-1.1
 */

const express = require("express");
const { v4: uuidv4 } = require("uuid");
require("dotenv").config({ path: "../../../.env" });

const { issueToken, verifyToken, TIER_FEATURES } = require("./token");
const { createOrg, updateOrg, getByStripeCustomer, getByEmail, getOrg } = require("./store");
const { sendWelcomeEmail, sendContactConfirmation, sendContactToTeam } = require("./mail");

const app = express();
const PORT = process.env.TOKEN_SERVICE_PORT || 4003;

// ── Keys (generate once with: node src/keygen.js) ────────────────────────────
const PRIVATE_KEY = (process.env.FLUVIOME_PRIVATE_KEY || "").replace(/\\n/g, "\n");
const PUBLIC_KEY  = (process.env.FLUVIOME_PUBLIC_KEY  || "").replace(/\\n/g, "\n");

if (!PRIVATE_KEY || !PUBLIC_KEY) {
  console.error(
    "[TokenService] FLUVIOME_PRIVATE_KEY and FLUVIOME_PUBLIC_KEY are required.\n" +
    "Generate them with: node src/keygen.js"
  );
  process.exit(1);
}

const STRIPE_SECRET  = process.env.STRIPE_SECRET_KEY  || "";
const STRIPE_WEBHOOK = process.env.STRIPE_WEBHOOK_SECRET || "";

const stripe = STRIPE_SECRET ? require("stripe")(STRIPE_SECRET) : null;

// Stripe webhook needs the raw body — must come BEFORE express.json()
app.use("/stripe/webhook", express.raw({ type: "application/json" }));
app.use(express.json());

app.use((req, res, next) => {
  res.setHeader("Access-Control-Allow-Origin", process.env.CORS_ORIGIN || "*");
  res.setHeader("Access-Control-Allow-Methods", "GET,POST,OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "content-type,authorization");
  if (req.method === "OPTIONS") return res.sendStatus(200);
  next();
});

// ── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Map a Stripe price ID or product metadata to a fluvioMe tier.
 * Configure STRIPE_PRICE_STARTER / _GROWTH / _ENTERPRISE in .env.
 */
function tierFromStripe(subscription) {
  const priceId = subscription.items?.data?.[0]?.price?.id || "";
  if (priceId === process.env.STRIPE_PRICE_ENTERPRISE) return "enterprise";
  if (priceId === process.env.STRIPE_PRICE_GROWTH)     return "growth";
  return "starter";
}

function slugify(name) {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

// ── Routes ───────────────────────────────────────────────────────────────────

app.get("/health", (_req, res) => {
  res.json({ status: "ok", service: "fluviome-token-service" });
});

/**
 * POST /stripe/webhook
 * Stripe sends all subscription lifecycle events here.
 * Verify the signature, then handle the event.
 */
app.post("/stripe/webhook", async (req, res) => {
  if (!stripe) {
    return res.status(503).json({ error: "Stripe not configured" });
  }

  let event;
  try {
    event = stripe.webhooks.constructEvent(req.body, req.headers["stripe-signature"], STRIPE_WEBHOOK);
  } catch (err) {
    console.error("[TokenService] Webhook signature failed:", err.message);
    return res.status(400).json({ error: "Webhook signature invalid" });
  }

  const subscription = event.data?.object;

  try {
    switch (event.type) {

      // ── New subscription — issue first token ───────────────────────────────
      case "customer.subscription.created": {
        const customer  = await stripe.customers.retrieve(subscription.customer);
        const email     = customer.email;
        const orgName   = customer.name || customer.metadata?.org_name || email.split("@")[0];
        const orgSlug   = slugify(orgName);
        const tier      = tierFromStripe(subscription);
        const orgId     = uuidv4();

        const token = issueToken({
          orgId,
          orgSlug,
          orgName,
          tier,
          stripeSubId:    subscription.id,
          privateKey:     PRIVATE_KEY,
        });

        createOrg({
          orgId, orgSlug, orgName, email, tier,
          stripeCustomerId: subscription.customer,
          stripeSubId:      subscription.id,
          token,
        });

        console.log(`[TokenService] Issued ${tier} token for ${orgName} (${email})`);

        await sendWelcomeEmail({ to: email, orgName, tier, token }).catch((e) =>
          console.error("[TokenService] Welcome email failed:", e.message)
        );
        break;
      }

      // ── Tier change or renewal — re-issue token ────────────────────────────
      case "customer.subscription.updated": {
        const org = getByStripeCustomer(subscription.customer);
        if (!org) break;

        const tier  = tierFromStripe(subscription);
        const token = issueToken({
          orgId:       org.orgId,
          orgSlug:     org.orgSlug,
          orgName:     org.orgName,
          tier,
          stripeSubId: subscription.id,
          privateKey:  PRIVATE_KEY,
        });

        updateOrg(org.orgId, { tier, stripeSubId: subscription.id, token, tokenIssuedAt: new Date().toISOString() });

        console.log(`[TokenService] Re-issued ${tier} token for ${org.orgName}`);

        await sendWelcomeEmail({ to: org.email, orgName: org.orgName, tier, token }).catch((e) =>
          console.error("[TokenService] Re-issue email failed:", e.message)
        );
        break;
      }

      // ── Cancellation — mark as cancelled (token still valid until exp) ─────
      case "customer.subscription.deleted": {
        const org = getByStripeCustomer(subscription.customer);
        if (!org) break;
        updateOrg(org.orgId, { status: "cancelled" });
        console.log(`[TokenService] Subscription cancelled for ${org.orgName}`);
        break;
      }

      // ── Payment failure — mark as past_due ────────────────────────────────
      case "invoice.payment_failed": {
        const invoice = event.data.object;
        const org = getByStripeCustomer(invoice.customer);
        if (!org) break;
        updateOrg(org.orgId, { status: "past_due" });
        console.warn(`[TokenService] Payment failed for ${org.orgName}`);
        break;
      }

      default:
        break;
    }
  } catch (err) {
    console.error(`[TokenService] Error handling ${event.type}:`, err);
    return res.status(500).json({ error: "Internal error" });
  }

  res.json({ received: true });
});

/**
 * GET /token/:orgId
 * Returns the current live token for an org (authenticated by secret header).
 * Called by the fluviome.com dashboard's "copy token" button.
 */
app.get("/token/:orgId", (req, res) => {
  const secret = req.headers["x-service-secret"];
  if (!secret || secret !== process.env.SERVICE_SECRET) {
    return res.status(401).json({ error: "Unauthorized" });
  }

  const org = getOrg(req.params.orgId);
  if (!org) return res.status(404).json({ error: "Org not found" });

  res.json({
    orgId:         org.orgId,
    orgName:       org.orgName,
    tier:          org.tier,
    status:        org.status,
    token:         org.token,
    tokenIssuedAt: org.tokenIssuedAt,
    features:      TIER_FEATURES[org.tier] || [],
  });
});

/**
 * POST /token/verify
 * Verify any token — used by support/debug tooling.
 */
app.post("/token/verify", (req, res) => {
  const { token } = req.body || {};
  if (!token) return res.status(400).json({ error: "token is required" });

  try {
    const decoded = verifyToken(token, PUBLIC_KEY);
    res.json({ valid: true, payload: decoded });
  } catch (err) {
    res.status(400).json({ valid: false, error: err.message });
  }
});

/**
 * POST /contact
 * Contact / sales enquiry form.
 * Called by the fluviome.com contact form and the in-engine chat widget.
 *
 * Body: { name, email, company?, subject?, message, tier? }
 */
app.post("/contact", async (req, res) => {
  const { name, email, company, subject, message, tier } = req.body || {};

  if (!name || !email || !message) {
    return res.status(400).json({ error: "name, email, and message are required" });
  }

  const emailRe = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!emailRe.test(email)) {
    return res.status(400).json({ error: "Invalid email address" });
  }

  try {
    await Promise.all([
      sendContactToTeam({ name, email, company, subject, message, tier }),
      sendContactConfirmation({ to: email, name, message }),
    ]);

    console.log(`[TokenService] Contact form: ${name} <${email}> — ${subject || "no subject"}`);
    res.json({ ok: true, message: "Message received — we'll reply within 1 business day." });
  } catch (err) {
    console.error("[TokenService] Contact email failed:", err.message);
    // Still return 200 — don't expose mail config errors to the user
    res.json({ ok: true, message: "Message received — we'll reply within 1 business day." });
  }
});

// ── Start ─────────────────────────────────────────────────────────────────────
app.listen(PORT, "0.0.0.0", () => {
  console.log(`[fluvioMe Token Service] :${PORT}`);
  console.log(`  Stripe:  ${stripe ? "configured" : "NOT configured — webhook will reject"}`);
  console.log(`  SMTP:    ${process.env.SMTP_HOST || "not configured — emails will fail"}`);
  console.log(`  Keys:    RS256 (${PRIVATE_KEY.length > 0 ? "loaded" : "MISSING"})`);
});
