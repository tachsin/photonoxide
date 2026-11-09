// The paper engine's state in the window (papers.rs does the work): Home's feed, read from the
// cache at once and refreshed behind it; the journal picker; the preprints' figures, asked one at
// a time for the rows on screen; the paper open on the Papers page and the way back; the reading
// list. The feed's checks are silent: no toast, and after three failed checks in a row, one quiet
// line in the feed.

import { convertFileSrc } from "@tauri-apps/api/core";

import { api } from "./api";
import { app, go, toast, updateSettings } from "./app.svelte";
import { withAll, withSource, type Feed, type Paper, type PaperSummary, type Starred } from "./paperText";

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

/** Asks the sources (papers.rs); `asked` when the user pressed Refresh; `onlyNew`, only the sources never asked. */
export async function refreshFeed(asked = false, onlyNew = false) {
  if (papers.refreshing) return;
  papers.refreshing = true;
  papers.asked = asked;
  try {
    papers.feed = await api.papersRefresh(onlyNew);
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

// ---- the journal picker: the settings keep the sources turned off (papers.rs reads them) ----

let newTimer: ReturnType<typeof setTimeout> | undefined;

/** Keeps the sources but `off` in the feed: shown at once from the cache; a source turned on that was never asked is asked alone, a moment after the last click (only while the feed checks by itself). */
async function setOff(off: string[]) {
  await updateSettings((s) => (s.paper_sources_off = off));
  await loadFeed(true);
  clearTimeout(newTimer);
  if (app.state?.settings.paper_feed && papers.feed?.missing) {
    newTimer = setTimeout(() => void refreshFeed(false, true), 1500);
  }
}

/** Turns one source of the feed on or off. */
export function setSourceOn(id: string, on: boolean) {
  return setOff(withSource(app.state?.settings.paper_sources_off ?? [], id, on));
}

/** Turns every source of the feed on, or off. */
export function setAllSources(on: boolean) {
  return setOff(withAll(papers.feed?.choices ?? [], on));
}

// ---- the preprints' figures (papers/images.rs), only while Settings' images are on ----

/** Each preprint's figure by paper id: its file's name, null for none, absent until asked. */
export const figures = $state<Record<string, string | null>>({});
/** The rows on screen that want a figure, in the order they came into view. */
const wanted = new Set<string>();
let fetching = false;
/** The switch's change on its way to papers.rs: the window shows it at once, and the asking waits for it. */
let saving: Promise<void> | null = null;

/** The URL a kept figure is shown by (the paperimg: scheme, studio.rs). */
export function figureUrl(name: string): string {
  return convertFileSrc(name, "paperimg");
}

/** A row with a preprint came on screen (`on`) or went off it: the figures are asked one at a time, for the rows still on screen. */
export function wantFigure(id: string, on: boolean) {
  if (!on) {
    wanted.delete(id);
    return;
  }
  if (id in figures || !app.state?.settings.paper_images) return;
  wanted.add(id);
  void askFigures();
}

async function askFigures() {
  if (fetching) return;
  fetching = true;
  try {
    if (saving) await saving;
    for (;;) {
      const id = wanted.values().next().value;
      if (id === undefined || !app.state?.settings.paper_images) break;
      wanted.delete(id);
      if (id in figures) continue;
      try {
        figures[id] = await api.paperImage(id);
      } catch {
        // offline or a server error: the initials stay, and it is asked again on the next start
        figures[id] = null;
      }
    }
  } finally {
    fetching = false;
  }
}

/** Turns the feed's figures on or off (Settings); off, none is asked or shown. */
export async function setFigures(on: boolean) {
  saving = updateSettings((s) => (s.paper_images = on));
  await saving;
  saving = null;
  if (!on) wanted.clear();
}

/** Removes the figures kept on disk, and forgets those shown. */
export async function clearFigures() {
  await api.paperImagesClear();
  for (const id of Object.keys(figures)) delete figures[id];
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
