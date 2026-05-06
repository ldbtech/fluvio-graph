"use client";

import { motion } from "framer-motion";

const nodes = [
  { cx: 18, cy: 22, r: 1.8, delay: 0 },
  { cx: 82, cy: 28, r: 1.5, delay: 0.4 },
  { cx: 12, cy: 72, r: 1.3, delay: 0.8 },
  { cx: 88, cy: 68, r: 1.6, delay: 0.2 },
  { cx: 50, cy: 12, r: 1.2, delay: 0.6 },
  { cx: 42, cy: 88, r: 1.4, delay: 1 },
];

const edges = [
  [0, 4],
  [1, 4],
  [2, 3],
  [4, 5],
  [0, 2],
];

export function GraphBackground() {
  return (
    <div className="pointer-events-none fixed inset-0 z-0 overflow-hidden opacity-[0.22]" aria-hidden>
      <svg className="h-full w-full" viewBox="0 0 100 100" preserveAspectRatio="xMidYMid slice">
        <defs>
          <linearGradient id="twin-edge" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#534AB7" stopOpacity="0.2" />
            <stop offset="100%" stopColor="#7F77DD" stopOpacity="0.45" />
          </linearGradient>
        </defs>
        {edges.map(([a, b], i) => {
          const A = nodes[a];
          const B = nodes[b];
          if (!A || !B) return null;
          return (
            <line
              key={i}
              x1={A.cx}
              y1={A.cy}
              x2={B.cx}
              y2={B.cy}
              stroke="url(#twin-edge)"
              strokeWidth="0.25"
              vectorEffect="non-scaling-stroke"
            />
          );
        })}
        {nodes.map((n, i) => (
          <motion.circle
            key={i}
            cx={n.cx}
            cy={n.cy}
            r={n.r}
            fill="#534AB7"
            initial={{ opacity: 0.35 }}
            animate={{ opacity: [0.35, 0.88, 0.35] }}
            transition={{
              duration: 3.2 + i * 0.15,
              repeat: Infinity,
              delay: n.delay,
              ease: "easeInOut",
            }}
          />
        ))}
      </svg>
    </div>
  );
}
