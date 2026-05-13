const BOOT_KEY = "fluvio_twin_chat_boot";

/** Cleared when leaving NFC landing so each tap can open a fresh chat turn. */
export function resetTwinChatBootstrap() {
  if (typeof window === "undefined") return;
  sessionStorage.removeItem(BOOT_KEY);
}

/**
 * Returns true once per browser session segment until `resetTwinChatBootstrap()` runs.
 * Used only for optional `?topic=…` auto-turn on the twin workspace (not for default empty chat).
 */
export function tryBeginTwinChatBootstrap(): boolean {
  if (typeof window === "undefined") return true;
  if (sessionStorage.getItem(BOOT_KEY)) return false;
  sessionStorage.setItem(BOOT_KEY, "1");
  return true;
}
