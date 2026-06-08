"use strict";
/**
 * Token issuance and verification.
 *
 * Enterprise tokens are RS256-signed JWTs that self-describe the org's tier.
 * The engine's coprocessor verifies them offline using the public key —
 * no call home required, no uptime dependency on this service.
 *
 * Community / non-commercial use:
 *   No token is needed. The engine runs forever with full pipeline/KG/BI features.
 *   Zero time limits. Zero feature expiry. Zero nag screens.
 *   "Non-commercial" = not used to generate revenue for a for-profit entity.
 *   Students, researchers, open-source maintainers, non-profits → always free.
 *
 * Payload shape:
 *   {
 *     sub:          "org_<uuid>",
 *     org:          "acme-corp",          // slug for display
 *     org_name:     "Acme Corp",          // display name
 *     tier:         "starter" | "growth" | "enterprise",
 *     features:     ["collaboration", "sso", "audit_logs", "white_label"],
 *     stripe_sub:   "sub_xxx",            // Stripe subscription ID
 *     iat, exp, iss
 *   }
 */

const jwt = require("jsonwebtoken");

const ISSUER   = "https://fluviome.com";
const AUDIENCE = "fluviome-engine";

// Tier → included features
const TIER_FEATURES = {
  starter:    ["collaboration"],
  growth:     ["collaboration", "audit_logs", "sso"],
  enterprise: ["collaboration", "audit_logs", "sso", "white_label", "priority_support"],
};

// Enterprise token TTL — long-lived so an expired Stripe card doesn't
// immediately break a production deployment. Billing enforcement is handled
// by Stripe webhooks re-issuing or revoking tokens, not by JWT expiry.
// The community (non-commercial) tier has NO token and NO expiry — it is
// free and lifetime. Token expiry only applies to paid enterprise tiers.
const TIER_TTL = {
  starter:    "366d",   // re-issued on each monthly renewal via Stripe webhook
  growth:     "366d",
  enterprise: "366d",   // annual — re-issued on renewal
};

/**
 * Issue a signed enterprise token for an organisation.
 */
function issueToken({ orgId, orgSlug, orgName, tier, stripeSubId, privateKey }) {
  const features = TIER_FEATURES[tier] || TIER_FEATURES.starter;
  const expiresIn = TIER_TTL[tier] || "31d";

  return jwt.sign(
    {
      sub:        `org_${orgId}`,
      org:        orgSlug,
      org_name:   orgName,
      tier,
      features,
      stripe_sub: stripeSubId,
    },
    privateKey,
    {
      algorithm: "RS256",
      expiresIn,
      issuer:    ISSUER,
      audience:  AUDIENCE,
    }
  );
}

/**
 * Verify and decode a token using the public key.
 * Used by the token-service itself to inspect tokens on /token/verify.
 */
function verifyToken(token, publicKey) {
  return jwt.verify(token, publicKey, {
    algorithms: ["RS256"],
    issuer:     ISSUER,
    audience:   AUDIENCE,
  });
}

module.exports = { issueToken, verifyToken, TIER_FEATURES };
