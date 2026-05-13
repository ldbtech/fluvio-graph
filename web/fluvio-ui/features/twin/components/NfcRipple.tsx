"use client";

import { motion } from "framer-motion";

const rings = [0, 1, 2, 3];

export function NfcRipple() {
  return (
    <div className="pointer-events-none absolute inset-0 flex items-center justify-center" aria-hidden>
      {rings.map((i) => (
        <motion.span
          key={i}
          className="absolute rounded-full border border-[#534AB7]/35"
          style={{ width: 72, height: 72 }}
          initial={{ scale: 0.4, opacity: 0.55 }}
          animate={{ scale: 3.2 + i * 0.35, opacity: 0 }}
          transition={{
            duration: 2.4,
            repeat: Infinity,
            delay: i * 0.55,
            ease: [0.22, 1, 0.36, 1],
          }}
        />
      ))}
    </div>
  );
}
