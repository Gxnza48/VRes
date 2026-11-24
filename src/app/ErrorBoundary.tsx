import { Component, type ErrorInfo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";

interface State {
  error: Error | null;
}

/** Si la interfaz falla, muestra un aviso en vez de una ventana en blanco. */
export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("interfaz", error, info.componentStack);
  }

  render() {
    return this.state.error ? <ErrorScreen /> : this.props.children;
  }
}

function ErrorScreen() {
  const { t } = useTranslation();
  return (
    <div role="alert" className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
      <h1 className="text-2xl font-semibold text-fg">{t("app.error.title")}</h1>
      <p className="max-w-md text-sm text-muted">{t("app.error.description")}</p>
      <Button variant="primary" onClick={() => window.location.reload()}>
        {t("app.error.reload")}
      </Button>
    </div>
  );
}
