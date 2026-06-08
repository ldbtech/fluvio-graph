const express = require("express");
const http = require("http");
const jwt = require("jsonwebtoken");
const jwksClient = require("jwks-rsa");
require("dotenv").config({ path: "../../.env" });

const app = express();

// Global CORS middleware to support preflights and ensure all error responses (like 401s) have correct access headers
app.use((req, res, next) => {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET,POST,OPTIONS,PUT,DELETE");
  res.setHeader("Access-Control-Allow-Headers", "content-type,x-user-id,authorization,apollo-require-preflight,x-apollo-operation-name");
  if (req.method === "OPTIONS") {
    return res.sendStatus(200);
  }
  next();
});

// Parse JSON body only for application/json to avoid consuming streams for multipart uploads
app.use((req, res, next) => {
  const contentType = req.headers["content-type"] || "";
  if (contentType.includes("application/json")) {
    express.json()(req, res, next);
  } else {
    next();
  }
});

const PORT = process.env.PORT || 4000;
const ROUTER_URL = "http://127.0.0.1:4001";
const DATABASE_URL = process.env.DATABASE_SERVICE_URL || "http://127.0.0.1:3005/graphql";
const FIREBASE_PROJECT_ID = process.env.FIREBASE_PROJECT_ID || "fluvio-web";
const IS_EMULATOR = !!process.env.FIREBASE_AUTH_EMULATOR_HOST;

const client = jwksClient({
  jwksUri: "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com",
  cache: true,
  rateLimit: true,
  jwksRequestsPerMin: 10
});

function getKey(header, callback) {
  if (!header || !header.kid) {
    return callback(new Error("Missing kid in token header"));
  }
  client.getSigningKey(header.kid, function (err, key) {
    if (err) {
      return callback(err);
    }
    const signingKey = key.publicKey || key.rsaPublicKey;
    callback(null, signingKey);
  });
}

// Health check route
app.get("/health", (req, res) => {
  res.json({ status: "healthy" });
});

// Helper for database calls
async function callDatabaseGraphQL(query, variables = {}) {
  const response = await fetch(DATABASE_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-user-id": "system-auth" // system context
    },
    body: JSON.stringify({ query, variables })
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`Database subgraph responded with HTTP ${response.status}: ${text}`);
  }

  const result = await response.json();
  if (result.errors && result.errors.length > 0) {
    throw new Error(`Database GraphQL error: ${result.errors.map(e => e.message).join(", ")}`);
  }
  return result.data;
}

// Catch-all route to proxy requests
app.all("*", async (req, res) => {
  let dbUserId = null;
  let isIntrospection = false;

  // Check if body is introspection query
  if (req.body && req.body.query) {
    const query = req.body.query;
    if (query.includes("__schema") || query.includes("__type")) {
      isIntrospection = true;
    }
  }

  // Verify Firebase token if not an introspection query
  if (!isIntrospection) {
    const authHeader = req.headers["authorization"] || req.headers["Authorization"];
    if (!authHeader) {
      return res.status(401).json({
        errors: [{
          message: "Authentication token is required.",
          extensions: { code: "UNAUTHENTICATED" }
        }]
      });
    }

    const parts = authHeader.split(" ");
    if (parts.length !== 2 || parts[0].toLowerCase() !== "bearer") {
      return res.status(400).json({
        errors: [{
          message: "Invalid Authorization header format. Must be Bearer <token>.",
          extensions: { code: "BAD_REQUEST" }
        }]
      });
    }

    const token = parts[1];

    try {
      let decoded;
      if (IS_EMULATOR) {
        // Decode JWT without signature validation under emulator
        decoded = jwt.decode(token);
        if (!decoded) {
          throw new Error("Invalid token format for emulator");
        }
      } else {
        // Verify signature against JWKS
        decoded = await new Promise((resolve, reject) => {
          jwt.verify(
            token,
            getKey,
            {
              audience: FIREBASE_PROJECT_ID,
              issuer: `https://securetoken.google.com/${FIREBASE_PROJECT_ID}`,
              algorithms: ["RS256"]
            },
            (err, decodedVal) => {
              if (err) reject(err);
              else resolve(decodedVal);
            }
          );
        });
      }

      const uid = decoded.uid || decoded.sub;
      if (!uid) {
        throw new Error("Token is missing a user ID (uid/sub claim)");
      }
      const email = decoded.email;
      const name = decoded.name;

      // Look up user in database
      const GET_USER = `
        query GetUserByFirebaseUid($firebaseUid: String!) {
          getUserByFirebaseUid(firebaseUid: $firebaseUid) {
            id
            email
            displayName
          }
        }
      `;

      const userRes = await callDatabaseGraphQL(GET_USER, { firebaseUid: uid });
      let dbUser = userRes?.getUserByFirebaseUid;

      const emailChanged = dbUser && email && dbUser.email !== email;
      const nameChanged = dbUser && name && dbUser.displayName !== name;

      if (!dbUser || emailChanged || nameChanged) {
        console.log(`[Auth Proxy] Syncing user details for Firebase UID ${uid} (emailChanged=${emailChanged}, nameChanged=${nameChanged})...`);
        const CREATE_USER = `
          mutation CreateUser($input: CreateUserInput!) {
            createUser(input: $input) {
              id
            }
          }
        `;

        const createRes = await callDatabaseGraphQL(CREATE_USER, {
          input: {
            firebaseUid: uid,
            email: email || `${uid}@fluvio-emulator.ai`,
            displayName: name || email || "User",
            avatarUrl: ""
          }
        });
        dbUser = createRes?.createUser;
      }

      dbUserId = dbUser?.id;
    } catch (error) {
      console.error("[Auth Proxy] Verification failed:", error);
      return res.status(401).json({
        errors: [{
          message: `Authentication failed: ${error.message}`,
          extensions: { code: "UNAUTHENTICATED" }
        }]
      });
    }
  }

  // Construct target request headers
  const targetHeaders = { ...req.headers };
  if (dbUserId) {
    targetHeaders["x-user-id"] = dbUserId;
  }
  // Remove Authorization to avoid downstream propagation
  delete targetHeaders["authorization"];
  delete targetHeaders["Authorization"];

  let bodyStr = "";
  if (req.body && Object.keys(req.body).length > 0) {
    bodyStr = JSON.stringify(req.body);
    targetHeaders["content-length"] = Buffer.byteLength(bodyStr);
  }

  // Forward the request to Apollo Router (port 4001)
  const targetUrl = new URL(ROUTER_URL + req.url);
  const proxyReq = http.request({
    host: "127.0.0.1",
    port: 4001,
    path: targetUrl.pathname + targetUrl.search,
    method: req.method,
    headers: targetHeaders
  }, (proxyRes) => {
    res.writeHead(proxyRes.statusCode, proxyRes.headers);
    proxyRes.pipe(res);
  });

  proxyReq.on("error", (err) => {
    console.error("[Auth Proxy] Router connection error:", err);
    res.status(502).json({
      errors: [{
        message: "Gateway connection error",
        extensions: { code: "BAD_GATEWAY" }
      }]
    });
  });

  // If request has JSON body parsed, write it out
  if (bodyStr) {
    proxyReq.write(bodyStr);
    proxyReq.end();
  } else {
    // Pipe raw stream (e.g. multipart/form-data) directly to Apollo Router
    req.pipe(proxyReq);
  }
});

app.listen(PORT, "0.0.0.0", () => {
  console.log(`[Auth Proxy] Gateway running on http://localhost:${PORT} forwarding to Router at port 4001 (Emulator: ${IS_EMULATOR}, Project: ${FIREBASE_PROJECT_ID})`);
});
