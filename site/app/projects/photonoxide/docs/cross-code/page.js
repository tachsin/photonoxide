import { ExternalLink } from "lucide-react";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { blobUrl, getRepoFile } from "@/lib/projects/photonoxide/github";
import { renderMathMarkdown, repoLinkResolver } from "@/lib/projects/photonoxide/markdown";
import {
  CROSS_CODE_DOCS_PATH,
  DOCS_PATH,
  PHOTONOXIDE_OG_IMAGE,
  PHOTONOXIDE_PATH,
} from "@/lib/projects/photonoxide/meta";

const DESCRIPTION =
  "photonoxide's solvers, each checked against an established code that solves the same physics: against Elmer for heat and electrostatics (accuracy and speed), against openEMS for the RF line (accuracy).";

export const metadata = projectsMetadata({
  title: "Cross-code comparison",
  fullTitle: "photonoxide: cross-code comparison",
  description: DESCRIPTION,
  path: CROSS_CODE_DOCS_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Docs", path: DOCS_PATH },
  { name: "Cross-code comparison", path: CROSS_CODE_DOCS_PATH },
];

const GUIDE = "docs/cross-code.md";
const HARNESS = "crosscode/README.md";

export default async function CrossCodeDocsPage() {
  const markdown = await getRepoFile(GUIDE);
  // the page has its own title: drop the document's
  const body = markdown ? markdown.replace(/^#\s+.*\r?\n/, "") : null;
  const html = body ? await renderMathMarkdown(body, { resolveUrl: repoLinkResolver("docs") }) : null;
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(CRUMBS)} />
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Docs</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Cross-code comparison</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
        <p className="mt-4 flex flex-wrap gap-2">
          <a href={blobUrl(HARNESS)} target="_blank" rel="noopener noreferrer" className="proj-pill">
            Running it yourself
            <ExternalLink size={12} aria-hidden />
          </a>
          <a href={blobUrl(GUIDE)} target="_blank" rel="noopener noreferrer" className="proj-pill">
            This page on GitHub
            <ExternalLink size={12} aria-hidden />
          </a>
        </p>
      </header>
      {html ? (
        <article
          className="proj-prose proj-rise-1 mt-10 max-w-3xl"
          dangerouslySetInnerHTML={{ __html: html }}
        />
      ) : (
        <div className="mt-10">
          <SourceUnavailable href={blobUrl(GUIDE)} linkLabel="The page on GitHub" />
        </div>
      )}
    </main>
  );
}
