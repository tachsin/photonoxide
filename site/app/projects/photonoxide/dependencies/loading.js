/** While the registries are read for the first time since the server started (a few seconds). */
export default function DependenciesLoading() {
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <header className="max-w-3xl">
        <p className="proj-eyebrow">Dependencies</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Every version we use</h1>
        <p className="proj-lead mt-4 text-lg">Reading the newest releases from the registries…</p>
      </header>
      <div className="proj-chart-skeleton mt-10 h-96 max-w-5xl rounded-2xl" />
    </main>
  );
}
