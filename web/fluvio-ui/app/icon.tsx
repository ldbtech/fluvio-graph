import { ImageResponse } from "next/og";

export const size = {
  width: 64,
  height: 64,
};

export const contentType = "image/png";

export default function Icon() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background:
            "radial-gradient(circle at 30% 20%, rgba(125,211,252,0.26), transparent 55%), #070a12",
        }}
      >
        <svg width="50" height="50" viewBox="0 0 24 24" fill="none">
          <path
            d="M12 3.5 L4.5 8.2 L4.5 15.8 L12 20.5 L19.5 15.8 L19.5 8.2 Z"
            stroke="rgba(125,211,252,0.85)"
            strokeWidth="1.2"
            opacity="0.9"
          />
          <circle cx="12" cy="7.7" r="1.5" fill="rgba(186,230,253,1)" />
          <circle cx="8.3" cy="14.7" r="1.35" fill="rgba(125,211,252,0.95)" />
          <circle cx="15.7" cy="14.7" r="1.35" fill="rgba(147,197,253,0.95)" />
          <path
            d="M12 9.2 L8.3 13.3 M12 9.2 L15.7 13.3 M8.3 14.7 L15.7 14.7"
            stroke="rgba(186,230,253,0.92)"
            strokeWidth="1.2"
          />
        </svg>
      </div>
    ),
    size,
  );
}
