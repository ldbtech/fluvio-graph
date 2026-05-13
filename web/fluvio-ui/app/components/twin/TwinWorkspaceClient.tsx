"use client";

/**
 * Compatibility re-export — some Webpack caches / tooling still resolved the old
 * `app/components/twin/...` path after the UI moved to `features/twin/components`.
 */
export { TwinWorkspaceClient } from "@/features/twin/components/TwinWorkspaceClient";
