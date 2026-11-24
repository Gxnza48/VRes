import { CrosshairPage } from "@/features/crosshair/CrosshairPage";
import { DisplayPage } from "@/features/display/DisplayPage";
import { LibraryPage } from "@/features/library/LibraryPage";
import { MorePage } from "@/features/more/MorePage";
import { CountdownDialog } from "@/features/resolution/CountdownDialog";
import { ResolutionPage } from "@/features/resolution/ResolutionPage";
import { useAppStore } from "@/state/app";
import { useResolutionStore } from "@/state/resolution";

import { tabById } from "./nav";
import { Notices } from "./Notices";
import { PlaceholderPage } from "./PlaceholderPage";
import { Sidebar } from "./Sidebar";

/** Estructura de la ventana principal: barra lateral + sección activa. */
export function Shell() {
  const tab = tabById(useAppStore((state) => state.tab));
  // La cuenta regresiva de resolución aparece en cualquier pestaña.
  const pending = useResolutionStore((store) => store.state?.pending ?? null);
  const busy = useResolutionStore((store) => store.busy);

  return (
    <div className="flex h-full bg-bg text-fg">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-5xl px-8 py-8">
          <Notices />
          {tab.id === "display" ? (
            <DisplayPage />
          ) : tab.id === "library" ? (
            <LibraryPage />
          ) : tab.id === "crosshair" ? (
            <CrosshairPage />
          ) : tab.id === "resolution" ? (
            <ResolutionPage />
          ) : tab.id === "more" ? (
            <MorePage />
          ) : (
            <PlaceholderPage key={tab.id} tab={tab} />
          )}
        </div>
      </main>
      {pending && <CountdownDialog pending={pending} busy={busy} />}
    </div>
  );
}
