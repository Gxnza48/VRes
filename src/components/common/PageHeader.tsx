import type { ReactNode } from "react";

interface PageHeaderProps {
  title: string;
  subtitle: string;
  aside?: ReactNode;
}

/** Encabezado común de cada sección. */
export function PageHeader({ title, subtitle, aside }: PageHeaderProps) {
  return (
    <header className="mb-6 flex items-start justify-between gap-6">
      <div className="flex flex-col gap-1.5">
        <h1 className="text-3xl font-semibold tracking-tight text-fg">{title}</h1>
        <p className="max-w-2xl text-sm text-muted">{subtitle}</p>
      </div>
      {aside}
    </header>
  );
}
