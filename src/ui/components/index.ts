import appWindow from "./AppWindow/AppWindow.module.css";
import group from "./Group/Group.module.css";
import row from "./Row/Row.module.css";
import toggle from "./Toggle/Toggle.module.css";
import select from "./Select/Select.module.css";
import shared from "./shared.module.css";

export { AppWindow } from "./AppWindow";
export { Group } from "./Group";
export { Row } from "./Row";
export { Toggle } from "./Toggle";
export { Select } from "./Select";

/** Every component's CSS module classes, merged so screens can use them by
 * class (e.g. `ui.btn`, `ui.card`) without depending on a single component. */
export const ui = { ...appWindow, ...group, ...row, ...toggle, ...select, ...shared };
