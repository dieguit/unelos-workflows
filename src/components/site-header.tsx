export function SiteHeader({ title }: { title: string }) {
  return (
    <header className="flex flex-col gap-1">
      <h1 className="text-2xl font-semibold">{title}</h1>
      <p className="text-sm text-foreground-dim">
        {title} content is not implemented yet.
      </p>
    </header>
  )
}
