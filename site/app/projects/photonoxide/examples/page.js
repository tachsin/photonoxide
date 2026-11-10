import "katex/dist/katex.min.css";
import { ArrowRight } from "lucide-react";
import Link from "next/link";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import rich from "@/components/projects/photonoxide/RichText.module.css";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { getExamples } from "@/lib/projects/photonoxide/examples";
import { plainText } from "@/lib/projects/photonoxide/inline";
import { renderInline, repoLinkResolver } from "@/lib/projects/photonoxide/markdown";
import { EXAMPLES_PATH, PHOTONOXIDE_LINKS, PHOTONOXIDE_OG_IMAGE, PHOTONOXIDE_PATH } from "@/lib/projects/photonoxide/meta";

const DESCRIPTION =
  "Each example computes a published result and prints it beside the number the paper prints, with the grid and the tolerance; it exits with code 1 when it is outside that tolerance. CI runs them all.";

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

const resolveExampleUrl = (href) => repoLinkResolver("examples")(href, "link");

/**
 * An example's card. The whole card opens the example (its heading's link
 * covers it), and the links in its summary, such as the paper's DOI, sit
 * above that and open themselves.
 */
function ExampleCard({ example: e }) {
  const href = `${EXAMPLES_PATH}/${e.slug}`;
  return (
    <li className="proj-card group relative flex h-full flex-col p-5 transition-colors focus-within:border-primary/50 hover:border-primary/40">
      <code className="text-base-content/55 text-xs">{e.name}</code>
      <h2 className="mt-2 font-semibold text-lg leading-snug tracking-tight">
        <Link
          href={href}
          title={plainText(e.title)}
          className="outline-none after:absolute after:inset-0 after:rounded-[inherit] after:content-[''] group-hover:text-primary focus-visible:after:outline-2 focus-visible:after:outline-primary focus-visible:after:outline-offset-2"
        >
          <span
            className={`${rich.inline} line-clamp-3`}
            dangerouslySetInnerHTML={{ __html: renderInline(e.head, { links: false }) }}
          />
        </Link>
      </h2>
      {e.subtitle ? (
        <p
          className={`${rich.inline} mt-1 line-clamp-2 text-base-content/60 text-sm leading-snug`}
          dangerouslySetInnerHTML={{ __html: renderInline(e.subtitle, { links: false }) }}
        />
      ) : null}
      <p
        className={`${rich.inline} proj-lead mt-2 line-clamp-3 flex-1 text-sm [&_a]:relative [&_a]:z-10`}
        dangerouslySetInnerHTML={{ __html: renderInline(e.summary, { refs: true, resolveUrl: resolveExampleUrl }) }}
      />
      <span className="pointer-events-none mt-4 inline-flex items-center gap-1 text-primary text-xs">
        Code and output
        <ArrowRight size={13} aria-hidden />
      </span>
    </li>
  );
}

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
        <pre className="mt-5 w-fit max-w-full overflow-x-auto rounded-xl bg-base-200 px-4 py-2.5 font-mono text-sm">
          cargo run --release --example &lt;name&gt;
        </pre>
      </header>

      {ok ? (
        <ul className="proj-rise-1 mt-10 grid gap-4 md:grid-cols-2">
          {examples.map((e) => (
            <ExampleCard key={e.slug} example={e} />
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
