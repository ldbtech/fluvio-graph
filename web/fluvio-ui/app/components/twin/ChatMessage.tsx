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
            ? "max-w-[min(92%,28rem)] rounded-2xl bg-white px-3.5 py-2.5 text-[15px] leading-relaxed text-[#0A0A0F] shadow-sm"
            : "max-w-[min(92%,28rem)] rounded-2xl border-l-[3px] border-[#534AB7] bg-[#1A1828] px-3.5 py-2.5 text-[15px] leading-relaxed text-[#FFFFFF]"
        }
      >
        <p className="whitespace-pre-wrap break-words">{content}</p>
      </div>
    </motion.div>
  );
}
