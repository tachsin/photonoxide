import { BookOpen } from "lucide-react";
import Link from "next/link";
import { notFound } from "next/navigation";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import LangCodeGroup from "@/components/projects/LangCodeGroup";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { getExample, getExamples, runCommand } from "@/lib/projects/photonoxide/examples";
import { blobUrl } from "@/lib/projects/photonoxide/github";
import { renderMathMarkdown, repoLinkResolver } from "@/lib/projects/photonoxide/markdown";
import {
  EXAMPLES_PATH,
  METHODS_PATH,
  PHOTONOXIDE_LINKS,
  PHOTONOXIDE_OG_IMAGE,
  PHOTONOXIDE_PATH,
} from "@/lib/projects/photonoxide/meta";
import { getMethods } from "@/lib/projects/photonoxide/methods";
import { absoluteUrl } from "@/lib/tools-metadata";

export async function generateMetadata({ params }) {
  const { slug } = await params;
  const example = await getExample(slug);
  if (!example) return { title: { absolute: "Example — photonoxide" }, robots: { index: false, follow: true } };
  return projectsMetadata({
    title: example.title,
    fullTitle: `${example.title} — photonoxide example`,
    description: example.summary || `The photonoxide example ${example.name}.`,
    path: `${EXAMPLES_PATH}/${example.slug}`,
    image: PHOTONOXIDE_OG_IMAGE,
    type: "article",
  });
}

export default async function ExamplePage({ params }) {
  const { slug } = await params;
  const example = await getExample(slug);
  if (!example) {
    const { ok } = await getExamples();
    if (ok) notFound();
    return (
      <main className="proj-container pt-10 pb-8 sm:pt-14">
        <SourceUnavailable href={PHOTONOXIDE_LINKS.examplesDir} linkLabel="Examples on GitHub">
          This example is in the repository's examples folder.
        </SourceUnavailable>
      </main>
    );
  }
  const { methods } = await getMethods();
  const usedBy = methods.filter((m) => m.examples.includes(example.name));
  const path = `${EXAMPLES_PATH}/${example.slug}`;
  const crumbs = [
    { name: "Projects", path: "/projects" },
    { name: "photonoxide", path: PHOTONOXIDE_PATH },
    { name: "Examples", path: EXAMPLES_PATH },
    { name: example.title, path },
  ];
  const html = example.doc.trim()
    ? await renderMathMarkdown(example.doc, { resolveUrl: repoLinkResolver("examples") })
    : "";
  const structuredData = {
    "@graph": [
      {
        "@type": "SoftwareSourceCode",
        name: `${example.title} — photonoxide example`,
        description: example.summary,
        url: absoluteUrl(path),
        codeRepository: PHOTONOXIDE_LINKS.github,
        codeSampleType: "full solution",
        programmingLanguage: [{ "@type": "ComputerLanguage", name: "Rust" }],
        isPartOf: { "@id": `${absoluteUrl(PHOTONOXIDE_PATH)}#source` },
      },
      breadcrumbList(crumbs),
    ],
  };

  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={structuredData} />
      <Breadcrumbs items={crumbs} />

      <header className="proj-rise max-w-3xl">
        <div className="flex flex-wrap items-center gap-2">
          <Link href={EXAMPLES_PATH} className="proj-tag" data-tone="accent">
            Example
          </Link>
          <code className="proj-tag">{example.name}</code>
        </div>
        <h1 className="mt-4 font-semibold text-4xl tracking-tight sm:text-5xl">{example.title}</h1>
      </header>

      {html ? (
        <article
          className="proj-prose proj-rise-1 mt-8 max-w-3xl"
          dangerouslySetInnerHTML={{ __html: html }}
        />
      ) : null}

      {usedBy.length ? (
        <p className="mt-6 flex flex-wrap items-center gap-2 text-sm">
          <BookOpen size={15} aria-hidden className="text-base-content/55" />
          <span className="text-base-content/65">Methods:</span>
          {usedBy.map((m) => (
            <Link key={m.slug} href={`${METHODS_PATH}/${m.slug}`} className="proj-pill">
              {m.title}
            </Link>
          ))}
        </p>
      ) : null}

      <section className="mt-10 max-w-4xl space-y-6">
        <LangCodeGroup
          id="example-code"
          panels={[{ lang: "rust", code: example.code, filename: example.path, href: blobUrl(example.path) }]}
        />
        {example.output ? (
          <LangCodeGroup
            id="example-output"
            className="[&_pre]:max-h-[32rem]"
            panels={[
              {
                lang: "rust",
                syntax: "text",
                code: example.output,
                filename: `$ ${runCommand(example)}`,
                href: blobUrl(`examples/output/${example.name}.txt`),
              },
            ]}
          />
        ) : null}
      </section>
    </main>
  );
}
