import "katex/dist/katex.min.css";
import { Check, ChevronRight, Link2 } from "lucide-react";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import rich from "@/components/projects/photonoxide/RichText.module.css";
import RoadmapControls from "@/components/projects/photonoxide/RoadmapControls";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { renderInline, renderMathMarkdown, repoLinkResolver } from "@/lib/projects/photonoxide/markdown";
import { PHOTONOXIDE_LINKS, PHOTONOXIDE_OG_IMAGE, PHOTONOXIDE_PATH, ROADMAP_PATH } from "@/lib/projects/photonoxide/meta";
import { milestoneAnchor } from "@/lib/projects/photonoxide/milestones";
import { getMilestones, getRoadmap } from "@/lib/projects/photonoxide/roadmap";

const DESCRIPTION =
  "photonoxide's plan, milestone by milestone, from mode solvers to layout export: what each one ships, how it is validated, and the papers behind it.";

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

/** The section whose `### ` blocks are the milestones. */
const MILESTONES = "Milestones";

/** ROADMAP.md's Markdown as HTML: math, issue chips, short DOI links, its notes set apart. */
const render = (markdown) =>
  renderMathMarkdown(markdown, { resolveUrl: repoLinkResolver(""), refs: true, compact: true, notes: true });

/** A section's Markdown again, its `### ` blocks under their headings. */
const sectionMarkdown = (s) => [s.intro, ...s.blocks.map((b) => `### ${b.heading}\n\n${b.body}`)].filter(Boolean).join("\n\n");

function Marker({ status }) {
  const base =
    "absolute top-3 left-0 z-10 flex size-7 items-center justify-center rounded-full ring-4 ring-base-100";
  if (status === "done") {
    return (
      <span aria-hidden className={`${base} bg-primary/15 text-primary`}>
        <Check size={14} strokeWidth={3} />
      </span>
    );
  }
  if (status === "next") {
    return (
      <span aria-hidden className={`${base} bg-primary text-primary-content shadow-[0_0_0_7px_color-mix(in_oklab,var(--color-primary)_18%,transparent)]`}>
        <span className="size-2.5 rounded-full bg-primary-content" />
      </span>
    );
  }
  if (status === "planned") {
    return (
      <span aria-hidden className={`${base} border border-base-content/20 bg-base-100`}>
        <span className="size-2 rounded-full bg-base-content/25" />
      </span>
    );
  }
  // a block between milestones ("Throughout", the decisions)
  return (
    <span aria-hidden className={`${base} bg-base-100`}>
      <span className="size-2.5 rotate-45 rounded-[2px] border border-base-content/35" />
    </span>
  );
}

const SUMMARY =
  "flex cursor-pointer list-none items-start gap-3 rounded-xl px-4 py-3 outline-offset-2 hover:bg-base-content/[0.03] focus-visible:outline-2 focus-visible:outline-primary sm:px-5 [&::-webkit-details-marker]:hidden";

