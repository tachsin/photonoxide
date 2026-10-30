import { ExternalLink } from "lucide-react";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { blobUrl, getRepoFile } from "@/lib/projects/photonoxide/github";
import { renderMathMarkdown, repoLinkResolver } from "@/lib/projects/photonoxide/markdown";
import {
  DESIGN_DOCS_PATH,
  DOCS_PATH,
  PHOTONOXIDE_OG_IMAGE,
  PHOTONOXIDE_PATH,
} from "@/lib/projects/photonoxide/meta";

const DESCRIPTION =
  "The designs people start from, a strip waveguide's modes, a ring's spectrum and a directional coupler, each as a job file beside the same calls in Python, with its layers from the top down, its materials and their indices, and what is solved and how.";

export const metadata = projectsMetadata({
  title: "Designs",
  fullTitle: "Designs as job files — photonoxide",
  description: DESCRIPTION,
  path: DESIGN_DOCS_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Docs", path: DOCS_PATH },
  { name: "Designs", path: DESIGN_DOCS_PATH },
];

const GUIDE = "docs/design.md";

export default async function DesignDocsPage() {
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
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Designs as job files</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
        <p className="mt-4 flex flex-wrap gap-2">
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
