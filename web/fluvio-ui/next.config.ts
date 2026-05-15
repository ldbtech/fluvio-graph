import path from "path";
import type { NextConfig } from "next";

const pkgRoot = __dirname;
const turbopackRoot = path.resolve(pkgRoot, "../..");

/** Turbopack expects alias targets relative to `turbopack.root` (not absolute filesystem paths). */
function turboRel(abs: string): string {
  const rel = path.relative(turbopackRoot, abs);
  if (!rel || rel.startsWith("..")) {
    throw new Error(`Path ${abs} is not under turbopack root ${turbopackRoot}`);
  }
  return rel.split(path.sep).join("/");
}

const nextConfig: NextConfig = {
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "images.unsplash.com",
        pathname: "/**",
      },
    ],
  },
  /** Lets dev HMR load from your machine’s LAN IP (e.g. phone on same Wi‑Fi). Add more hosts if your IP changes. */
  allowedDevOrigins: ["192.168.1.83"],
  turbopack: {
    root: turbopackRoot,
    resolveAlias: {
      "@fluvio-tools": turboRel(path.resolve(pkgRoot, "../../fluvio-tools")),
      three: turboRel(path.resolve(pkgRoot, "node_modules/three")),
    },
  },
  webpack: (config) => {
    config.resolve = config.resolve ?? {};
    const prev = config.resolve.alias;
    const base =
      prev && typeof prev === "object" && !Array.isArray(prev)
        ? (prev as Record<string, string | false | string[]>)
        : {};
    config.resolve.alias = {
      ...base,
      "@fluvio-tools": path.resolve(pkgRoot, "../../fluvio-tools"),
      // One three.js instance for fluvio-ui + fluvio-tools (avoids duplicate @types/three).
      three: path.resolve(pkgRoot, "node_modules/three"),
    };
    return config;
  },
};

export default nextConfig;
