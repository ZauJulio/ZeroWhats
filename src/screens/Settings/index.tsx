import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AppWindow, Group, Row, Toggle, Select, ui } from "../../ui/components";
import { cx } from "../../lib/cx";
import { ConfigView, Theme, getConfig, saveConfig, setPassword, setTheme } from "../../lib/api";
import { applyTheme } from "../../lib/theme";
import { useReveal } from "../../lib/window";
import { t } from "../../lib/translations";
import styles from "./Settings.module.css";

export default function Settings() {
  const [cfg, setCfg] = useState<ConfigView | null>(null);
  const [newPassword, setNewPassword] = useState("");

  useEffect(() => {
    getConfig().then((c) => {
      setCfg(c);
      applyTheme(c.theme);
    });
  }, []);

  // Reveal only once the config is loaded and the screen has rendered.
  useReveal(cfg !== null);

  if (!cfg) return null;

  const update = (patch: Partial<ConfigView>) => {
    const next = { ...cfg, ...patch };
    setCfg(next);

    const { has_password: _has, ...rest } = next;
    saveConfig(rest);
  };

  const savePassword = () => {
    if (!newPassword) return;

    setPassword(newPassword);
    setCfg({ ...cfg, has_password: true });
    setNewPassword("");
  };

  const removePassword = () => {
    setPassword(null);
    setCfg({ ...cfg, has_password: false });
  };

  const chooseFolder = async () => {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") update({ download_path: dir });
  };

  // Theme drives three things: persisted config, this window's chrome, and the
  // WhatsApp page (which reloads to pick it up).
  const changeTheme = (theme: Theme) => {
    update({ theme });
    applyTheme(theme);
    setTheme(theme);
  };

  return (
    <AppWindow title={t.settingsTitle}>
      <Group title={t.general}>
        <Row title={t.theme} subtitle={t.themeDesc}>
          <Select<Theme>
            value={cfg.theme}
            onChange={changeTheme}
            options={[
              { value: "system", label: t.themeSystem },
              { value: "light", label: t.themeLight },
              { value: "dark", label: t.themeDark },
            ]}
          />
        </Row>

        <Row title={t.language} subtitle={t.languageDesc}>
          <Select<string>
            value={cfg.locale ?? "system"}
            onChange={(v) => update({ locale: v === "system" ? null : v })}
            options={[
              { value: "system", label: t.languageSystem },
              { value: "en", label: "English" },
              { value: "pt-br", label: "Português (Brasil)" },
            ]}
          />
        </Row>
      </Group>

      <Group title={t.security}>
        <Row title={t.appLock} subtitle={t.appLockDesc}>
          {cfg.has_password ? (
            <button className={cx(ui.btn, ui.danger)} onClick={removePassword}>
              {t.removePassword}
            </button>
          ) : (
            <div className={styles.passwordRow}>
              <input
                className={ui.input}
                type="password"
                placeholder={t.newPassword}
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
              />
              
              {newPassword && (
                <button className={cx(ui.btn, ui.accent)} onClick={savePassword}>
                  {t.savePassword}
                </button>
              )}
            </div>
          )}
        </Row>

        <Row title={t.lockOnClose} subtitle={t.lockOnCloseDesc}>
          <Toggle checked={cfg.lock_on_close} onChange={(v) => update({ lock_on_close: v })} />
        </Row>

        <Row
          title={t.autoLock}
          subtitle={cfg.has_password ? t.autoLockDesc : t.autoLockNeedsPassword}
        >
          <Select<string>
            value={String(cfg.auto_lock_minutes ?? 0)}
            disabled={!cfg.has_password}
            onChange={(v) => update({ auto_lock_minutes: Number(v) === 0 ? null : Number(v) })}
            options={[
              { value: "0", label: t.autoLockOff },
              { value: "1", label: t.autoLock1 },
              { value: "5", label: t.autoLock5 },
              { value: "15", label: t.autoLock15 },
              { value: "30", label: t.autoLock30 },
            ]}
          />
        </Row>

        <Row title={t.proxy} subtitle={t.proxyDesc}>
          <Toggle checked={cfg.proxy_enabled} onChange={(v) => update({ proxy_enabled: v })} />
        </Row>

        {cfg.proxy_enabled && (
          <Row title={t.proxyUrl}>
            <input
              className={ui.input}
              type="text"
              placeholder="http://127.0.0.1:8080"
              value={cfg.proxy_url}
              onChange={(e) => setCfg({ ...cfg, proxy_url: e.target.value })}
              onBlur={() => update({ proxy_url: cfg.proxy_url })}
            />
          </Row>
        )}
      </Group>

      <Group title={t.downloads}>
        <Row title={t.autoDownload} subtitle={t.autoDownloadDesc}>
          <Toggle checked={cfg.auto_download} onChange={(v) => update({ auto_download: v })} />
        </Row>

        <Row title={t.downloadFolder}>
          <button className={ui.btn} onClick={chooseFolder}>
            {cfg.download_path ?? t.choose}
          </button>
        </Row>
      </Group>

      <Group title={t.notifications}>
        <Row title={t.mute} subtitle={t.muteDesc}>
          <Toggle
            checked={cfg.mute_notifications}
            onChange={(v) => update({ mute_notifications: v })}
          />
        </Row>
      </Group>

      <Group title={t.advanced}>
        <Row title={t.hwAccel} subtitle={t.hwAccelDesc}>
          <Toggle
            checked={cfg.hardware_acceleration}
            onChange={(v) => update({ hardware_acceleration: v })}
          />
        </Row>

        <Row title={t.launchAtLogin} subtitle={t.launchAtLoginDesc}>
          <Toggle checked={cfg.auto_start} onChange={(v) => update({ auto_start: v })} />
        </Row>

        <Row title={t.cache} subtitle={t.cacheDesc}>
          <Toggle checked={cfg.cache_enabled} onChange={(v) => update({ cache_enabled: v })} />
        </Row>
      </Group>
    </AppWindow>
  );
}
