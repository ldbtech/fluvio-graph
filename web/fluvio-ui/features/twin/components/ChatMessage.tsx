"use client";

import { motion } from "framer-motion";

export type ChatRole = "user" | "assistant";

type Props = {
  role: ChatRole;
  content: string;
};

export function ChatMessage({ role, content }: Props) {
  const isUser = role === "user";
  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.28, ease: [0.22, 1, 0.36, 1] }}
      className={`flex w-full ${isUser ? "justify-end" : "justify-start"}`}
    >
      <div
        className={
          isUser
            ? "max-w-[min(92%,28rem)] rounded-[1.15rem] bg-zinc-100 px-3.5 py-2.5 text-[15px] leading-relaxed text-zinc-950 shadow-[0_1px_0_rgba(0,0,0,0.06)]"
            : "max-w-[min(92%,28rem)] rounded-[1.15rem] border border-white/[0.07] border-l-[3px] border-l-[#534AB7] bg-[#14131e] px-3.5 py-2.5 text-[15px] leading-relaxed text-zinc-100 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]"
        }
      >
        <p className="whitespace-pre-wrap break-words">{content}</p>
      </div>
    </motion.div>
  );
}
