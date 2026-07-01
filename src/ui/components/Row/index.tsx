import { ReactNode } from "react";
import s from "./Row.module.css";

export function Row({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children?: ReactNode;
}) {
  return (
    <div className={s.row}>
      <div className={s.rowText}>
        <div className={s.rowTitle}>{title}</div>
        {subtitle && <div className={s.rowSubtitle}>{subtitle}</div>}
      </div>
      <div className={s.rowControl}>{children}</div>
    </div>
  );
}
