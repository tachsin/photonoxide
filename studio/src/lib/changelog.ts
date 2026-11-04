// The changelog's latest release, for Home's "What's new" card: its version, date and lines, as
// plain text (release-plz writes `- *(scope)* what ([#n](link))`). Plain functions, tested by
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
        .replace(/[*_`]/g, "")
        .trim();
      return { scope: scope?.[1] ?? "", text };
    });
  return { version: head[1], date: head[2] ?? "", lines };
}
