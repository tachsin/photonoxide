import { ArrowRight, BookOpen, FlaskConical } from "lucide-react";
import Link from "next/link";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { METHODS_PATH, PHOTONOXIDE_LINKS, PHOTONOXIDE_OG_IMAGE, PHOTONOXIDE_PATH } from "@/lib/projects/photonoxide/meta";
import { ALGORITHMS, getMethods } from "@/lib/projects/photonoxide/methods";

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

const plural = (n, one, many) => `${n} ${n === 1 ? one : many}`;

function MethodCard({ m, heading: Heading }) {
  return (
    <Link
      href={`${METHODS_PATH}/${m.slug}`}
      className="proj-card group flex h-full flex-col p-5 transition-colors hover:border-primary/40"
    >
      {m.module ? <code className="text-base-content/55 text-xs">{m.module}</code> : null}
      <Heading className="mt-2 font-semibold text-lg tracking-tight group-hover:text-primary">{m.title}</Heading>
      <p className="proj-lead mt-1.5 flex-1 text-sm">{m.summary}</p>
      <p className="mt-4 flex flex-wrap gap-x-4 gap-y-1 text-base-content/55 text-xs">
        {m.papers.length ? (
          <span className="inline-flex items-center gap-1">
            <BookOpen size={13} aria-hidden />
            {plural(m.papers.length, "paper", "papers")}
          </span>
        ) : null}
        {m.validation.length ? (
          <span className="inline-flex items-center gap-1">
            <FlaskConical size={13} aria-hidden />
            {plural(m.validation.length, "validation case", "validation cases")}
          </span>
        ) : null}
        <span className="ml-auto inline-flex items-center gap-1 text-primary">
          Read
          <ArrowRight size={13} aria-hidden />
        </span>
      </p>
    </Link>
  );
}

export default async function MethodsPage() {
  const { ok, methods, categories } = await getMethods();
  const bySlug = new Map(methods.map((m) => [m.slug, m]));
  // each algorithm with the write-ups this release has
  const algorithms = ALGORITHMS.map((a) => ({ ...a, methods: a.methods.map((s) => bySlug.get(s)).filter(Boolean) }))
    .filter((a) => a.methods.length)
    .sort((a, b) => a.name.localeCompare(b.name));

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
        <>
          <nav aria-label="Categories" className="proj-rise-1 mt-8 flex flex-wrap gap-2">
            {categories.map((c) => (
              <a key={c.id} href={`#${c.id}`} className="proj-pill">
                {c.title}
                <span className="ml-1.5 text-base-content/50">
                  {c.groups.reduce((n, g) => n + g.methods.length, 0)}
                </span>
              </a>
            ))}
            {algorithms.length ? (
              <a href="#algorithms" className="proj-pill">
                Algorithms, A to Z
              </a>
            ) : null}
          </nav>

          {categories.map((c) => (
            <section key={c.id} className="mt-14" aria-labelledby={c.id}>
              <div className="max-w-3xl">
                <h2 id={c.id} className="scroll-mt-32 font-semibold text-3xl tracking-tight">
                  {c.title}
                </h2>
                <p className="proj-lead mt-2">{c.blurb}</p>
              </div>
              {c.groups.map((g) => (
                <div key={g.title ?? c.id} className="mt-6">
                  {g.title ? (
                    <h3 className="mb-3 font-medium text-base-content/70 text-sm uppercase tracking-wide">{g.title}</h3>
                  ) : null}
                  <ul className="grid gap-4 md:grid-cols-2">
                    {g.methods.map((m) => (
                      <li key={m.slug}>
                        <MethodCard m={m} heading={g.title ? "h4" : "h3"} />
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </section>
          ))}

          {algorithms.length ? (
            <section className="mt-16" aria-labelledby="algorithms">
              <div className="max-w-3xl">
                <h2 id="algorithms" className="scroll-mt-32 font-semibold text-3xl tracking-tight">
                  Algorithms, A to Z
                </h2>
                <p className="proj-lead mt-2">
                  The named algorithms inside the methods, and the write-ups that derive, validate and measure them.
                </p>
              </div>
              <dl className="proj-card mt-6 divide-y divide-base-content/10">
                {algorithms.map((a) => (
                  <div key={a.name} className="grid gap-x-6 gap-y-2 px-5 py-3 sm:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
                    <dt>
                      <span className="font-medium">{a.name}</span>
                      <span className="block text-base-content/60 text-sm">{a.does}</span>
                    </dt>
                    <dd className="flex flex-wrap items-start gap-2">
                      {a.methods.map((m) => (
                        <Link key={m.slug} href={`${METHODS_PATH}/${m.slug}`} className="proj-pill">
                          {m.title}
                        </Link>
                      ))}
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          ) : null}
        </>
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
