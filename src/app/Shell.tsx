import { DisplayPage } from "@/features/display/DisplayPage";
import { MorePage } from "@/features/more/MorePage";
import { ResolutionPage } from "@/features/resolution/ResolutionPage";
import { useAppStore } from "@/state/app";

import { tabById } from "./nav";
import { Notices } from "./Notices";
import { PlaceholderPage } from "./PlaceholderPage";
import { Sidebar } from "./Sidebar";

/** Estructura de la ventana principal: barra lateral + sección activa. */
export function Shell() {
  const tab = tabById(useAppStore((state) => state.tab));

  return (
    <div className="flex h-full bg-bg text-fg">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-5xl px-8 py-8">
          <Notices />
          {tab.id === "display" ? (
            <DisplayPage />
          ) : tab.id === "resolution" ? (
            <ResolutionPage />
          ) : tab.id === "more" ? (
            <MorePage />
          ) : (
            <PlaceholderPage key={tab.id} tab={tab} />
          )}
        </div>
      </main>
    </div>
  );
}
