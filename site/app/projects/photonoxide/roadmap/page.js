import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { getRepoFile } from "@/lib/projects/photonoxide/github";
import { renderMathMarkdown, repoLinkResolver } from "@/lib/projects/photonoxide/markdown";
import { PHOTONOXIDE_LINKS, PHOTONOXIDE_OG_IMAGE, PHOTONOXIDE_PATH, ROADMAP_PATH } from "@/lib/projects/photonoxide/meta";
import { getMilestones } from "@/lib/projects/photonoxide/roadmap";

const DESCRIPTION =
  "photonoxide's plan, milestone by milestone, from mode solvers to tape-out: what each one ships, how it is validated, and the papers behind it.";

export const metadata = projectsMetadata({
  title: "Roadmap",
  fullTitle: "Roadmap — photonoxide",
  description: DESCRIPTION,
  path: ROADMAP_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Roadmap", path: ROADMAP_PATH },
];

const STATUS_LABEL = { done: "Done", next: "Next", planned: "Planned" };

export default async function RoadmapPage() {
  const [markdown, { milestones }] = await Promise.all([getRepoFile("ROADMAP.md"), getMilestones()]);
  // the page has its own title: drop the document's
  const body = markdown ? markdown.replace(/^#\s+.*\r?\n/, "") : null;
  const html = body ? await renderMathMarkdown(body, { resolveUrl: repoLinkResolver("") }) : null;
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(CRUMBS)} />
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Roadmap</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Milestone by milestone</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
      </header>

      <ol className="proj-rise-1 mt-8 flex max-w-5xl flex-wrap gap-2">
        {milestones.map((m) => (
          <li key={m.version}>
            <a href={`#${m.version.replace(/\./g, "")}-${m.title.toLowerCase().replace(/[^\p{L}\p{N}\s-]/gu, "").trim().replace(/\s+/g, "-")}`} className="proj-pill" data-status={m.status}>
              <span className="font-mono text-xs">{m.version}</span>
              {m.title}
              <span className="proj-tag" data-tone={m.status === "planned" ? undefined : "accent"}>
                {STATUS_LABEL[m.status]}
                {m.total > 0 && m.status !== "done" ? ` · ${m.done}/${m.total}` : ""}
              </span>
            </a>
          </li>
        ))}
      </ol>

      {html ? (
        <article
          className="proj-prose mt-12 max-w-3xl"
          dangerouslySetInnerHTML={{ __html: html }}
        />
      ) : (
        <div className="mt-10">
          <SourceUnavailable href={PHOTONOXIDE_LINKS.roadmap} linkLabel="ROADMAP.md on GitHub" />
        </div>
      )}
    </main>
  );
}
