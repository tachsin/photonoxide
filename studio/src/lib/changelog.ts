// The changelog's latest release, for Home's "What's new" card: its version, date and lines, as
// plain text (release-plz writes `- *(scope)* what ([#n](link))`); and every release, with its
// sections and pull requests, for the release notes. Plain functions, tested by
// tests/changelog.test.ts.

export interface ReleaseLine {
  /** The conventional commit's scope ("studio", "fabrication"), or "" when it has none. */
  scope: string;
  text: string;
}

export interface Release {
  version: string;
  date: string;
  lines: ReleaseLine[];
}

/** The first released version in `changelog` (not [Unreleased]), its lines in order; null when there is none. */
export function latestRelease(changelog: string): Release | null {
  const head = /^## \[(\d[^\]]*)\][^\n]*?(\d{4}-\d{2}-\d{2})?\s*$/m.exec(changelog);
  if (!head) return null;
  const rest = changelog.slice(head.index + head[0].length);
  const body = rest.split(/^## /m)[0];
  const lines = body
    .split("\n")
    .filter((l) => l.startsWith("- "))
    .map((l) => {
      const scope = /^- \*\(([^)]*)\)\*\s*/.exec(l);
      const text = l
        .slice(scope ? scope[0].length : 2)
        // the PR links at the end, then any link left as its text, then emphasis
        .replace(/(\s*\(\[#\d+\]\([^)]*\)\))+\s*$/, "")
        .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
        .replace(/[*`]/g, "")
        .trim();
      return { scope: scope?.[1] ?? "", text };
    });
  return { version: head[1], date: head[2] ?? "", lines };
}

/** A line of the release notes: its scope, its text (inline code kept in backticks) and its pull requests. */
export interface Entry extends ReleaseLine {
  prs: number[];
}

/** A heading of a release (Added, Fixed, …) and its lines. */
export interface Section {
  title: string;
  entries: Entry[];
}

/** A release in the notes: "Unreleased" for what is on main and not released yet. */
export interface Notes {
  version: string;
  date: string;
  sections: Section[];
  /** Any sentence under the release that isn't a line of a section. */
  text: string[];
}

function entry(line: string): Entry {
  const scope = /^- \*\(([^)]*)\)\*\s*/.exec(line);
  const rest = line.slice(scope ? scope[0].length : 2);
  // the PR links at the end, as release-plz writes them
  const tail = /(\s*\(\[#\d+\]\([^)]*\)\))+\s*$/.exec(rest);
  const prs = tail ? [...tail[0].matchAll(/\[#(\d+)\]/g)].map((m) => Number(m[1])) : [];
  const text = (tail ? rest.slice(0, tail.index) : rest)
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\*/g, "")
    .trim();
  return { scope: scope?.[1] ?? "", text, prs };
}

/** Every release in `changelog`, newest first; a release with nothing in it (an empty
 *  [Unreleased]) is left out. */
export function releases(changelog: string): Notes[] {
  return changelog
    .split(/^## /m)
    .slice(1)
    .map((part) => {
      const [head, ...body] = part.split("\n");
      const version = /^\[([^\]]+)\]/.exec(head)?.[1] ?? head.trim();
      const date = /(\d{4}-\d{2}-\d{2})\s*$/.exec(head)?.[1] ?? "";
      const sections: Section[] = [];
      const text: string[] = [];
      let last: Entry | null = null;
      for (const raw of body) {
        const line = raw.trimEnd();
        if (line.startsWith("### ")) {
          sections.push({ title: line.slice(4).replace(/<!--.*?-->/g, "").trim(), entries: [] });
          last = null;
        } else if (line.startsWith("- ")) {
          if (!sections.length) sections.push({ title: "", entries: [] });
          last = entry(line);
          sections[sections.length - 1].entries.push(last);
        } else if (/^\s+\S/.test(line) && last) {
          // a line wrapped under its bullet
          last.text = `${last.text} ${line.trim()}`;
        } else if (line.trim()) {
          text.push(line.trim());
        }
      }
      return { version, date, sections: sections.filter((s) => s.entries.length), text };
    })
    .filter((r) => r.sections.length || r.text.length);
}
