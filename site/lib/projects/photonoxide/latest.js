/**
 * The latest release's tag, kept in the server's memory for ten minutes, and which milestones a
 * release has done.
 *
 * Every page reads the repository at the tag of GitHub's latest release (github.js). The lookup
 * is one small API call, so it is not kept in Next's data cache: that cache lives on disk
 * (`.next/cache/fetch-cache`), outlives builds and restarts, and serves a stale answer to the
 * first visit after its time is up, so a release once waited for a day and two visits, deploy or
 * not (0.6.3, November 2026). Here the answer is kept in memory for ten minutes and asked again
 * with its ETag, which GitHub answers with a 304 that its rate limit doesn't count. A restart (a
 * deploy) starts empty, so the first page after it asks GitHub. When GitHub can't be reached the
 * last answer stays and is asked again after a minute; with none, the pages read the commit
 * they were synced from.
 *
 * Plain functions with no React or app imports, so the studio's tests run them
 * (`studio/tests/site-latest.test.ts`).
 */

/** How long an answer is kept: a release shows within this. */
export const LATEST_TTL_MS = 10 * 60 * 1000;

/** How long after a failed lookup the next one is tried. */
export const RETRY_MS = 60 * 1000;

/** A release tag the pages read: "v0.6.3". */
export function isReleaseTag(tag) {
  return typeof tag === "string" && /^v\d+\.\d+\.\d+$/.test(tag);
}

/**
 * @typedef {{ status: 200, tag: string | null, etag: string | null } | { status: 304 }} Answer
 *   what GitHub answered: the release's tag and the answer's ETag, or 304 for "unchanged"
 */

/**
 * A lookup of the latest release's tag, kept for `ttlMs`.
 * @param {object} options
 * @param {(etag: string | null) => Promise<Answer | null>} options.ask
 *   asks GitHub, sending the ETag when there is one; null when it can't be read
 * @param {() => number} [options.now]
 * @param {number} [options.ttlMs]
 * @param {number} [options.retryMs]
 * @returns {() => Promise<string | null>} the tag, e.g. "v0.6.3", or null when none is known
 */
export function latestTagLookup({ ask, now = Date.now, ttlMs = LATEST_TTL_MS, retryMs = RETRY_MS }) {
  let tag = null;
  let etag = null;
  let nextAt = 0;
  let pending = null;

  async function refresh() {
    let answer = null;
    try {
      answer = await ask(etag);
    } catch {
      answer = null;
    }
    if (answer?.status === 200) {
      tag = isReleaseTag(answer.tag) ? answer.tag : null;
      etag = tag ? answer.etag ?? null : null;
      nextAt = now() + ttlMs;
    } else if (answer?.status === 304 && tag) {
      nextAt = now() + ttlMs;
    } else {
      // GitHub can't be read (or a 304 with nothing kept): keep the last tag, try again soon
      if (answer?.status === 304) etag = null;
      nextAt = now() + retryMs;
    }
    return tag;
  }

  return async function latestTag() {
    if (now() < nextAt) return tag;
    pending ??= refresh().finally(() => {
      pending = null;
    });
    return pending;
  };
}

/**
 * A version's numbers, "0.6.3" -> [0, 6, 3], "0.3.x" -> [0, 3, Infinity]: an "x" milestone is
 * every patch of its minor.
 * @param {string} version
 */
function versionParts(version) {
  return String(version)
    .split(".")
    .map((p) => (p === "x" ? Number.POSITIVE_INFINITY : Number(p)));
}

/**
 * Whether a release has the milestone: its version is the release's or older ("0.6" is 0.6.0).
 * A released milestone is done whatever its heading and boxes say.
 * @param {string} milestone  e.g. "0.6.3", "0.6", "0.3.x"
 * @param {string | null} release  e.g. "0.6.3"; null when unknown
 */
export function isReleased(milestone, release) {
  if (!release) return false;
  const a = versionParts(milestone);
  const b = versionParts(release);
  if (a.some(Number.isNaN) || b.some(Number.isNaN)) return false;
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const x = a[i] ?? 0;
    const y = b[i] ?? 0;
    if (x !== y) return x < y;
  }
  return true;
}
