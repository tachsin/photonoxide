import Link from "next/link";
import {
  ArrowRight,
  Atom,
  BookOpen,
  PlayCircle,
  CircuitBoard,
  Factory,
  FlaskConical,
  Layers,
  MonitorPlay,
  Sparkles,
  Waves,
  Wrench,
} from "lucide-react";
import { SiGithub, SiRust } from "react-icons/si";
import JsonLd from "@/components/projects/JsonLd";
import HeroLight from "@/components/projects/photonoxide/HeroLight";
import { breadcrumbList, WEBSITE_ID, website } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { getExamples } from "@/lib/projects/photonoxide/examples";
import { getMethods } from "@/lib/projects/photonoxide/methods";
import { getReport } from "@/lib/projects/photonoxide/validation";
import {
  DOCS_PATH,
  EXAMPLES_PATH,
  METHODS_PATH,
  PHOTONOXIDE_DESCRIPTION,
  PHOTONOXIDE_HIGHLIGHTS,
  PHOTONOXIDE_KEYWORDS,
  PHOTONOXIDE_LICENSE,
  PHOTONOXIDE_LINKS,
  PHOTONOXIDE_OG_IMAGE,
  PHOTONOXIDE_PATH,
  PHOTONOXIDE_PIPELINE,
  PHOTONOXIDE_PROPERTIES,
  PHOTONOXIDE_TAGLINE,
  PHOTONOXIDE_VALIDATION,
  ROADMAP_PATH,
  VALIDATION_PATH,
} from "@/lib/projects/photonoxide/meta";
import { getMilestones } from "@/lib/projects/photonoxide/roadmap";
import { absoluteUrl } from "@/lib/tools-metadata";

export const metadata = projectsMetadata({
  title: "photonoxide",
  fullTitle: `photonoxide — ${PHOTONOXIDE_TAGLINE}`,
  description: PHOTONOXIDE_DESCRIPTION,
  path: PHOTONOXIDE_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
  imageAlt: `photonoxide — ${PHOTONOXIDE_TAGLINE}`,
  keywords: PHOTONOXIDE_KEYWORDS,
});

const HIGHLIGHT_ICONS = {
  modes: Atom,
  fields: Waves,
  layers: Layers,
  inverse: Sparkles,
  layout: CircuitBoard,
  tapeout: Factory,
  studio: MonitorPlay,
  rust: SiRust,
};

const HERO_LINKS = [
  { label: "GitHub", href: PHOTONOXIDE_LINKS.github, Icon: SiGithub },
  { label: "crates.io", href: PHOTONOXIDE_LINKS.crates, Icon: SiRust },
  { label: "docs.rs", href: PHOTONOXIDE_LINKS.docsRs, Icon: BookOpen },
];

// this site's pages first, then GitHub's
const RESOURCES = [
  { label: "Getting started", href: DOCS_PATH, note: "a first slab and strip waveguide" },
  { label: "Methods", href: METHODS_PATH, note: "equations, papers, validation, limits" },
  { label: "Examples", href: EXAMPLES_PATH, note: "published results, reproduced and checked" },
  { label: "Validation report", href: VALIDATION_PATH, note: "every case, its tolerance and grid" },
  { label: "Roadmap", href: ROADMAP_PATH, note: "every milestone, with its references" },
  { label: "Pitfalls ruled out by design", href: PHOTONOXIDE_LINKS.pitfalls, note: "each one measured, each with a test" },
  { label: "Ideas and use cases", href: PHOTONOXIDE_LINKS.issues, note: "GitHub issues" },
  { label: "Source code", href: PHOTONOXIDE_LINKS.github, note: "github.com/tachsin/photonoxide" },
];

const STATUS_LABEL = { done: "Done", next: "Next", planned: "Planned" };

