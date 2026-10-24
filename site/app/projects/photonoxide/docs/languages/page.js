import { ExternalLink } from "lucide-react";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { blobUrl, getRepoFile } from "@/lib/projects/photonoxide/github";
import { renderMathMarkdown, repoLinkResolver } from "@/lib/projects/photonoxide/markdown";
import {
  DOCS_PATH,
  LANGUAGES_DOCS_PATH,
  PHOTONOXIDE_OG_IMAGE,
  PHOTONOXIDE_PATH,
} from "@/lib/projects/photonoxide/meta";

const DESCRIPTION =
  "photonoxide from any language: a job file in, the photonoxide program run headless, and the run's results.json and CSV tables read, with examples in the shell, R, JavaScript and Go.";

export const metadata = projectsMetadata({
  title: "Any language",
  fullTitle: "photonoxide from any language",
  description: DESCRIPTION,
  path: LANGUAGES_DOCS_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Docs", path: DOCS_PATH },
  { name: "Any language", path: LANGUAGES_DOCS_PATH },
];

const GUIDE = "docs/languages.md";
const RUNS = "docs/runs.md";

export default async function LanguagesDocsPage() {
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
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Any language</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
        <p className="mt-4 flex flex-wrap gap-2">
          <a href={blobUrl(RUNS)} target="_blank" rel="noopener noreferrer" className="proj-pill">
            A run&apos;s folder, field by field
            <ExternalLink size={12} aria-hidden />
          </a>
          <a href={blobUrl(GUIDE)} target="_blank" rel="noopener noreferrer" className="proj-pill">
            This guide on GitHub
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
          <SourceUnavailable href={blobUrl(GUIDE)} linkLabel="The guide on GitHub" />
        </div>
      )}
    </main>
  );
}
