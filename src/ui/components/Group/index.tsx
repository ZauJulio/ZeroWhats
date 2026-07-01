import { ReactNode } from "react";
import s from "./Group.module.css";

export function Group({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <section className={s.group}>
      {title && <h2 className={s.groupTitle}>{title}</h2>}
      <div className={s.card}>{children}</div>
    </section>
  );
}
