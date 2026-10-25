import { ExternalLink } from "lucide-react";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { blobUrl, getRepoFile, rawUrl } from "@/lib/projects/photonoxide/github";
import { renderMathMarkdown, repoLinkResolver } from "@/lib/projects/photonoxide/markdown";
import {
  CROSS_CODE_DOCS_PATH,
  DOCS_PATH,
  PHOTONOXIDE_OG_IMAGE,
  PHOTONOXIDE_PATH,
} from "@/lib/projects/photonoxide/meta";

const DESCRIPTION =
  "photonoxide against other codes on the same cases: openEMS (3D FDTD) and Elmer (finite elements) on a coplanar line and a heater, each code's error in percent against photonoxide's converged result and the time it took, on one chart.";

export const metadata = projectsMetadata({
  title: "Against other codes",
  fullTitle: "photonoxide against other codes",
  description: DESCRIPTION,
  path: CROSS_CODE_DOCS_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Docs", path: DOCS_PATH },
  { name: "Against other codes", path: CROSS_CODE_DOCS_PATH },
];

const GUIDE = "docs/cross-code.md";
const HARNESS = "crosscode/README.md";
const CHART = "docs/cross-code-chart.svg";
const CHART_DARK = "docs/cross-code-chart-dark.svg";
const CHART_ALT =
  "Each code's error against photonoxide's converged result, in percent, against its own solve time, both on log scales: photonoxide and Elmer at the bottom left, openEMS at the right.";

export default async function CrossCodeDocsPage() {
  const markdown = await getRepoFile(GUIDE);
  // the page has its own title, and draws the chart itself: drop the document's
  const body = markdown
    ? markdown.replace(/^#\s+.*\r?\n/, "").replace(/<picture>[\s\S]*?<\/picture>\s*/, "")
    : null;
  const html = body ? await renderMathMarkdown(body, { resolveUrl: repoLinkResolver("docs") }) : null;
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(CRUMBS)} />
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Docs</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Against other codes</h1>
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
      <figure className="proj-rise-1 mt-10 max-w-5xl">
        <picture>
          <source media="(prefers-color-scheme: dark)" srcSet={rawUrl(CHART_DARK)} />
          <img src={rawUrl(CHART)} alt={CHART_ALT} className="w-full h-auto" />
        </picture>
      </figure>
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
