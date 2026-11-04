// The paper engine's state in the window (papers.rs does the work): Home's feed, read from the
// cache at once and refreshed behind it; the paper open on the Papers page and the way back; the
// reading list. The feed's checks are silent: no toast, and after three failed checks in a row,
// one quiet line in the feed.

import { api } from "./api";
import { app, go, toast, updateSettings } from "./app.svelte";
import type { Feed, Paper, PaperSummary, Starred } from "./paperText";

/** Checks in a row that reached no source before the feed says so. */
export const QUIET_FAILURES = 3;

export const papers = $state({
  feed: null as Feed | null,
  /** A check running, and whether the user asked for it (Refresh shows a spinner; the background's shows nothing). */
  refreshing: false,
  asked: false,
  /** The paper open on the Papers page (`doi:...` or `arxiv:...`), and those opened before it, for Back. */
  open: null as string | null,
  trail: [] as string[],
  /** The Papers page's tab when no paper is open. */
  tab: "feed" as "feed" | "list" | "search",
  query: "",
  list: [] as Starred[],
  listLoaded: false,
});

/** The feed from the cache, once (again with `force`). */
export async function loadFeed(force = false) {
  if (papers.feed && !force) return;
  try {
    papers.feed = await api.papersFeed();
  } catch {
    // a cache that can't be read is an empty feed: the next check writes one
  }
}

/** Asks the sources (papers.rs); `asked` when the user pressed Refresh. */
export async function refreshFeed(asked = false) {
  if (papers.refreshing) return;
  papers.refreshing = true;
  papers.asked = asked;
  try {
    papers.feed = await api.papersRefresh();
  } catch (e) {
    if (asked) toast(`Couldn't check for new papers: ${e}`, "error");
  } finally {
    papers.refreshing = false;
    papers.asked = false;
  }
}

/** A check when one is due: on start, and every ten minutes while the window is open (papers.rs decides when one is due: four hours after the last, half an hour after a failed one). Only while the setting is on. Returns what stops it. */
export function startFeed(): () => void {
  const tick = async () => {
    await loadFeed(true);
    if (app.state?.settings.paper_feed && papers.feed?.due) await refreshFeed();
  };
  void tick();
  const t = setInterval(tick, 10 * 60_000);
  return () => clearInterval(t);
}

/** Turns the feed's own checks on or off (Settings); on, it checks at once if one is due. */
export async function setFeed(on: boolean) {
  await updateSettings((s) => (s.paper_feed = on));
  if (on && papers.feed?.due) void refreshFeed();
}

/** Opens a paper's page, by `doi:...`, `arxiv:...` or as pasted. */
export function openPaper(id: string) {
  if (papers.open && papers.open !== id && app.page === "papers") papers.trail.push(papers.open);
  else if (app.page !== "papers") papers.trail = [];
  papers.open = id;
  go("papers");
}

/** Opens the paper with DOI `doi`. */
export function openPaperLink(doi: string) {
  openPaper(`doi:${doi.toLowerCase()}`);
}

/** Back to the paper opened before, or to the lists. */
export function paperBack() {
  papers.open = papers.trail.pop() ?? null;
}

/** The Papers page on its lists: the feed, the reading list or a search. */
export function showPapers(tab: "feed" | "list" | "search" = "feed") {
  papers.open = null;
  papers.trail = [];
  papers.tab = tab;
  go("papers");
}

// ---- the reading list, kept in the workspace (papers/reading-list.json) ----

export async function loadList(force = false) {
  if (papers.listLoaded && !force) return;
  try {
    papers.list = await api.readingList();
  } catch {
    papers.list = [];
  }
  papers.listLoaded = true;
}

async function saveList() {
  try {
    await api.readingListSave($state.snapshot(papers.list) as Starred[]);
  } catch (e) {
    toast(`Couldn't save the reading list: ${e}`, "error");
  }
}

export function isStarred(id: string): boolean {
  return papers.list.some((s) => s.paper.id === id);
}

/** A summary as a paper, for the list (its abstract comes when its page is opened). */
export function paperOf(s: PaperSummary | Paper): Paper {
  return "abstract" in s ? s : { id: s.id, doi: s.doi, arxiv: s.arxiv, title: s.title, authors: s.authors, venue: s.venue, date: s.date, abstract: "", topics: s.topics };
}

/** Stars `paper`, or takes its star away. */
export async function toggleStar(paper: PaperSummary | Paper) {
  await loadList();
  const k = papers.list.findIndex((s) => s.paper.id === paper.id);
  if (k >= 0) papers.list.splice(k, 1);
  else papers.list.unshift({ paper: paperOf(paper), note: "", added: Math.floor(Date.now() / 1000) });
  await saveList();
}

/** A starred paper's fuller record (its abstract, all its authors) once its page is opened. */
export async function updateStarred(paper: Paper) {
  const s = papers.list.find((x) => x.paper.id === paper.id);
  if (!s || (s.paper.abstract && s.paper.authors.length >= paper.authors.length)) return;
  s.paper = paper;
  await saveList();
}

let noteTimer: ReturnType<typeof setTimeout> | undefined;
/** Sets a starred paper's note, saved a moment after the typing stops. */
export function setNote(id: string, note: string) {
  const s = papers.list.find((x) => x.paper.id === id);
  if (!s) return;
  s.note = note;
  clearTimeout(noteTimer);
  noteTimer = setTimeout(() => void saveList(), 600);
}
