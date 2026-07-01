import { useEffect } from "react";
import { AppWindow, Group, Row } from "../../ui/components";
import { getConfig } from "../../lib/api";
import { applyTheme } from "../../lib/theme";
import { useReveal } from "../../lib/window";
import { t } from "../../lib/translations";
import s from "./Shortcuts.module.css";

export default function Shortcuts() {
  useEffect(() => {
    getConfig().then((c) => applyTheme(c.theme));
  }, []);
  useReveal();

  return (
    <AppWindow title={t.shortcutsTitle}>
      <Group title={t.general}>
        <Row title={t.scLock}>
          <span className={s.accel}>Ctrl+L</span>
        </Row>

        <Row title={t.scPreferences}>
          <span className={s.accel}>Ctrl+,</span>
        </Row>

        <Row title={t.scShortcuts}>
          <span className={s.accel}>Ctrl+/</span>
        </Row>

        <Row title={t.scClose}>
          <span className={s.accel}>Ctrl+W</span>
        </Row>
      </Group>
    </AppWindow>
  );
}