function structuredData() {
  const url = absoluteUrl(PHOTONOXIDE_PATH);
  return {
    "@graph": [
      website(),
      {
        "@type": "SoftwareSourceCode",
        "@id": `${url}#source`,
        name: "photonoxide",
        description: PHOTONOXIDE_DESCRIPTION,
        url,
        codeRepository: PHOTONOXIDE_LINKS.github,
        programmingLanguage: [{ "@type": "ComputerLanguage", name: "Rust" }],
        runtimePlatform: ["Rust"],
        license: ["https://opensource.org/licenses/MIT", "https://www.apache.org/licenses/LICENSE-2.0"],
        keywords: PHOTONOXIDE_KEYWORDS.join(", "),
        author: { "@type": "Person", name: "tachsin", url: "https://github.com/tachsin" },
        creativeWorkStatus: "Alpha",
        isPartOf: { "@id": WEBSITE_ID },
      },
      breadcrumbList([
        { name: "Projects", path: "/projects" },
        { name: "photonoxide", path: PHOTONOXIDE_PATH },
      ]),
    ],
  };
}

function SectionHeading({ eyebrow, title, children, id }) {
  return (
    <div className="max-w-2xl">
      {eyebrow ? <p className="proj-eyebrow">{eyebrow}</p> : null}
      <h2 id={id} className="mt-2 scroll-mt-32 font-semibold text-3xl tracking-tight">
        {title}
      </h2>
      {children ? <p className="proj-lead mt-3">{children}</p> : null}
    </div>
  );
}

