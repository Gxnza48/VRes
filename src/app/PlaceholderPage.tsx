import { ExternalLink, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { inTauri, ipc } from "@/lib/ipc";
import { ROADMAP_URL } from "@/lib/links";

import type { TabDefinition } from "./nav";

/** Sección que todavía no está implementada: cuenta qué va a traer y cuándo. */
export function PlaceholderPage({ tab }: { tab: TabDefinition }) {
  const { t } = useTranslation();
  const items = t(`pages.${tab.id}.items`, { returnObjects: true }) as string[];

  return (
    <>
      <PageHeader
        title={t(`pages.${tab.id}.title`)}
        subtitle={t(`pages.${tab.id}.subtitle`)}
        aside={
          <Badge tone="accent">
            <Sparkles aria-hidden className="size-3.5" />
            {t("phase.badge", { phase: tab.phase })}
          </Badge>
        }
      />
      <Card>
        <CardHeader>
          <CardTitle>{t("phase.upcoming")}</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="flex list-disc flex-col gap-2 pl-5 text-sm text-fg marker:text-accent">
            {items.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
          {inTauri() && (
            <div>
              <Button variant="ghost" size="sm" onClick={() => void ipc.openExternal(ROADMAP_URL)}>
                <ExternalLink aria-hidden />
                {t("phase.roadmap")}
              </Button>
            </div>
          )}
        </CardContent>
      </Card>
    </>
  );
}
