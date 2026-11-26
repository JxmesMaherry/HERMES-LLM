import type { ReactNode } from "react";
import { ArrowUpRight, Check, LoaderCircle } from "lucide-react";
import type { ServiceStatus } from "./types";
export function Badge({ status }: { status?: ServiceStatus }) {
  const state = status?.state ?? "checking";
  return (
    <span className={`badge ${state}`}>
      <i />
      {state}
    </span>
  );
}
export function Button({
  children,
  onClick,
  disabled,
  secondary = false,
  type = "button",
}: {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  secondary?: boolean;
  type?: "button" | "submit";
}) {
  return (
    <button
      className={secondary ? "button secondary" : "button"}
      onClick={onClick}
      disabled={disabled}
      type={type}
    >
      {children}
    </button>
  );
}
export function Heading({
  eyebrow,
  title,
  children,
}: {
  eyebrow: string;
  title: string;
  children?: ReactNode;
}) {
  return (
    <div className="page-heading">
      <div>
        <p className="eyebrow">{eyebrow}</p>
        <h1>{title}</h1>
      </div>
      {children}
    </div>
  );
}
export function Empty({
  icon,
  title,
  children,
}: {
  icon?: ReactNode;
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="empty">
      {icon}
      <h2>{title}</h2>
      <p>{children}</p>
    </div>
  );
}
export function Busy({ text }: { text: string }) {
  return (
    <div className="busy" role="status">
      <LoaderCircle className="spin" size={17} />
      {text}
    </div>
  );
}
export function External({
  children,
  onClick,
}: {
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button className="text-button" onClick={onClick}>
      {children}
      <ArrowUpRight size={14} />
    </button>
  );
}
export function Step({
  done,
  active,
  children,
  n,
}: {
  done: boolean;
  active: boolean;
  children: ReactNode;
  n: number;
}) {
  return (
    <div className={`step ${active ? "active" : ""} ${done ? "done" : ""}`}>
      <span>{done ? <Check size={15} /> : n}</span>
      {children}
    </div>
  );
}
