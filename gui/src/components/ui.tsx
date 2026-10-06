import { cn } from "../lib/cn";
import type { ButtonHTMLAttributes, ReactNode } from "react";

export function Card({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <div className={cn("rounded-xl border border-white/10 bg-[#1a1f2b] p-3", className)}>
      {children}
    </div>
  );
}

type BtnProps = ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "green" | "red" | "ghost" | "dark" };

export function Btn({ variant = "dark", className, ...rest }: BtnProps) {
  const styles = {
    green: "bg-green-600 hover:bg-green-700 text-white",
    red: "bg-red-600 hover:bg-red-700 text-white",
    ghost: "bg-transparent hover:bg-white/10 text-gray-200 border border-white/10",
    dark: "bg-white/10 hover:bg-white/15 text-white",
  }[variant];
  return (
    <button
      className={cn(
        "rounded-lg px-3 py-2 text-sm font-semibold transition disabled:cursor-not-allowed disabled:opacity-50",
        styles,
        className
      )}
      {...rest}
    />
  );
}

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs text-gray-400">{label}</span>
      {children}
    </label>
  );
}

export function TextInput(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={cn(
        "w-full rounded-lg border border-white/10 bg-[#0b0d11] px-2.5 py-2 text-sm text-gray-100 outline-none focus:border-sky-500",
        props.className
      )}
    />
  );
}

export function Select(props: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      {...props}
      className={cn(
        "rounded-lg border border-white/10 bg-[#0b0d11] px-2 py-2 text-sm text-gray-100 outline-none focus:border-sky-500",
        props.className
      )}
    />
  );
}
