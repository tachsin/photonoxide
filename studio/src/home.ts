// The start page: run a job from jobs/ (or any job file), or reopen a run from runs/ (or any
// run folder).

export interface JobItem {
  path: string;
  name: string;
  kind: string;
  about: string;
}

export interface RunItem {
  dir: string;
  name: string;
  job: string;
  started: string;
  finished: boolean;
}

export interface Home {
  root: string;
  jobs: JobItem[];
  runs: RunItem[];
}

const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

/** "2026-10-02T09:17:32Z" as "2026-10-02 09:17 UTC". */
function when(started: string): string {
  const m = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(started);
  return m ? `${m[1]} ${m[2]} UTC` : started;
}

/** Fills `root` with the start page; `current` is the run the window has open, if any. */
export function renderHome(root: HTMLElement, h: Home, current: string | null) {
  const jobs = h.jobs.length
    ? h.jobs
        .map(
          (j) => `<div class="item">
            <div class="what"><strong>${esc(j.name)}</strong> <span class="tag">${esc(j.kind)}</span>
              ${j.about ? `<p class="weak">${esc(j.about)}</p>` : ""}</div>
            <button class="primary" data-run="${esc(j.path)}">Run</button></div>`,
        )
        .join("")
    : `<p class="weak">No job files in <code>jobs/</code> here.</p>`;
  const runs = h.runs.length
    ? h.runs
        .map(
          (r) => `<div class="item">
            <div class="what"><strong>${esc(r.job || r.name)}</strong>
              <span class="tag${r.finished ? "" : " unfinished"}">${r.finished ? "finished" : "unfinished"}</span>
              <p class="weak">${esc(when(r.started))} · ${esc(r.name)}</p></div>
            <button data-open="${esc(r.dir)}">Open</button></div>`,
        )
        .join("")
    : `<p class="weak">No runs in <code>runs/</code> yet.</p>`;
  root.innerHTML = `<div class="home">
      <h1>photonoxide studio</h1>
      <p class="weak">Working in <code>${esc(h.root)}</code>.
        ${current ? `<a href="#" data-goto="run">Back to ${esc(current)}</a>` : ""}</p>
      <div class="columns">
        <section class="panel"><h2>Run a job</h2>${jobs}
          <button class="pick" data-pick="job">Run a job file…</button></section>
        <section class="panel"><h2>Open a run</h2>${runs}
          <button class="pick" data-pick="run">Open a run folder…</button></section>
      </div>
    </div>`;
}
