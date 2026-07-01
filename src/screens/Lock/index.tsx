import { useEffect, useRef, useState } from "react";
import { Lock as LockIcon } from "lucide-react";
import { ask } from "@tauri-apps/plugin-dialog";
import {
  getConfig,
  unlockApp,
  resetPassword,
  forgetPasswordWipe,
  getPlatform,
} from "../../lib/api";
import { applyTheme } from "../../lib/theme";
import { useReveal } from "../../lib/window";
import { t } from "../../lib/translations";
import { cx } from "../../lib/cx";
import { ui } from "../../ui/components";
import lk from "./Lock.module.css";

export default function Lock() {
  const [input, setInput] = useState("");
  const [error, setError] = useState("");
  const [isLinux, setIsLinux] = useState(false);
  const [cooldown, setCooldown] = useState(0);
  const attempts = useRef(0);

  useEffect(() => {
    getConfig().then((c) => applyTheme(c.theme));
    getPlatform().then((os) => setIsLinux(os === "linux"));
  }, []);

  useReveal();

  // Tick down the post-failure cooldown (one decrement per second).
  useEffect(() => {
    if (cooldown <= 0) return;
    const id = setTimeout(() => setCooldown((s) => Math.max(0, s - 1)), 1000);
    return () => clearTimeout(id);
  }, [cooldown]);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (cooldown > 0) return;

    const ok = await unlockApp(input);

    if (ok) {
      attempts.current = 0;
      // On success the backend closes this window and shows the main one.
      return;
    }

    attempts.current += 1;
    setError(t.wrongPassword);
    setInput("");

    // After a few misses, enforce a growing delay (1, 2, 4, 8 … capped at 30s).
    if (attempts.current >= 3) {
      setCooldown(Math.min(2 ** (attempts.current - 3), 30));
    }
  };

  // Linux: authenticate as a system admin via polkit and clear just the
  // password. Everywhere else there is no equivalent system-auth one-liner, so
  // the only safe recovery is a full reset — log WhatsApp out and erase all
  // local data — gated behind an explicit confirmation.
  const forgot = async () => {
    if (isLinux) {
      const ok = await resetPassword();

      if (ok)
        await unlockApp(""); // password cleared -> empty unlock succeeds
      else setError(t.resetFailed);

      return;
    }

    const confirmed = await ask(t.wipeConfirmBody, {
      title: t.wipeConfirmTitle,
      kind: "warning",
      okLabel: t.wipeConfirmOk,
    });

    if (confirmed) await forgetPasswordWipe();
  };

  const locked = cooldown > 0;

  return (
    <div className={lk.lock}>
      <div className={lk.lockBox}>
        <LockIcon className={lk.lockIcon} size={48} />

        <h1>{t.lockHeading}</h1>

        <p>{t.lockSubheading}</p>

        <form onSubmit={submit}>
          <input
            className={ui.input}
            type="password"
            placeholder={t.passwordPlaceholder}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            disabled={locked}
            autoFocus
          />

          {locked ? (
            <span className={lk.lockError}>
              {t.tryAgainIn} {cooldown}s
            </span>
          ) : (
            error && <span className={lk.lockError}>{error}</span>
          )}

          <button
            className={cx(ui.btn, ui.accent, ui.unlock, lk.unlockBtn)}
            type="submit"
            disabled={locked}
          >
            {t.unlock}
          </button>

          <button className={lk.lockForgot} type="button" onClick={forgot}>
            {t.forgotPassword}
          </button>
        </form>
      </div>
    </div>
  );
}
