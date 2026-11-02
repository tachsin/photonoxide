import { Suspense } from "react";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import DependencyChangelog from "@/components/projects/photonoxide/DependencyChangelog";
import { NewMarker, RememberVersions } from "@/components/projects/photonoxide/NewSinceLastVisit";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { getDependencies } from "@/lib/projects/photonoxide/dependencies/report";
import { DEPENDENCIES_PATH, PHOTONOXIDE_LINKS, PHOTONOXIDE_OG_IMAGE, PHOTONOXIDE_PATH } from "@/lib/projects/photonoxide/meta";

const DESCRIPTION =
  "Every library and dependency photonoxide uses, the version we use beside its newest release: the native libraries its backends load, the Rust crates, the studio's pnpm packages, the Python requirements, the GitHub Actions and the toolchain.";

export const metadata = projectsMetadata({
  title: "Dependencies",
  fullTitle: "Dependencies — photonoxide",
  description: DESCRIPTION,
  path: DEPENDENCIES_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Dependencies", path: DEPENDENCIES_PATH },
];

/** What each group's "we use" is, and where its newest release is read. */
const ABOUT = {
  native:
    "The external libraries photonoxide-native loads, at the newest version their install guides were checked with (docs/libraries.md), against every place each publisher releases it. A release isn't supported until it has been checked.",
  crates:
    "Every workspace crate's direct dependencies, dev and build ones too, at Cargo.lock's versions, against crates.io's index.",
  pnpm: "The studio's package.json, at pnpm-lock.yaml's versions, against the npm registry.",
  python: "python/pyproject.toml's requirements as written, against PyPI: a requirement is behind when it doesn't take the newest release.",
  actions:
    "The workflows' actions at their tags, against each action's latest release: a tag such as v7 takes every 7.x.y.",
  toolchain: "The Rust, pnpm and Node.js versions the repository names.",
};

/** A time as `2026-11-02 14:05 UTC`. */
function when(ms) {
  if (!ms) return "never";
  return `${new Date(ms).toISOString().slice(0, 16).replace("T", " ")} UTC`;
}

function RowStatus({ row }) {
  if (row.failedAt) {
    return (
      <span className="text-warning text-xs" title={row.error ?? undefined}>
        couldn't be read at {when(row.failedAt)}
        {row.readAt ? `; shown as read at ${when(row.readAt)}` : ""}
      </span>
    );
  }
  return <span className="text-base-content/55 text-xs">{when(row.readAt)}</span>;
}

function Newest({ row }) {
  if (!row.newest) {
    return <span className="text-base-content/55 text-xs">couldn't be read</span>;
  }
  return (
    <div className="flex flex-col items-start gap-1">
      <span className={`font-mono text-xs ${row.newer ? "font-semibold" : ""}`}>
        {row.page ? (
          <a href={row.page} target="_blank" rel="noopener noreferrer" className="link link-hover">
            {row.newest}
          </a>
        ) : (
          row.newest
        )}
      </span>
      {row.newestDate || row.newestLabel ? (
        <span className="text-base-content/55 text-[0.7rem]">{[row.newestDate, row.newestLabel].filter(Boolean).join(" · ")}</span>
      ) : null}
      <span className="flex flex-wrap gap-1">
        {row.major ? (
          <a href={row.notesUrl ?? row.page ?? "#"} target="_blank" rel="noopener noreferrer" className="proj-tag" data-tone="accent">
            major · release notes
          </a>
        ) : row.newer ? (
          <span className="proj-tag" data-tone="accent">
            newer
          </span>
        ) : null}
        <NewMarker id={row.key} version={row.newest} />
      </span>
    </div>
  );
}

function GroupTable({ group }) {
  return (
    <div className="mt-4 overflow-x-auto rounded-2xl border border-base-content/10">
      <table className="table table-sm">
        <thead>
          <tr>
            <th>Name</th>
            <th>Source</th>
            <th>We use</th>
            <th>Newest</th>
            <th>Last read</th>
          </tr>
        </thead>
        <tbody>
          {group.rows.map((row) => (
            <RowLines key={row.key} row={row} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function RowLines({ row }) {
  return (
    <>
      <tr className="align-top">
        <td className="min-w-[10rem]">
          <span className="font-mono text-xs">{row.name}</span>
          {row.note ? <p className="mt-1 max-w-xs text-base-content/55 text-[0.7rem]">{row.note}</p> : null}
        </td>
        <td className="text-xs">
          {row.sources ? (
            row.sources.map((s, i) => (
              <span key={s.name}>
                {i ? ", " : ""}
                <a href={s.page} target="_blank" rel="noopener noreferrer" className="link link-hover">
                  {s.name}
                </a>
              </span>
            ))
          ) : (
            row.source
          )}
        </td>
        <td className="whitespace-nowrap font-mono text-xs">{row.usesLabel ?? row.uses ?? "—"}</td>
        <td>
          <Newest row={row} />
        </td>
        <td>
          <RowStatus row={row} />
        </td>
      </tr>
      {row.newer ? (
        <tr>
          <td colSpan={5} className="pt-0">
            <details className="group">
              <summary className="cursor-pointer text-base-content/70 text-xs hover:text-base-content">
                What changed from {row.usesLabel ?? row.uses} to {row.newest}
              </summary>
              <div className="mt-3 mb-2">
                <Suspense fallback={<p className="text-base-content/55 text-xs">Reading the release notes…</p>}>
                  <DependencyChangelog row={row} />
                </Suspense>
              </div>
            </details>
          </td>
        </tr>
      ) : null}
    </>
  );
}

export default async function DependenciesPage() {
  const { groups } = await getDependencies();
  const rows = groups.flatMap((g) => g.rows);
  const newer = rows.filter((r) => r.newer);
  const majors = newer.filter((r) => r.major);
  const failed = rows.filter((r) => r.failedAt);
  const read = rows.map((r) => r.readAt).filter(Boolean);
  const versions = Object.fromEntries(rows.filter((r) => r.newest).map((r) => [r.key, r.newest]));
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(CRUMBS)} />
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Dependencies</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Every version we use</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
        <p className="proj-lead mt-3 text-sm">
          Our versions are read from the repository's <code>main</code> branch, and each newest release from its
          registry, at most once an hour. A pre-release is never marked newer. A major release, one our requirement
          doesn't take (for a 0.x crate or package, a new 0.y), is marked with its release notes, and every newer one
          lists what changed in between. A registry that can't be read keeps the last version read, and its row says
          when it failed. Direct dependencies only: theirs are Cargo's, pnpm's and pip's to resolve.
        </p>
      </header>

      <dl className="proj-rise-1 mt-8 grid max-w-3xl grid-cols-2 gap-px overflow-hidden rounded-2xl border border-base-content/10 bg-base-content/10 sm:grid-cols-4">
        <div className="bg-base-100 p-4">
          <dt className="text-base-content/60 text-xs">Dependencies</dt>
          <dd className="mt-1 font-semibold text-2xl">{rows.length}</dd>
        </div>
        <div className="bg-base-100 p-4">
          <dt className="text-base-content/60 text-xs">With a newer release</dt>
          <dd className="mt-1 font-semibold text-2xl">{newer.length}</dd>
        </div>
        <div className="bg-base-100 p-4">
          <dt className="text-base-content/60 text-xs">Major ones</dt>
          <dd className="mt-1 font-semibold text-2xl">{majors.length}</dd>
        </div>
        <div className="bg-base-100 p-4">
          <dt className="text-base-content/60 text-xs">Read</dt>
          <dd className="mt-1 font-semibold text-sm">{read.length ? when(Math.min(...read)) : "not yet"}</dd>
          {failed.length ? <dd className="mt-1 text-warning text-xs">{failed.length} couldn't be read since</dd> : null}
        </div>
      </dl>

      {groups.map((group) => (
        <section key={group.id} className="mt-12" aria-labelledby={`group-${group.id}`}>
          <h2 id={`group-${group.id}`} className="font-semibold text-2xl tracking-tight">
            {group.title}
          </h2>
          <p className="mt-2 max-w-3xl text-base-content/70 text-sm">{ABOUT[group.id]}</p>
          {group.rows.length ? (
            <GroupTable group={group} />
          ) : (
            <div className="mt-4">
              <SourceUnavailable href={PHOTONOXIDE_LINKS.github} linkLabel="The repository on GitHub">
                The files this group is read from couldn't be read{group.filesError ? ` (${group.filesError})` : ""}.
              </SourceUnavailable>
            </div>
          )}
        </section>
      ))}
      <p className="mt-12 max-w-3xl text-base-content/60 text-xs">
        What to do with a native library's new release, before photonoxide supports it:{" "}
        <a href={`${PHOTONOXIDE_LINKS.libraries}#new-releases`} target="_blank" rel="noopener noreferrer" className="link">
          docs/libraries.md
        </a>
        .
      </p>
      <RememberVersions versions={versions} />
    </main>
  );
}
