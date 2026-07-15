import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { Download } from "lucide-react";
import { AppWindow, ui } from "../../ui/components";
import { checkForUpdate, openUrl, getConfig } from "../../lib/api";
import { applyTheme } from "../../lib/theme";
import { useReveal } from "../../lib/window";
import { t } from "../../lib/translations";
import { cx } from "../../lib/cx";
import s from "./Update.module.css";

interface ReleaseInfo {
  tag_name: string;
  name: string;
  body: string;
  html_url: string;
  published_at: string;
}

export default function Update() {
  const [version, setVersion] = useState("");
  const [release, setRelease] = useState<ReleaseInfo | null>(null);
  const [checking, setChecking] = useState(true);

  useEffect(() => {
    getConfig().then((c) => applyTheme(c.theme));
    getVersion().then(setVersion);
    checkForUpdate()
      .then((r) => {
        setRelease(r);
        setChecking(false);
      })
      .catch(() => setChecking(false));
  }, []);
  useReveal();

  return (
    <AppWindow title={t.checkForUpdates}>
      <div className={s.update}>
        <img src="/icon.png" alt="ZeroWhats" />
        <h1>ZeroWhats</h1>

        {checking && <div className={s.loading}>{t.checking}...</div>}

        {!checking && !release && (
          <div className={s.upToDate}>
            <div className={s.checkmark}>&#10003;</div>
            <div>{t.upToDate}</div>
            {version && (
              <div style={{ marginTop: "0.5rem" }}>
                <span className={s.pill}>{version}</span>
              </div>
            )}
          </div>
        )}

        {!checking && release && (
          <>
            <div className={s.versions}>
              <span className={s.pill}>{version}</span>
              <span className={s.arrow}>&rarr;</span>
              <span className={s.pillNew}>{release.tag_name}</span>
            </div>

            <div className={s.notesLabel}>{t.releaseNotes}</div>
            <div className={cx(ui.card, s.notes)}>{release.body}</div>

            <div className={s.actions}>
              <button className={s.primary} onClick={() => openUrl(release.html_url)}>
                <Download size={16} style={{ marginRight: 6, verticalAlign: -3 }} />
                {t.downloadUpdate}
              </button>
            </div>
          </>
        )}
      </div>
    </AppWindow>
  );
}
