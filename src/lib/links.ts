import { inTauri, ipc } from "@/lib/ipc";

/** Enlaces públicos del proyecto. */
export const REPOSITORY_URL = "https://github.com/Gxnza48/VRes";
export const ROADMAP_URL = `${REPOSITORY_URL}/blob/main/docs/ROADMAP.md`;
export const ISSUES_URL = `${REPOSITORY_URL}/issues/new/choose`;
/** Servidor de la comunidad. */
export const DISCORD_URL = "https://discord.gg/aG2ByWDHfG";
/** Sitio web de VRes. */
export const SITE_URL = "https://vresolution.vercel.app";
/** Galería de crosshairs con sus códigos (no se cargan en la app). */
export const CROSSHAIRS_URL = `${SITE_URL}/miras`;

/** Abre una página del sitio de VRes en el navegador. */
export function openSite(url: string): void {
  if (inTauri()) void ipc.openExternal(url);
  else window.open(url, "_blank", "noopener");
}
