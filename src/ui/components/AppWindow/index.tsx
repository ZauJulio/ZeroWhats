import { ReactNode, useEffect } from "react";
import { X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { cx } from "../../../lib/cx";
import s from "./AppWindow.module.css";

/**
 * Window chrome wrapper. Every frameless secondary window draws its own React
 * titlebar with a drag region and a close button.
 */
export function AppWindow({ title, children }: { title: string; children: ReactNode }) {
  useEffect(() => {
    const close = () => getCurrentWindow().close();
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "w") {
        e.preventDefault();
        close();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <div className={s.window}>
      <header className={s.titlebar} data-tauri-drag-region>
        <span className={s.title} data-tauri-drag-region>
          {title}
        </span>

        <button
          className={cx(s.winBtn, s.close)}
          onClick={() => getCurrentWindow().close()}
          title="Close"
        >
          <X size={18} />
        </button>
      </header>
      <main className={s.content}>{children}</main>
    </div>
  );
}
