"use client";

type Props = {
  className?: string;
  size?: number;
};

/** Fluvio logo mark: three connected nodes (NFC twin aesthetic). */
export function FluvioTwinMark({ className = "", size = 56 }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 48 48"
      className={className}
      fill="none"
      aria-hidden
    >
      <path
        d="M24 6 L9 15.5 L9 32.5 L24 42 L39 32.5 L39 15.5 Z"
        stroke="#534AB7"
        strokeWidth="1.2"
        opacity={0.45}
      />
      <path d="M24 14 L15 19.2 M24 14 L33 19.2 M15 28.8 L24 34 M33 28.8 L24 34 M15 19.2 L15 28.8 M33 19.2 L33 28.8" stroke="#7F77DD" strokeWidth="1.05" opacity={0.85} />
      <circle cx="24" cy="14" r="3.2" fill="#534AB7" />
      <circle cx="15" cy="24" r="2.8" fill="#7F77DD" />
      <circle cx="33" cy="24" r="2.8" fill="#AFA9EC" />
    </svg>
  );
}
