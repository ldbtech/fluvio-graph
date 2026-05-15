/** Shared wordmark glyph for landing and marketing pages. */
export function FluvioMark() {
  return (
    <span
      className="relative inline-flex h-7 w-7 items-center justify-center overflow-hidden rounded-md border border-violet-500/20 bg-violet-500/[0.08]"
      aria-hidden
    >
      <svg viewBox="0 0 24 24" className="relative h-5 w-5 text-violet-200/90" fill="none">
        <path d="M12 3.5 L4.5 8.2 L4.5 15.8 L12 20.5 L19.5 15.8 L19.5 8.2 Z" stroke="currentColor" strokeWidth="1.1" opacity="0.65" />
        <circle cx="12" cy="7.7" r="1.5" className="fill-violet-100" />
        <circle cx="8.3" cy="14.7" r="1.35" className="fill-violet-200/90" />
        <circle cx="15.7" cy="14.7" r="1.35" className="fill-violet-200/80" />
        <path d="M12 9.2 L8.3 13.3 M12 9.2 L15.7 13.3 M8.3 14.7 L15.7 14.7" stroke="currentColor" strokeWidth="1.05" opacity="0.8" />
      </svg>
    </span>
  );
}
