import { renderMarkdown } from "@/lib/projects/highlight";
import { getChangelog } from "@/lib/projects/photonoxide/dependencies/report";

/**
 * The changes from our version of a dependency to its newest release, release by release,
 * newest first (lib/projects/photonoxide/dependencies/report.js): the notes of each release
 * with its day and a link to them, a release marked "breaking" when its notes say so or it is
 * a major one for us. A long note is cut, with a link to the rest. The page streams it in
 * (React Suspense), so the table never waits for it.
 * @param {{ row: import("@/lib/projects/photonoxide/dependencies/report").Row }} props
 */
export default async function DependencyChangelog({ row }) {
  let changelog;
  try {
    changelog = await getChangelog(row);
  } catch (e) {
    return (
      <p className="text-base-content/70 text-sm">
        Its repository's release notes couldn't be read ({String(e?.message ?? e)}).{" "}
        {row.notesUrl ? <NotesLink href={row.notesUrl}>Its release notes</NotesLink> : null}
      </p>
    );
  }
  if (!changelog.releases.length) {
    return (
      <p className="text-base-content/70 text-sm">
        {row.repo
          ? "No notes for these releases could be read from its repository (no GitHub release or CHANGELOG names them)."
          : "Its publisher's release notes are a page of their own, not readable as text here."}{" "}
        {changelog.fullUrl ? <NotesLink href={changelog.fullUrl}>{row.notesLabel ?? "The release notes"}</NotesLink> : null}
      </p>
    );
  }
  const rendered = await Promise.all(
    changelog.releases.map(async (r) => ({
      ...r,
      html:
        r.html ??
        (await renderMarkdown(r.markdown ?? "", {
          resolveUrl: (href) => {
            if (/^[a-z]+:|^#/i.test(href) || !changelog.base) return href;
            try {
              return new URL(href, changelog.base).href;
            } catch {
              return "#";
            }
          },
        })),
    })),
  );
  return (
    <div className="space-y-5">
      {rendered.map((r) => (
        <article key={r.version} className="border-base-content/10 border-l-2 pl-4">
          <header className="flex flex-wrap items-baseline gap-2">
            <h4 className="font-mono font-semibold text-sm">{r.version}</h4>
            {r.date ? <span className="text-base-content/60 text-xs">{r.date}</span> : null}
            {r.breaking ? (
              <span className="proj-tag" data-tone="accent">
                breaking
              </span>
            ) : null}
            <NotesLink href={r.url ?? changelog.fullUrl}>full notes</NotesLink>
          </header>
          {r.html ? (
            <div className="proj-prose mt-2 max-w-none text-sm" dangerouslySetInnerHTML={{ __html: r.html }} />
          ) : (
            <p className="mt-2 text-base-content/60 text-sm">No notes.</p>
          )}
          {r.cut ? (
            <p className="mt-1 text-xs">
              Cut short: <NotesLink href={r.url ?? changelog.fullUrl}>the full notes</NotesLink>.
            </p>
          ) : null}
        </article>
      ))}
      <p className="text-base-content/60 text-xs">
        From {changelog.from === "feed" ? "its GitHub releases" : "its CHANGELOG"}
        {changelog.more ? "; older releases in between aren't shown" : ""}:{" "}
        <NotesLink href={changelog.fullUrl}>every release's notes</NotesLink>.
      </p>
    </div>
  );
}

function NotesLink({ href, children }) {
  if (!href) return null;
  return (
    <a href={href} target="_blank" rel="noopener noreferrer" className="link link-primary text-xs">
      {children}
    </a>
  );
}
