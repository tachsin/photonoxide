/**
 * Reads the registries politely, for the Dependencies page: each URL at most once an hour,
 * a few at a time, crates.io's API no more than once a second, and conditionally (the
 * answer's ETag or Last-Modified sent back, so an unchanged answer is a short 304, which
 * GitHub's API doesn't count against its limit). Plain JavaScript, no imports: the studio's
 * tests run it with a recorded fetch.
 *
 * What it keeps is in the server's memory, the parsed answer of each URL (not its body: some
 * answers are megabytes), and when it was read:
 *
 * - **Within the hour,** the kept answer, with no request.
 * - **After it,** the kept answer at once, and a new read behind it, for the next visit
 *   (stale while revalidating); with nothing kept, the page waits for the read.
 * - **A read that fails** (a timeout, an error status, an answer that doesn't parse) keeps the
 *   last answer and says when it failed; it is tried again after ten minutes.
 *
 * The app's other pages go through Next's data cache (`fetchCached`), daily. This page reads
 * hourly, sends conditional requests, and keeps the last answer through a failure, which
 * Next's cache doesn't do for an answer it never stored (over 2 MB, as some registries' are).
 * A restart of the server starts it empty: the first visit after one waits for every
 * registry, a few seconds.
 */

export const HOUR = 3600_000;
const RETRY_AFTER_FAILURE = 600_000;

/**
 * @typedef {{ value: any, readAt: number | null, failedAt: number | null, error: string | null }} Read
 *   `value` is the parsed answer last read (undefined when none was), `readAt` when it was
 *   read (or confirmed unchanged), `failedAt` and `error` the last failure since.
 */

/**
 * @param {{
 *   fetch?: typeof fetch,
 *   now?: () => number,
 *   userAgent: string,
 *   store?: Map<string, any>,
 *   concurrency?: number,
 *   timeoutMs?: number,
 *   spacing?: Record<string, number>,
 *   background?: boolean,
 * }} options  `spacing` is the least time between two requests to a host, in ms;
 *   `background: false` makes a stale read wait for its refresh (the tests' choice)
 */
export function createReader({
  fetch: fetchImpl = globalThis.fetch,
  now = Date.now,
  userAgent,
  store = new Map(),
  concurrency = 6,
  timeoutMs = 10_000,
  spacing = {},
  background = true,
}) {
  const inflight = new Map();
  const lastStart = new Map();
  let running = 0;
  const queue = [];

  const acquire = () =>
    new Promise((resolve) => {
      if (running < concurrency) {
        running++;
        resolve();
      } else queue.push(resolve);
    });
  const release = () => {
    const next = queue.shift();
    if (next) next();
    else running--;
  };

  /** Waits until the host's spacing since its last request has passed. */
  async function space(host) {
    const gap = spacing[host];
    if (!gap) return;
    const at = Math.max(now(), (lastStart.get(host) ?? 0) + gap);
    lastStart.set(host, at);
    const wait = at - now();
    if (wait > 0) await new Promise((r) => setTimeout(r, wait));
  }

  const snapshot = (e) => ({
    value: e?.value,
    readAt: e?.readAt ?? null,
    failedAt: e?.failedAt ?? null,
    error: e?.error ?? null,
  });

  async function refresh(url, { parse, headers, missing }) {
    const e = store.get(url) ?? {};
    const conditional = {};
    if (e.etag) conditional["If-None-Match"] = e.etag;
    else if (e.lastModified) conditional["If-Modified-Since"] = e.lastModified;
    await acquire();
    try {
      await space(new URL(url).host);
      const res = await fetchImpl(url, {
        headers: { "User-Agent": userAgent, ...headers, ...(e.value !== undefined ? conditional : {}) },
        cache: "no-store",
        signal: AbortSignal.timeout(timeoutMs),
      });
      const at = now();
      if (res.status === 304 && e.value !== undefined) {
        store.set(url, { ...e, readAt: at, checkedAt: at, failedAt: null, error: null });
      } else if (res.ok || (res.status === 404 && missing)) {
        const value = res.ok ? parse(await res.text()) : null;
        if (value === undefined) throw new Error("the answer couldn't be read");
        store.set(url, {
          value,
          etag: res.ok ? res.headers.get("etag") : null,
          lastModified: res.ok ? res.headers.get("last-modified") : null,
          readAt: at,
          checkedAt: at,
          failedAt: null,
          error: null,
        });
      } else {
        throw new Error(`HTTP ${res.status}`);
      }
    } catch (err) {
      const at = now();
      const message = err?.name === "TimeoutError" ? "no answer in time" : String(err?.message ?? err);
      store.set(url, { ...e, checkedAt: at, failedAt: at, error: message });
    } finally {
      release();
    }
  }

  function start(url, options) {
    if (!inflight.has(url)) {
      inflight.set(
        url,
        refresh(url, options).finally(() => inflight.delete(url)),
      );
    }
    return inflight.get(url);
  }

  /**
   * A URL's parsed answer. `parse(text)` returns the value to keep, or undefined when the
   * answer doesn't read; `missing: true` keeps a 404 as `null` (a file that isn't there).
   * @param {string} url
   * @param {{ parse?: (text: string) => any, headers?: Record<string, string>, ttl?: number, missing?: boolean }} [options]
   * @returns {Promise<Read>}
   */
  async function read(url, { parse = (t) => t, headers = {}, ttl = HOUR, missing = false } = {}) {
    const e = store.get(url);
    const fresh = e && now() - e.checkedAt < (e.failedAt ? Math.min(ttl, RETRY_AFTER_FAILURE) : ttl);
    if (fresh) return snapshot(e);
    const pending = start(url, { parse, headers, missing });
    if (e?.value !== undefined && background) {
      pending.catch(() => {});
      return snapshot(e);
    }
    await pending;
    return snapshot(store.get(url));
  }

  /**
   * A computed value, kept like an answer: `compute()` again after `ttl`, the last value
   * served meanwhile. A failed computation isn't kept.
   * @template T
   * @param {string} key
   * @param {number} ttl
   * @param {() => Promise<T>} compute
   * @returns {Promise<T>}
   */
  async function memo(key, ttl, compute) {
    const k = `memo:${key}`;
    const e = store.get(k);
    if (e && now() - e.at < ttl) return e.value;
    if (!inflight.has(k)) {
      inflight.set(
        k,
        compute()
          .then((value) => {
            store.set(k, { value, at: now() });
            return value;
          })
          .finally(() => inflight.delete(k)),
      );
    }
    const pending = inflight.get(k);
    if (e && background) {
      pending.catch(() => {});
      return e.value;
    }
    return pending;
  }

  /** Waits for every read and computation under way (the tests'). */
  async function idle() {
    while (inflight.size) await Promise.allSettled([...inflight.values()]);
  }

  return { read, memo, idle };
}
