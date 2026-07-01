import type { Strings } from "./types";
import { EN } from "./en";
import { PT_BR } from "./pt-BR";

export type { Strings } from "./types";

function detect(): Strings {
  const locale = navigator.language?.toLowerCase() ?? "en";
  return locale.startsWith("pt") ? PT_BR : EN;
}

export const t: Strings = detect();
