// When the studio looks for updates by itself, and how the last look reads. Plain functions, no
// state of the window's, so `pnpm test` (node --test, tests/schedule.test.ts) runs them as they are.

/** A check when the window comes back waits this long after the last one. */
export const FOCUS_GAP = 15 * 60_000;
/** And one every hour while the window stays open. */
export const HOURLY = 60 * 60_000;

/** Why a check runs: the window opening, coming back (focus, or shown again), or the hourly timer. */
export type Why = "open" | "focus" | "hourly";

/** How a check ended: nothing new, something found, or it couldn't check (a failure, GitHub's limit, a backoff). */
export type Outcome = "current" | "found" | "error";

export interface LastCheck {
  /** When it ended, in ms since 1970. */
  at: number;
  outcome: Outcome;
  /** What was found ("photonoxide 0.5.5 is out"), or why it couldn't check. */
  detail: string;
}

/** A check is due when the window comes back: the last one, or the last one started, is 15 minutes old or more. */
export function dueOnFocus(now: number, checkedAt: number, started = 0): boolean {
  return now - Math.max(checkedAt, started) >= FOCUS_GAP;
}

/** `14:02` today, `Oct 30, 14:02` before. */
export function clock(at: number, now: number): string {
  const d = new Date(at);
  const time = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  if (d.toDateString() === new Date(now).toDateString()) return time;
  const month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][d.getMonth()];
  return `${month} ${d.getDate()}, ${time}`;
}

/** "Last checked 14:02: up to date", and so on; the line Settings shows beside the switch. */
export function describe(last: LastCheck | null, now: number, enabled = true): string {
  if (!last) return enabled ? "Not checked yet since photonoxide opened." : "Automatic checks are off; Check now looks once.";
  const what =
    last.outcome === "current"
      ? `up to date${last.detail ? ` (${last.detail})` : ""}`
      : last.outcome === "found"
        ? last.detail
        : `could not check (${last.detail})`;
  return `Last checked ${clock(last.at, now)}: ${what}`;
}

export interface Scheduler {
  /** Whether the settings ask for automatic checks, read each time. */
  enabled: () => boolean;
  /** When the last check ended (ms since 1970; 0: none yet). */
  checkedAt: () => number;
  /** Starts a check; it doesn't wait for it. */
  check: (why: Why) => void;
  now: () => number;
  /** Runs `run` every `ms`; returns what stops it. */
  every: (run: () => void, ms: number) => () => void;
  /** Runs `run` when the window comes back; returns what stops it. */
  onFocus: (run: () => void) => () => void;
}

/**
 * Checks at once (the window has just opened), when the window comes back 15 minutes or more
 * after the last check, and every hour; each only while the settings ask for it. Returns what
 * stops it all.
 */
export function startSchedule(s: Scheduler): () => void {
  let started = 0;
  const run = (why: Why) => {
    if (!s.enabled()) return;
    if (why === "focus" && !dueOnFocus(s.now(), s.checkedAt(), started)) return;
    started = s.now();
    s.check(why);
  };
  run("open");
  const stopHourly = s.every(() => run("hourly"), HOURLY);
  const stopFocus = s.onFocus(() => run("focus"));
  return () => {
    stopHourly();
    stopFocus();
  };
}
