import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import { useAppStore } from "@/state/app";

import { TABS } from "./nav";

/** Barra lateral con las secciones de VRes. */
export function Sidebar() {
  const { t } = useTranslation();
  const tab = useAppStore((state) => state.tab);
  const setTab = useAppStore((state) => state.setTab);
  const version = useAppStore((state) => state.bootstrap?.appVersion);

  return (
    <nav
      aria-label={t("nav.label")}
      className="flex w-24 shrink-0 flex-col gap-1 overflow-y-auto border-r border-border bg-surface px-2 py-4"
    >
      <div className="mb-3 flex items-center justify-center gap-1.5 px-1">
        <img src="/vres.svg" alt="" className="size-6" />
        <span className="text-base font-semibold tracking-tight">{t("app.name")}</span>
      </div>

      {TABS.map(({ id, icon: Icon }) => {
        const active = id === tab;
        return (
          <button
            key={id}
            type="button"
            aria-current={active ? "page" : undefined}
            onClick={() => setTab(id)}
            className={cn(
              "flex flex-col items-center gap-1 rounded-xl px-1 py-2.5 text-center text-xs transition-colors",
              active ? "bg-surface-2 text-fg" : "text-muted hover:bg-surface-2/60 hover:text-fg",
            )}
          >
            <Icon aria-hidden className="size-5" strokeWidth={1.75} />
            <span className="leading-tight">{t(`nav.${id}`)}</span>
          </button>
        );
      })}

      <div className="mt-auto pt-3 text-center text-[11px] text-muted">
        {version ? `v${version}` : null}
      </div>
    </nav>
  );
}
