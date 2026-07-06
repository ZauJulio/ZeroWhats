// IPC layer: typed bindings to the Rust commands. These are only callable from
// the local React windows (the remote WhatsApp window talks to Rust via events).
import { invoke } from "@tauri-apps/api/core";

export type Theme = "system" | "light" | "dark";

/** How much of a notification's content is shown natively. */
export type NotificationPrivacy = "full" | "generic" | "hidden";

export interface ConfigView {
  theme: Theme;
  locale: string | null;
  proxy_enabled: boolean;
  proxy_url: string;
  auto_download: boolean;
  download_path: string | null;
  notification_privacy: NotificationPrivacy;
  hide_content_on_unfocus: boolean;
  cache_enabled: boolean;
  auto_start: boolean;
  hardware_acceleration: boolean;
  lock_on_close: boolean;
  auto_lock_minutes: number | null;
  has_password: boolean;
}

export type ConfigPatch = Omit<ConfigView, "has_password">;

export const getConfig = () => invoke<ConfigView>("get_config");
export const saveConfig = (patch: ConfigPatch) => invoke("save_config", { patch });
export const setPassword = (plain: string | null) => invoke("set_password", { plain });
export const resetPassword = () => invoke<boolean>("reset_password");
/** Non-Linux recovery: log WhatsApp out and erase all local data + password. */
export const forgetPasswordWipe = () => invoke("forget_password_wipe");
export const getPlatform = () => invoke<string>("get_platform");
export const setTheme = (theme: Theme) => invoke("set_theme", { theme });
export const unlockApp = (password: string) => invoke<boolean>("unlock", { password });
export const openUrl = (url: string) => invoke("open_url", { url });
