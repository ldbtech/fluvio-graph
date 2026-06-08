#!/usr/bin/env node
/**
 * One-time key generation utility.
 * Run: node src/keygen.js
 * Paste the output into your .env file.
 */
const { generateKeyPairSync } = require("crypto");

const { privateKey, publicKey } = generateKeyPairSync("rsa", {
  modulusLength: 2048,
  publicKeyEncoding:  { type: "spki",  format: "pem" },
  privateKeyEncoding: { type: "pkcs8", format: "pem" },
});

const toEnvLine = (label, pem) =>
  `${label}="${pem.replace(/\n/g, "\\n")}"`;

console.log("\n# Paste these into your .env (token-service and fluviome-engine):\n");
console.log(toEnvLine("FLUVIOME_PRIVATE_KEY", privateKey));
console.log(toEnvLine("FLUVIOME_PUBLIC_KEY",  publicKey));
console.log("\n# The PUBLIC key also goes into the engine's .env so the");
console.log("# enterprise coprocessor can verify tokens without calling home.\n");
