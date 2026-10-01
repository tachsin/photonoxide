import { ArrowLeft, ArrowRight, BookOpen, Code2, ExternalLink, FlaskConical, PlayCircle } from "lucide-react";
import Link from "next/link";
import { notFound } from "next/navigation";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import ValidationRows from "@/components/projects/photonoxide/ValidationRows";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { repoLinkResolver, renderMathMarkdown } from "@/lib/projects/photonoxide/markdown";
import {
  EXAMPLES_PATH,
  METHODS_PATH,
  PHOTONOXIDE_LINKS,
  PHOTONOXIDE_OG_IMAGE,
  PHOTONOXIDE_PATH,
  VALIDATION_PATH,
} from "@/lib/projects/photonoxide/meta";
import { blobUrl, doiUrl, getMethod, getMethods } from "@/lib/projects/photonoxide/methods";
import { getReport } from "@/lib/projects/photonoxide/validation";

export async function generateMetadata({ params }) {
  const { slug } = await params;
  const method = await getMethod(slug);
  if (!method) return { title: { absolute: "Method — photonoxide" }, robots: { index: false, follow: true } };
  return projectsMetadata({
    title: method.title,
    fullTitle: `${method.title} — photonoxide`,
    description: method.summary,
    path: `${METHODS_PATH}/${method.slug}`,
    image: PHOTONOXIDE_OG_IMAGE,
    type: "article",
  });
}

const exampleSlug = (name) => name.replace(/_/g, "-");

export default async function MethodPage({ params }) {
  const { slug } = await params;
  const [{ ok, methods }, report] = await Promise.all([getMethods(), getReport()]);
  const index = methods.findIndex((m) => m.slug === slug);
  if (index === -1) {
    if (ok) notFound();
    return (
      <main className="proj-container pt-10 pb-8 sm:pt-14">
        <SourceUnavailable href={PHOTONOXIDE_LINKS.methodsDir} linkLabel="Methods on GitHub">
          This method is written up in the repository's docs/methods folder.
        </SourceUnavailable>
      </main>
    );
  }
  const method = methods[index];
  const [prev, next] = [methods[index - 1], methods[index + 1]];
  const path = `${METHODS_PATH}/${method.slug}`;
  const crumbs = [
    { name: "Projects", path: "/projects" },
    { name: "photonoxide", path: PHOTONOXIDE_PATH },
    { name: "Methods", path: METHODS_PATH },
    { name: method.title, path },
  ];
  const html = await renderMathMarkdown(method.body, { resolveUrl: repoLinkResolver("docs/methods") });
  const cases = method.validation
    .map((id) => report.cases.find((c) => c.id === id))
    .filter(Boolean);

  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(crumbs)} />
      <Breadcrumbs items={crumbs} />

      <header className="proj-rise max-w-3xl">
        <div className="flex flex-wrap items-center gap-2">
          <Link href={METHODS_PATH} className="proj-tag" data-tone="accent">
            Method {index + 1} of {methods.length}
          </Link>
          {method.module ? <code className="proj-tag">{method.module}</code> : null}
        </div>
        <h1 className="mt-4 font-semibold text-4xl tracking-tight sm:text-5xl">{method.title}</h1>
        {method.summary ? <p className="proj-lead mt-4 text-lg">{method.summary}</p> : null}

        <dl className="proj-card proj-facts mt-6">
          {method.papers.length ? (
            <div className="proj-fact">
              <dt>
                <BookOpen size={15} aria-hidden />
                {method.papers.length === 1 ? "Paper" : "Papers"}
              </dt>
              <dd>
                <ul className="space-y-1">
                  {method.papers.map((p) => (
                    <li key={p.cite}>
                      {p.doi ? (
                        <a
                          href={doiUrl(p.doi)}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="underline decoration-base-content/25 underline-offset-2 hover:decoration-primary"
                        >
                          {p.cite}
                          <ExternalLink size={12} aria-hidden className="ml-1 inline align-baseline opacity-60" />
                        </a>
                      ) : (
                        p.cite
                      )}
                    </li>
                  ))}
                </ul>
              </dd>
            </div>
          ) : null}
          {method.source ? (
            <div className="proj-fact">
              <dt>
                <Code2 size={15} aria-hidden />
                Source
              </dt>
              <dd>
                <a
                  href={blobUrl(method.source)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="font-mono text-sm underline decoration-base-content/25 underline-offset-2 hover:decoration-primary"
                >
                  {method.source}
                </a>
              </dd>
            </div>
          ) : null}
        </dl>
      </header>

      <article
        className="proj-prose proj-rise-1 mt-10 max-w-3xl"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: Markdown and KaTeX rendered on the server; raw HTML in the source is escaped
        dangerouslySetInnerHTML={{ __html: html }}
      />

      {cases.length ? (
        <section className="mt-12 max-w-5xl" aria-labelledby="validation">
          <h2 id="validation" className="flex items-center gap-2 font-semibold text-2xl tracking-tight">
            <FlaskConical size={20} aria-hidden />
            In the validation report
          </h2>
          <p className="proj-lead mt-2 text-sm">
            These rows are the{" "}
            <Link href={VALIDATION_PATH} className="link link-primary">
              report
            </Link>{" "}
            the library writes and CI checks on every change.
          </p>
          <ValidationRows cases={cases} />
        </section>
      ) : null}

      {method.examples.length ? (
        <section className="mt-12 max-w-3xl" aria-labelledby="examples">
          <h2 id="examples" className="flex items-center gap-2 font-semibold text-2xl tracking-tight">
            <PlayCircle size={20} aria-hidden />
            Examples
          </h2>
          <ul className="mt-4 flex flex-wrap gap-2">
            {method.examples.map((name) => (
              <li key={name}>
                <Link href={`${EXAMPLES_PATH}/${exampleSlug(name)}`} className="proj-pill font-mono">
                  {name}
                </Link>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <nav className="mt-16 flex flex-wrap justify-between gap-4 border-base-content/10 border-t pt-6" aria-label="Methods">
        {prev ? (
          <Link href={`${METHODS_PATH}/${prev.slug}`} className="btn btn-ghost btn-sm">
            <ArrowLeft size={15} aria-hidden />
            {prev.title}
          </Link>
        ) : (
          <span />
        )}
        {next ? (
          <Link href={`${METHODS_PATH}/${next.slug}`} className="btn btn-ghost btn-sm">
            {next.title}
            <ArrowRight size={15} aria-hidden />
          </Link>
        ) : null}
      </nav>
    </main>
  );
}
