"use strict";
/**
 * In-memory org store — swap for Postgres/Redis in production.
 *
 * Schema:
 *   orgs[orgId] = {
 *     orgId, orgSlug, orgName, email,
 *     tier, stripeCustomerId, stripeSubId,
 *     token,          // current live JWT
 *     tokenIssuedAt,
 *     status,         // "active" | "cancelled" | "past_due"
 *     createdAt,
 *   }
 */

const orgs = new Map();                    // orgId → org record
const byStripeCustomer = new Map();        // stripeCustomerId → orgId
const byEmail = new Map();                 // email → orgId

function createOrg({ orgId, orgSlug, orgName, email, tier, stripeCustomerId, stripeSubId, token }) {
  const record = {
    orgId, orgSlug, orgName, email, tier,
    stripeCustomerId, stripeSubId,
    token,
    tokenIssuedAt: new Date().toISOString(),
    status: "active",
    createdAt: new Date().toISOString(),
  };
  orgs.set(orgId, record);
  byStripeCustomer.set(stripeCustomerId, orgId);
  byEmail.set(email.toLowerCase(), orgId);
  return record;
}

function updateOrg(orgId, patch) {
  const existing = orgs.get(orgId);
  if (!existing) throw new Error(`Org ${orgId} not found`);
  const updated = { ...existing, ...patch, updatedAt: new Date().toISOString() };
  orgs.set(orgId, updated);
  return updated;
}

function getByStripeCustomer(stripeCustomerId) {
  const orgId = byStripeCustomer.get(stripeCustomerId);
  return orgId ? orgs.get(orgId) : null;
}

function getByEmail(email) {
  const orgId = byEmail.get(email.toLowerCase());
  return orgId ? orgs.get(orgId) : null;
}

function getOrg(orgId) {
  return orgs.get(orgId) || null;
}

module.exports = { createOrg, updateOrg, getByStripeCustomer, getByEmail, getOrg };
