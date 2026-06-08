"use strict";
/**
 * Email delivery — welcome email with token, contact form confirmations.
 * Uses nodemailer. In production point SMTP_* vars at SendGrid / Postmark / SES.
 */

const nodemailer = require("nodemailer");

const transporter = nodemailer.createTransport({
  host:   process.env.SMTP_HOST   || "smtp.sendgrid.net",
  port:   Number(process.env.SMTP_PORT)  || 587,
  secure: false,
  auth: {
    user: process.env.SMTP_USER || "apikey",
    pass: process.env.SMTP_PASS || "",
  },
});

const FROM = process.env.MAIL_FROM || "fluvioMe <hello@fluviome.com>";

async function sendWelcomeEmail({ to, orgName, tier, token }) {
  const subject = `Your fluvioMe Enterprise token is ready — ${tier} tier`;
  const text = `
Hi ${orgName},

Welcome to fluvioMe Enterprise (${tier} tier)!

Your enterprise token:
──────────────────────────────────────────────────
${token}
──────────────────────────────────────────────────

Add it to your .env file:
  FLUVIOME_ENTERPRISE_TOKEN=${token}

Then restart your stack:
  docker compose up          # community
  docker compose --profile enterprise up   # with enterprise gate

What's unlocked on your tier:
${tier === "starter"    ? "  • Real-time collaboration" : ""}
${tier === "growth"     ? "  • Collaboration\n  • Audit logs\n  • SSO" : ""}
${tier === "enterprise" ? "  • Collaboration\n  • Audit logs\n  • SSO\n  • White-label\n  • Priority support" : ""}

Docs: https://docs.fluviome.com/enterprise
Support: https://fluviome.com/support

— The fluvioMe team
`.trim();

  await transporter.sendMail({ from: FROM, to, subject, text });
}

async function sendContactConfirmation({ to, name, message }) {
  const subject = "We received your message — fluvioMe";
  const text = `
Hi ${name},

Thanks for reaching out! We got your message and will reply within 1 business day.

Your message:
${message}

— The fluvioMe team
https://fluviome.com
`.trim();

  await transporter.sendMail({ from: FROM, to, subject, text });
}

async function sendContactToTeam({ name, email, company, subject: sub, message, tier }) {
  const subject = `[Contact] ${sub || "New enquiry"} — ${company || email}`;
  const text = `
Name:    ${name}
Email:   ${email}
Company: ${company || "—"}
Tier interest: ${tier || "—"}

Message:
${message}
`.trim();

  const teamEmail = process.env.TEAM_EMAIL || "team@fluviome.com";
  await transporter.sendMail({ from: FROM, to: teamEmail, replyTo: email, subject, text });
}

module.exports = { sendWelcomeEmail, sendContactConfirmation, sendContactToTeam };
