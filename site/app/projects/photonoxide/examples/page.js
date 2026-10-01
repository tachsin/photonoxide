import { ArrowRight } from "lucide-react";
import Link from "next/link";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { getExamples } from "@/lib/projects/photonoxide/examples";
import { EXAMPLES_PATH, PHOTONOXIDE_LINKS, PHOTONOXIDE_OG_IMAGE, PHOTONOXIDE_PATH } from "@/lib/projects/photonoxide/meta";

const DESCRIPTION =
  "Each example reproduces a published result: it computes something with photonoxide, prints it beside the number the paper prints, and fails when they disagree. CI runs them all.";

export const metadata = projectsMetadata({
  title: "Examples",
  fullTitle: "Examples — photonoxide",
  description: DESCRIPTION,
  path: EXAMPLES_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Examples", path: EXAMPLES_PATH },
];

export default async function ExamplesPage() {
  const { ok, examples } = await getExamples();
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(CRUMBS)} />
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Examples</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Published results, reproduced</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
        <pre className="mt-5 w-fit rounded-xl bg-base-200 px-4 py-2.5 font-mono text-sm">
          cargo run --release --example &lt;name&gt;
        </pre>
      </header>

      {ok ? (
        <ul className="proj-rise-1 mt-10 grid gap-4 md:grid-cols-2">
          {examples.map((e) => (
            <li key={e.slug}>
              <Link
                href={`${EXAMPLES_PATH}/${e.slug}`}
                className="proj-card group flex h-full flex-col p-5 transition-colors hover:border-primary/40"
              >
                <code className="text-base-content/55 text-xs">{e.name}</code>
                <h2 className="mt-2 font-semibold text-lg tracking-tight group-hover:text-primary">{e.title}</h2>
                <p className="proj-lead mt-1.5 line-clamp-3 flex-1 text-sm">{e.summary}</p>
                <span className="mt-4 inline-flex items-center gap-1 text-primary text-xs">
                  Code and output
                  <ArrowRight size={13} aria-hidden />
                </span>
              </Link>
            </li>
          ))}
        </ul>
      ) : (
        <div className="mt-10">
          <SourceUnavailable href={PHOTONOXIDE_LINKS.examplesDir} linkLabel="Examples on GitHub">
            The examples are in the repository's examples folder.
          </SourceUnavailable>
        </div>
      )}
    </main>
  );
}