/** One milestone, or one block between them, on the timeline. */
function TimelineItem({ block, html, last }) {
  const m = block.milestone;
  const status = m?.status ?? null;
  const open = status === "next";
  const counted = m && m.status !== "done" && m.total > 0;
  return (
    <li className="relative pb-4 pl-11">
      <span
        aria-hidden
        className={`absolute left-[0.8125rem] w-0.5 rounded-full bg-base-content/10 ${last ? "top-3 h-4" : "top-3 bottom-0"}`}
      />
      <Marker status={status} />
      <details
        id={block.anchor}
        data-roadmap=""
        open={open}
        className={`group scroll-mt-32 rounded-xl border transition-colors ${
          status === "next"
            ? "border-primary/35 bg-primary/[0.04]"
            : "border-base-content/10 bg-base-100/60 open:border-base-content/15"
        }`}
      >
        <summary className={SUMMARY}>
          <ChevronRight
            size={16}
            aria-hidden
            className="mt-1 shrink-0 text-base-content/40 transition-transform group-open:rotate-90"
          />
          <span className="min-w-0 flex-1">
            <h3 className="flex flex-wrap items-baseline gap-x-2.5 gap-y-0.5 text-base leading-snug">
              {m ? <span className="font-mono text-base-content/55 text-sm">{m.version}</span> : null}
              <span
                className={`font-semibold tracking-tight ${status === "planned" ? "text-base-content/80" : ""} ${m ? "" : "text-base-content/75"}`}
                dangerouslySetInnerHTML={{ __html: renderInline(m ? m.title : block.heading, { links: false }) }}
              />
            </h3>
            {counted ? (
              <span className="mt-2 flex items-center gap-2.5 text-base-content/60 text-xs">
                <progress
                  className={`progress h-1.5 w-24 sm:w-32 ${status === "next" ? "progress-primary" : ""}`}
                  value={m.done}
                  max={m.total}
                  aria-label={`${m.done} of ${m.total} items done`}
                />
                {m.done} of {m.total} done
              </span>
            ) : null}
          </span>
          {m ? (
            <span className="proj-tag mt-0.5 shrink-0" data-tone={status === "planned" ? undefined : "accent"}>
              {STATUS_LABEL[status]}
            </span>
          ) : null}
        </summary>
        {html ? (
          <div className="border-base-content/10 border-t px-4 pt-4 pb-3 sm:px-6">
            <div
              className={`proj-prose ${rich.prose} text-[0.95rem]`}
              dangerouslySetInnerHTML={{ __html: html }}
            />
            <a
              href={`#${block.anchor}`}
              className="mt-1 inline-flex items-center gap-1 text-base-content/45 text-xs hover:text-primary"
            >
              <Link2 size={12} aria-hidden />
              Link to {m ? m.version : block.heading}
            </a>
          </div>
        ) : null}
      </details>
    </li>
  );
}

/** A section of ROADMAP.md other than the milestones, as one collapsible card. */
function SectionCard({ section, html, open }) {
  return (
    <details
      id={section.anchor}
      data-roadmap=""
      open={open}
      className="group scroll-mt-32 rounded-2xl border border-base-content/10 bg-base-100/60 open:border-base-content/15"
    >
      <summary className={SUMMARY}>
        <ChevronRight
          size={18}
          aria-hidden
          className="mt-1.5 shrink-0 text-base-content/40 transition-transform group-open:rotate-90"
        />
        <h2 className="flex-1 font-semibold text-xl tracking-tight">{section.heading}</h2>
      </summary>
      <div className="border-base-content/10 border-t px-4 pt-4 pb-2 sm:px-6">
        <div
          className={`proj-prose ${rich.prose}`}
          dangerouslySetInnerHTML={{ __html: html }}
        />
      </div>
    </details>
  );
}

