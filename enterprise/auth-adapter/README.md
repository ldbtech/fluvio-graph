# fluvioMe Enterprise — Auth Adapter

This is the **enterprise-only** authentication adapter for fluvioMe.

## What it does

Sits in front of the Apollo Router (:4001) and:
1. Verifies Firebase JWTs (or any OIDC provider)
2. Creates/syncs users in the fluvioMe database
3. Injects the internal `x-user-id` header before forwarding to the router

## Usage

Only used when `FLUVIOME_ENTERPRISE_TOKEN` is set and valid.  
Enterprise tokens are issued at **https://fluviome.com**.

## Configuration

```env
FLUVIOME_ENTERPRISE_TOKEN=<token from fluviome.com>
FIREBASE_PROJECT_ID=your-firebase-project
PORT=4000
```

## Without Enterprise

In community/headless mode, clients call Apollo Router (:4001) directly  
and supply `x-user-id` themselves. No auth adapter is needed.
