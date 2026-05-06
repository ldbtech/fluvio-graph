"use client";

import { useCallback, type FormEvent, type KeyboardEvent } from "react";

type Props = {
  value: string;
  onChange: (v: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  disabled?: boolean;
};

function vibrateShort() {
  if (typeof navigator !== "undefined" && typeof navigator.vibrate === "function") {
    navigator.vibrate(12);
  }
}

export function InputBar({ value, onChange, onSubmit, placeholder = "Ask Ali anything...", disabled }: Props) {
  const submit = useCallback(() => {
    if (!value.trim() || disabled) return;
    vibrateShort();
    onSubmit();
  }, [value, disabled, onSubmit]);

  const onFormSubmit = useCallback(
    (e: FormEvent) => {
      e.preventDefault();
      submit();
    },
    [submit],
  );

  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        submit();
      }
    },
    [submit],
  );

  return (
    <form
      onSubmit={onFormSubmit}
      className="touch-manipulation border-t border-[#5F5E5A]/40 bg-[#0A0A0F]/90 px-3 pb-[max(0.75rem,env(safe-area-inset-bottom))] pt-2.5 backdrop-blur-md sm:px-4"
    >
      <div className="mx-auto flex max-w-2xl items-end gap-2">
        <input
          type="text"
          enterKeyHint="send"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={placeholder}
          disabled={disabled}
          autoComplete="off"
          autoCorrect="on"
          className="min-h-11 min-w-0 flex-1 bg-transparent py-2.5 text-[16px] text-[#FFFFFF] placeholder:text-[#5F5E5A] focus:outline-none disabled:opacity-45 sm:text-[15px]"
        />
        <button
          type="submit"
          disabled={disabled || !value.trim()}
          className="flex min-h-11 min-w-11 shrink-0 items-center justify-center rounded-xl px-3 text-[14px] font-semibold text-[#AFA9EC] transition active:scale-[0.97] enabled:hover:bg-white/[0.06] enabled:hover:text-white disabled:opacity-35 sm:min-w-14 sm:text-[13px]"
        >
          Send
        </button>
      </div>
    </form>
  );
}
