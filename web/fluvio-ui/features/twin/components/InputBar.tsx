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
      className="touch-manipulation shrink-0 border-t border-white/[0.08] bg-[#0c0b14]/98 px-3 pb-[max(0.65rem,env(safe-area-inset-bottom))] pt-3 backdrop-blur-xl sm:px-4"
    >
      <div className="mx-auto flex max-w-2xl items-end gap-2 rounded-[1.25rem] border border-white/[0.08] bg-[#12111c] px-1 py-1 pl-3 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] sm:pl-3.5">
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
          className="min-h-[2.75rem] min-w-0 flex-1 bg-transparent py-2.5 text-[16px] text-white placeholder:text-zinc-600 focus:outline-none disabled:opacity-45 sm:min-h-11 sm:text-[15px]"
        />
        <button
          type="submit"
          disabled={disabled || !value.trim()}
          className="flex min-h-[2.75rem] min-w-[3.25rem] shrink-0 items-center justify-center rounded-[0.9rem] bg-[#534AB7] px-3.5 text-[14px] font-semibold text-white shadow-sm transition active:scale-[0.98] enabled:hover:bg-[#6258c9] disabled:bg-zinc-800 disabled:text-zinc-500 disabled:shadow-none sm:min-h-11 sm:min-w-[4.25rem] sm:text-[13px]"
        >
          Send
        </button>
      </div>
    </form>
  );
}
