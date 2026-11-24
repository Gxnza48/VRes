import { useEffect } from "react";

import { ErrorBoundary } from "@/app/ErrorBoundary";
import { Shell } from "@/app/Shell";
import { startApp } from "@/app/startup";

export default function App() {
  useEffect(() => {
    void startApp();
  }, []);

  return (
    <ErrorBoundary>
      <Shell />
    </ErrorBoundary>
  );
}