/** Where the plan stands: milestones done, and the next one. */
function AtAGlance({ milestones }) {
  const done = milestones.filter((m) => m.status === "done").length;
  const next = milestones.find((m) => m.status === "next");
  return (
    <div className="proj-card proj-rise-1 mt-8 grid max-w-4xl gap-5 p-5 sm:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] sm:p-6">
      <div>
        <p className="proj-eyebrow">Done</p>
        <p className="mt-1.5 font-semibold text-2xl tracking-tight">
          {done} <span className="font-normal text-base text-base-content/60">of {milestones.length} milestones</span>
        </p>
        <progress
          className="progress progress-primary mt-3 h-2 w-full"
          value={done}
          max={milestones.length}
          aria-label={`${done} of ${milestones.length} milestones done`}
        />
      </div>
      {next ? (
        <div>
          <p className="proj-eyebrow">Next</p>
          <a
            href={`#${milestoneAnchor(next)}`}
            className="mt-1.5 block font-semibold text-lg leading-snug tracking-tight hover:text-primary"
          >
            <span className="mr-2 font-mono text-base-content/55 text-sm">{next.version}</span>
            {next.title}
          </a>
          {next.total > 0 ? (
            <p className="mt-1.5 text-base-content/60 text-sm">
              {next.done} of {next.total} items done
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

export default async function RoadmapPage() {
  const [roadmap, { milestones }] = await Promise.all([getRoadmap(), getMilestones()]);
  const sections = roadmap?.sections ?? [];
  const milestonesAt = sections.findIndex((s) => s.heading === MILESTONES);
  // the sections before the timeline: all of them, if ever there were no "Milestones"
  const before = milestonesAt >= 0 ? milestonesAt : sections.length;
  const preface = roadmap?.preface ? await render(roadmap.preface) : "";
  const rendered = await Promise.all(
    sections.map(async (s, i) =>
      i === milestonesAt
        ? { intro: s.intro ? await render(s.intro) : "", blocks: await Promise.all(s.blocks.map((b) => (b.body ? render(b.body) : ""))) }
        : { html: await render(sectionMarkdown(s)) },
    ),
  );
  // without ROADMAP.md, the milestones' titles alone (the fallback), as a timeline still
  const timeline =
    milestonesAt >= 0
      ? sections[milestonesAt].blocks.map((block, j) => ({ block, html: rendered[milestonesAt].blocks[j] }))
      : milestones.map((m) => ({ block: { heading: `${m.version}: ${m.title}`, anchor: milestoneAnchor(m), milestone: m }, html: "" }));

  const timelineSection = (
    <section key="milestones" aria-labelledby="milestones" className="mt-12">
      <div className="flex max-w-4xl flex-wrap items-end justify-between gap-3">
        <div>
          <p className="proj-eyebrow">Timeline</p>
          <h2 id="milestones" className="mt-1.5 scroll-mt-32 font-semibold text-2xl tracking-tight sm:text-3xl">
            {MILESTONES}
          </h2>
        </div>
        <RoadmapControls />
      </div>
      {milestonesAt >= 0 && rendered[milestonesAt].intro ? (
        <div
          className={`proj-prose ${rich.prose} mt-4 max-w-3xl`}
          dangerouslySetInnerHTML={{ __html: rendered[milestonesAt].intro }}
        />
      ) : null}
      <ol className="mt-6 max-w-4xl">
        {timeline.map(({ block, html }, j) => (
          <TimelineItem key={block.anchor} block={block} html={html} last={j === timeline.length - 1} />
        ))}
      </ol>
    </section>
  );

  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(CRUMBS)} />
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Roadmap</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Milestone by milestone</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
        <p className="mt-3 text-base-content/55 text-sm">
          As written in{" "}
          <a href={PHOTONOXIDE_LINKS.roadmap} className="underline decoration-base-content/25 underline-offset-2 hover:text-primary">
            ROADMAP.md
          </a>
          , in parts that open and close; the numbered chips are issues and pull requests on GitHub.
        </p>
      </header>

      <AtAGlance milestones={milestones} />

      {preface ? (
        <div
          className={`proj-prose ${rich.prose} mt-10 max-w-3xl`}
          dangerouslySetInnerHTML={{ __html: preface }}
        />
      ) : null}

      {roadmap ? (
        <div className="mt-10 max-w-4xl space-y-3">
          {sections.slice(0, before).map((s, i) => (
            <SectionCard key={s.anchor} section={s} html={rendered[i].html} open={i === 0} />
          ))}
        </div>
      ) : (
        <div className="mt-10">
          <SourceUnavailable href={PHOTONOXIDE_LINKS.roadmap} linkLabel="ROADMAP.md on GitHub" />
        </div>
      )}

      {timelineSection}

      {roadmap && milestonesAt >= 0 ? (
        <div className="mt-10 max-w-4xl space-y-3">
          {sections.slice(milestonesAt + 1).map((s, k) => (
            <SectionCard key={s.anchor} section={s} html={rendered[milestonesAt + 1 + k].html} open={false} />
          ))}
        </div>
      ) : null}
    </main>
  );
}
