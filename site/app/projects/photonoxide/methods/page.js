import { ArrowRight, BookOpen, FlaskConical } from "lucide-react";
import Link from "next/link";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { METHODS_PATH, PHOTONOXIDE_LINKS, PHOTONOXIDE_OG_IMAGE, PHOTONOXIDE_PATH } from "@/lib/projects/photonoxide/meta";
import { getMethods } from "@/lib/projects/photonoxide/methods";

const DESCRIPTION =
  "Every method photonoxide implements: its equations, the paper it comes from, how it was validated, and where it is weak.";

export const metadata = projectsMetadata({
  title: "Methods",
  fullTitle: "Methods — photonoxide",
  description: DESCRIPTION,
  path: METHODS_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Methods", path: METHODS_PATH },
];

export default async function MethodsPage() {
  const { ok, methods } = await getMethods();
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(CRUMBS)} />
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Methods</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">How photonoxide computes</h1>
        <p className="proj-lead mt-4 text-lg">
          {DESCRIPTION} Each one names its paper by DOI, says how a sign or a misprint in it was handled, and
          shows the numbers it was checked against, with their grids.
        </p>
      </header>

      {ok ? (
        <ol className="proj-rise-1 mt-10 grid gap-4 md:grid-cols-2">
          {methods.map((m, i) => (
            <li key={m.slug}>
              <Link
                href={`${METHODS_PATH}/${m.slug}`}
                className="proj-card group flex h-full flex-col p-5 transition-colors hover:border-primary/40"
              >
                <div className="flex items-center gap-2">
                  <span className="proj-tag" data-tone="accent">
                    {i + 1}
                  </span>
                  {m.module ? <code className="text-base-content/55 text-xs">{m.module}</code> : null}
                </div>
                <h2 className="mt-3 font-semibold text-lg tracking-tight group-hover:text-primary">{m.title}</h2>
                <p className="proj-lead mt-1.5 flex-1 text-sm">{m.summary}</p>
                <p className="mt-4 flex flex-wrap gap-x-4 gap-y-1 text-base-content/55 text-xs">
                  <span className="inline-flex items-center gap-1">
                    <BookOpen size={13} aria-hidden />
                    {m.papers.length} {m.papers.length === 1 ? "paper" : "papers"}
                  </span>
                  {m.validation.length ? (
                    <span className="inline-flex items-center gap-1">
                      <FlaskConical size={13} aria-hidden />
                      {m.validation.length} validation {m.validation.length === 1 ? "case" : "cases"}
                    </span>
                  ) : null}
                  <span className="ml-auto inline-flex items-center gap-1 text-primary">
                    Read
                    <ArrowRight size={13} aria-hidden />
                  </span>
                </p>
              </Link>
            </li>
          ))}
        </ol>
      ) : (
        <div className="mt-10">
          <SourceUnavailable href={PHOTONOXIDE_LINKS.methodsDir} linkLabel="Methods on GitHub">
            The methods are written up in the repository's docs/methods folder.
          </SourceUnavailable>
        </div>
      )}
    </main>
  );
}
