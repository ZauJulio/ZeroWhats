import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { ExternalLink, Globe, Code2, Bug, Mail } from "lucide-react";
import { AppWindow, ui } from "../../ui/components";
import { getConfig, openUrl } from "../../lib/api";
import { applyTheme } from "../../lib/theme";
import { useReveal } from "../../lib/window";
import { t } from "../../lib/translations";
import { cx } from "../../lib/cx";
import s from "./About.module.css";

const REPO = "https://github.com/ZauJulio/ZeroWhats";
const ISSUES = "https://github.com/ZauJulio/ZeroWhats/issues";
const EMAIL = "zaujulio.dev@gmail.com";

function LinkRow({ icon, label, uri }: { icon: React.ReactNode; label: string; uri: string }) {
  return (
    <button className={s.linkRow} onClick={() => openUrl(uri)}>
      {icon}
      <span className={s.label}>{label}</span>
      <ExternalLink className={s.ext} size={16} />
    </button>
  );
}

export default function About() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getConfig().then((c) => applyTheme(c.theme));
    getVersion().then(setVersion);
  }, []);
  useReveal();

  return (
    <AppWindow title="About ZeroWhats">
      <div className={s.about}>
        <img src="/icon.png" alt="ZeroWhats" />
        <h1>ZeroWhats</h1>
        <div className={s.dev}>by ZauJulio</div>
        {version && <span className={s.pill}>{version}</span>}
        <p className={s.comments}>{t.aboutComments}</p>
        <div className={cx(ui.card, s.links)}>
          <LinkRow icon={<Globe size={18} />} label={t.website} uri={REPO} />
          <LinkRow icon={<Code2 size={18} />} label={t.sourceCode} uri={REPO} />
          <LinkRow icon={<Bug size={18} />} label={t.reportIssue} uri={ISSUES} />
          <LinkRow icon={<Mail size={18} />} label={t.contactDev} uri={`mailto:${EMAIL}`} />
        </div>
      </div>
    </AppWindow>
  );
}
