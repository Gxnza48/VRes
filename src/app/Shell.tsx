import { CrosshairPage } from "@/features/crosshair/CrosshairPage";
import { DisplayPage } from "@/features/display/DisplayPage";
import { EnhancersPage } from "@/features/enhancers/EnhancersPage";
import { HomePage } from "@/features/home/HomePage";
import { LibraryPage } from "@/features/library/LibraryPage";
import { PotatoPage } from "@/features/potato/PotatoPage";
import { ProfilesPage } from "@/features/profiles/ProfilesPage";
import { MorePage } from "@/features/more/MorePage";
import { CountdownDialog } from "@/features/resolution/CountdownDialog";
import { ResolutionPage } from "@/features/resolution/ResolutionPage";
import { useAppStore } from "@/state/app";
import { useResolutionStore } from "@/state/resolution";

import { Backdrop } from "./Backdrop";
import { tabById } from "./nav";
import { Notices } from "./Notices";
import { Sidebar } from "./Sidebar";
import { useTheme } from "./theme";

/** Estructura de la ventana principal: barra lateral + sección activa. */
export function Shell() {
  const tab = tabById(useAppStore((state) => state.tab));
  // La cuenta regresiva de resolución aparece en cualquier pestaña.
  const pending = useResolutionStore((store) => store.state?.pending ?? null);
  const busy = useResolutionStore((store) => store.busy);
  useTheme({ scale: true });

  return (
    <div className="relative flex h-full overflow-hidden bg-bg text-fg">
      <Backdrop />
      <Sidebar />
      <main className="relative z-10 flex-1 overflow-y-auto">
        <div className="mx-auto max-w-5xl px-8 py-8">
          <Notices />
          {tab.id === "home" ? (
            <HomePage />
          ) : tab.id === "display" ? (
            <DisplayPage />
          ) : tab.id === "profiles" ? (
            <ProfilesPage />
          ) : tab.id === "library" ? (
            <LibraryPage />
          ) : tab.id === "crosshair" ? (
            <CrosshairPage />
          ) : tab.id === "resolution" ? (
            <ResolutionPage />
          ) : tab.id === "enhancers" ? (
            <EnhancersPage />
          ) : tab.id === "potato" ? (
            <PotatoPage />
          ) : (
            <MorePage />
          )}
        </div>
      </main>
      {pending && <CountdownDialog pending={pending} busy={busy} />}
    </div>
  );
}
