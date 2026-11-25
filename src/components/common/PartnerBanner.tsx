import { ArrowUpRight, BadgeCheck } from "lucide-react";
import { useTranslation } from "react-i18next";

import { inTauri, ipc } from "@/lib/ipc";
import { PARTNER_ROBRUN_DISCORD_URL } from "@/lib/links";

function DiscordIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden className={className} fill="currentColor">
      <path d="M20.32 4.37a19.8 19.8 0 0 0-4.89-1.52.07.07 0 0 0-.08.04c-.21.38-.44.87-.61 1.25a18.3 18.3 0 0 0-5.49 0 12.6 12.6 0 0 0-.62-1.25.08.08 0 0 0-.08-.04 19.7 19.7 0 0 0-4.89 1.52.07.07 0 0 0-.03.03C.53 9.05-.32 13.58.1 18.06a.08.08 0 0 0 .03.06 19.9 19.9 0 0 0 5.99 3.03.08.08 0 0 0 .08-.03c.46-.63.87-1.3 1.23-1.99a.08.08 0 0 0-.04-.11 13.1 13.1 0 0 1-1.87-.89.08.08 0 0 1-.01-.13l.37-.29a.07.07 0 0 1 .08-.01c3.93 1.79 8.18 1.79 12.07 0a.07.07 0 0 1 .08.01l.37.29a.08.08 0 0 1-.01.13c-.6.35-1.22.65-1.87.89a.08.08 0 0 0-.04.11c.36.7.77 1.36 1.22 1.99a.08.08 0 0 0 .08.03 19.8 19.8 0 0 0 6-3.03.08.08 0 0 0 .03-.06c.5-5.18-.84-9.67-3.55-13.66a.06.06 0 0 0-.03-.03ZM8.02 15.33c-1.18 0-2.16-1.08-2.16-2.42 0-1.33.96-2.42 2.16-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.34-.96 2.42-2.16 2.42Zm7.97 0c-1.18 0-2.15-1.08-2.15-2.42 0-1.33.95-2.42 2.15-2.42 1.21 0 2.18 1.1 2.16 2.42 0 1.34-.95 2.42-2.16 2.42Z" />
    </svg>
  );
}

/** Banner del servidor partner (ROB RUN LATAM): abre su Discord. */
export function PartnerBanner() {
  const { t } = useTranslation();

  const open = () => {
    if (inTauri()) void ipc.openExternal(PARTNER_ROBRUN_DISCORD_URL);
    else window.open(PARTNER_ROBRUN_DISCORD_URL, "_blank", "noopener");
  };

  return (
    <button
      type="button"
      onClick={open}
      aria-label={t("partner.aria")}
      className="partner-sweep group relative mb-6 block h-48 w-full overflow-hidden rounded-card border border-gold/40 text-left transition hover:border-gold/80"
    >
      <img
        src="/partners/robrun-latam.jpg"
        alt=""
        className="absolute inset-0 size-full object-cover object-[50%_42%] transition duration-500 group-hover:scale-[1.03]"
      />
      <div aria-hidden className="partner-scrim absolute inset-0" />
      <div className="relative flex h-full items-center justify-between gap-4 px-6">
        <div className="flex flex-col gap-1.5">
          <span className="partner-gold flex items-center gap-2 font-display text-xl font-black tracking-[0.18em] uppercase">
            <BadgeCheck aria-hidden className="size-5 text-gold" />
            {t("partner.badge")}
          </span>
          <span className="font-display text-sm font-bold tracking-wider text-fg uppercase">
            {t("partner.name")}
          </span>
          <span className="text-xs text-fg/75">{t("partner.text")}</span>
        </div>
        <span className="inline-flex shrink-0 items-center gap-2 rounded-xl bg-discord px-4 py-2.5 text-sm font-semibold text-discord-fg shadow-lg transition group-hover:brightness-110">
          <DiscordIcon className="size-4" />
          {t("partner.cta")}
          <ArrowUpRight aria-hidden className="size-4" />
        </span>
      </div>
    </button>
  );
}