export default async function PhotonoxidePage() {
  const [{ ok: roadmapOk, milestones }, { methods }, { examples }, report] = await Promise.all([
    getMilestones(),
    getMethods(),
    getExamples(),
    getReport(),
  ]);
  const passing = report.cases.filter((c) => c.passed).length;
  const now = milestones.find((m) => m.status === "next");
  const today = [
    {
      Icon: BookOpen,
      href: METHODS_PATH,
      count: methods.length || null,
      label: "methods",
      body: "From exact slabs and transfer matrices to full-vector modes with a PML, each with its paper and its limits.",
    },
    {
      Icon: PlayCircle,
      href: EXAMPLES_PATH,
      count: examples.length || null,
      label: "examples",
      body: "Each reproduces a published result, prints it beside the paper's numbers, and fails when they disagree.",
    },
    {
      Icon: FlaskConical,
      href: VALIDATION_PATH,
      count: report.cases.length ? `${passing}/${report.cases.length}` : null,
      label: "validation cases passing",
      body: "Analytic solutions and published results, each with its tolerance and grid, checked by CI on every change.",
    },
  ];

  return (
    <main>
      <JsonLd data={structuredData()} />

      {/* ---------- Hero ---------- */}
      <section className="proj-container relative isolate pt-16 pb-12 text-center sm:pt-24">
        <div className="proj-rise flex justify-center">
          <Link href={ROADMAP_PATH} className="proj-pill">
            <span className="size-1.5 rounded-full bg-warning" aria-hidden />
            <span>Alpha · {now ? `${now.version}: ${now.title.toLowerCase()} in progress` : "built in the open"}</span>
            <ArrowRight size={13} aria-hidden />
          </Link>
        </div>

        <h1 className="proj-rise proj-gradient-text mt-6 font-mono font-semibold text-6xl tracking-tighter sm:text-8xl">
          photonoxide
        </h1>
        <p className="proj-rise-1 mx-auto mt-5 max-w-2xl text-balance text-base-content/80 text-lg sm:text-xl">
          Photonics for Rust: mode solvers, FDFD, FDTD, inverse design, layout and tape-out in one library, with a
          studio to watch every simulation and optimization live.
        </p>
        <p className="proj-rise-1 mx-auto mt-3 max-w-xl text-balance text-base-content/60 text-sm">
          Validated against analytic solutions and published devices, and fabricable: designs leave as files a
          foundry accepts.
        </p>

        <div className="proj-rise-2 mt-9 flex flex-wrap justify-center gap-3">
          <Link href={DOCS_PATH} className="btn btn-primary">
            Get started
            <ArrowRight size={16} aria-hidden />
          </Link>
          <Link href={METHODS_PATH} className="btn btn-ghost border-base-content/15">
            How it computes
          </Link>
        </div>

        <ul className="proj-rise-2 mt-8 flex flex-wrap justify-center gap-2">
          {HERO_LINKS.map(({ label, href, Icon }) => (
            <li key={label}>
              <a href={href} target="_blank" rel="noopener noreferrer" className="proj-pill">
                <Icon size={14} aria-hidden />
                {label}
              </a>
            </li>
          ))}
          <li>
            <span className="proj-pill">{PHOTONOXIDE_LICENSE}</span>
          </li>
        </ul>

        <HeroLight />
      </section>

      {/* ---------- What works today ---------- */}
      <section className="proj-container py-16" aria-labelledby="today">
        <SectionHeading id="today" eyebrow="What works today" title="Validated, one method at a time">
          0.1, the foundations, is on crates.io; the mode solvers of 0.2 are on the main branch. Nothing ships without
          an analytic test, a published result it reproduces, and a measured convergence order.
        </SectionHeading>
        <ul className="mt-10 grid gap-4 md:grid-cols-3">
          {today.map(({ Icon, href, count, label, body }) => (
            <li key={label}>
              <Link href={href} className="proj-card group flex h-full flex-col p-6 transition-colors hover:border-primary/40">
                <span className="proj-icon-tile">
                  <Icon size={19} aria-hidden />
                </span>
                <p className="mt-4 font-semibold text-3xl tracking-tight">
                  {count ?? "—"} <span className="font-normal text-base text-base-content/60">{label}</span>
                </p>
                <p className="proj-lead mt-2 flex-1 text-sm">{body}</p>
                <span className="mt-4 inline-flex items-center gap-1 text-primary text-sm">
                  Open
                  <ArrowRight size={14} aria-hidden />
                </span>
              </Link>
            </li>
          ))}
        </ul>
      </section>

      {/* ---------- Highlights ---------- */}
      <section className="proj-container py-16" aria-labelledby="highlights">
        <SectionHeading id="highlights" eyebrow="What it will be" title="From Maxwell's equations to a chip">
          Solvers, inverse design, layout and tape-out in one Rust library, built milestone by milestone.
        </SectionHeading>
        <ul className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {PHOTONOXIDE_HIGHLIGHTS.map((h) => {
            const Icon = HIGHLIGHT_ICONS[h.icon] ?? Sparkles;
            return (
              <li key={h.title} className="proj-card p-5">
                <span className="proj-icon-tile">
                  <Icon size={19} aria-hidden />
                </span>
                <h3 className="mt-4 font-semibold tracking-tight">{h.title}</h3>
                <p className="proj-lead mt-1.5 text-sm">{h.body}</p>
              </li>
            );
          })}
        </ul>

        <dl className="mt-6 grid gap-px overflow-hidden rounded-2xl border border-base-content/10 bg-base-content/10 sm:grid-cols-2 lg:grid-cols-4">
          {PHOTONOXIDE_PROPERTIES.map((p) => (
            <div key={p.title} className="bg-base-100 p-5">
              <dt className="font-semibold text-sm">{p.title}</dt>
              <dd className="proj-lead mt-1 text-sm">{p.body}</dd>
            </div>
          ))}
        </dl>
      </section>

      {/* ---------- Pipeline ---------- */}
      <section className="proj-container py-16" aria-labelledby="pipeline">
        <SectionHeading id="pipeline" eyebrow="The path" title="From a simulation to a measured chip">
          Fabrication is a constraint from the first step, not a step at the end. The first target is SiEPIC
          openEBL: 220 nm silicon-on-insulator, electron-beam lithography, submitted as a GitHub pull request and
          measured remotely.
        </SectionHeading>
        <ol className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {PHOTONOXIDE_PIPELINE.map((step, i) => (
            <li key={step.title} className="proj-card flex gap-4 p-5">
              <span className="proj-tag h-fit" data-tone="accent">
                {i + 1}
              </span>
              <div>
                <h3 className="font-semibold tracking-tight">{step.title}</h3>
                <p className="proj-lead mt-1 text-sm">{step.body}</p>
              </div>
            </li>
          ))}
        </ol>
        <p className="proj-lead mt-6 text-sm">
          The optimizers come from{" "}
          <a href={PHOTONOXIDE_LINKS.genoxide} className="link link-primary">
            genoxide
          </a>
          : photonoxide supplies the physics, the gradients and the fabrication constraints.
        </p>
      </section>

      {/* ---------- Roadmap ---------- */}
      <section className="proj-container py-16" aria-labelledby="roadmap">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <SectionHeading id="roadmap" eyebrow="Roadmap" title="Milestone by milestone">
            Every milestone ships with its validation: an analytic test, a published result and a convergence
            study.
          </SectionHeading>
          <Link href={ROADMAP_PATH} className="btn btn-sm btn-ghost border-base-content/15">
            Full roadmap
            <ArrowRight size={15} aria-hidden />
          </Link>
        </div>
        <ol className="mt-8 grid gap-3 md:grid-cols-2">
          {milestones.map((m) => (
            <li key={m.version} className="proj-card p-5" data-status={m.status}>
              <div className="flex items-baseline justify-between gap-3">
                <h3 className="font-semibold tracking-tight">
                  <span className="mr-2 font-mono text-base-content/50 text-sm">{m.version}</span>
                  {m.title}
                </h3>
                <span className="proj-tag shrink-0" data-tone={m.status === "planned" ? undefined : "accent"}>
                  {STATUS_LABEL[m.status]}
                  {m.total > 0 && m.status !== "done" ? ` · ${m.done}/${m.total}` : ""}
                </span>
              </div>
              {m.items.length ? (
                <p className="proj-lead mt-2 text-sm">{m.items.join(" · ")}</p>
              ) : null}
            </li>
          ))}
        </ol>
        {roadmapOk ? null : (
          <p className="proj-lead mt-4 text-sm">
            The roadmap couldn't be read from GitHub just now, so only the milestones' titles are shown.
          </p>
        )}
      </section>

      {/* ---------- Validation + resources ---------- */}
      <section className="proj-container grid gap-6 py-16 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)]">
        <div className="proj-card flex flex-col p-7">
          <span className="proj-icon-tile">
            <FlaskConical size={19} aria-hidden />
          </span>
          <h2 className="mt-5 font-semibold text-2xl tracking-tight">Validation</h2>
          <p className="proj-lead mt-2">Every solver on three tiers, before it ships:</p>
          <ul className="mt-4 space-y-3">
            {PHOTONOXIDE_VALIDATION.map((v) => (
              <li key={v.title}>
                <p className="font-medium text-sm">{v.title}</p>
                <p className="proj-lead text-sm">{v.body}</p>
              </li>
            ))}
          </ul>
        </div>

        <div className="proj-card p-7">
          <span className="proj-icon-tile">
            <Wrench size={19} aria-hidden />
          </span>
          <h2 className="mt-5 font-semibold text-2xl tracking-tight">Resources</h2>
          <ul className="mt-5 grid gap-x-8 gap-y-1 sm:grid-cols-2">
            {RESOURCES.map((r) => (
              <li key={r.label}>
                <a
                  href={r.href}
                  {...(r.href.startsWith("/") ? {} : { target: "_blank", rel: "noopener noreferrer" })}
                  className="-mx-2 flex flex-col rounded-lg px-2 py-2 transition-colors hover:bg-base-content/5"
                >
                  <span className="font-medium text-sm">{r.label}</span>
                  <span className="text-base-content/55 text-xs">{r.note}</span>
                </a>
              </li>
            ))}
          </ul>
        </div>
      </section>
    </main>
  );
}
