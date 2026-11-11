// The Academy (studio/src-tauri/src/academy.rs and charts.rs): its lessons, tracks and paths,
// loaded once, and what the page remembers between visits: the lesson open, the path followed,
// and the sections folded.

import { api } from "./api";
import type { Raster } from "./events";

export type Level = "introductory" | "intermediate" | "advanced";
export type Role = "origin" | "milestone" | "review" | "current";
/** How deep a section goes: intuition is always open, theory and research on demand. */
export type Depth = "intuition" | "theory" | "research";

export interface Paper {
  cite: string;
  title: string;
  doi: string;
  year: number;
  role: Role;
  note: string;
}

export type LessonBlock =
  | { kind: "text"; markdown: string }
  | { kind: "diagram"; diagram: string }
  | { kind: "chart"; chart: string; values: Record<string, number> }
  | { kind: "example"; name: string }
  | { kind: "validation"; cases: string[] }
  | { kind: "timeline" }
  | { kind: "answer"; markdown: string }
  | { kind: "device"; device: string }
  /** How a library device is made: its flow played step by step, from `job` (TOML), as the Fabrication page plays it. */
  | { kind: "fabrication"; device: string; label: string; parameters: Record<string, string>; job: string }
  /** Lithography machines in 2D and 3D, animated (components/fabrication/LithoTool.svelte): the first shown, the rest to compare. */
  | { kind: "lithography"; tools: string[] };

/** A section still to be written: what it will answer, and the examples it will use. */
export interface Coming {
  /** One paragraph, Markdown with TeX math. */
  answers: string;
  examples: string[];
}

export interface LessonSection {
  title: string;
  /** Its anchor, from the title. */
  id: string;
  depth: Depth;
  /** 2 for a `##` section, 3 for a part of one (a `###` heading with a depth). */
  level: 2 | 3;
  /** Set when the section is still to be written; its blocks are then none. */
  coming: Coming | null;
  blocks: LessonBlock[];
}

/** Written, or laid out with its subsections and still to be written. */
export type Status = "published" | "coming";

/** A validation case's row in the published report (docs/validation.md). */
export interface CaseRow {
  id: string;
  tier: string;
  what: string;
  against: string;
  measured: string;
  expected: string;
  tolerance: string;
  pass: boolean;
}

export interface Lesson {
  /** Its file name without .md, e.g. "ring-resonator". */
  id: string;
  title: string;
  summary: string;
  topic: string;
  level: Level;
  status: Status;
  /** For a lesson coming soon, the milestone whose solvers it waits for (ROADMAP.md), e.g. "0.6". */
  milestone: string | null;
  /** About how long it takes to read; 0 for a lesson coming soon. */
  minutes: number;
  prerequisites: string[];
  examples: string[];
  jobs: string[];
  circuits: string[];
  methods: string[];
  validation: string[];
  charts: string[];
  papers: Paper[];
  sections: LessonSection[];
  cases: CaseRow[];
}

export interface ChartParam {
  key: string;
  /** May hold TeX between dollars. */
  label: string;
  /** As a chart block writes it: "um", "nm", "dB/cm", "dB", or "" for a number. */
  unit: string;
  min: number;
  max: number;
  default: number;
  /** The slider's step; 1 for a whole number. */
  step: number;
  log: boolean;
  about: string;
}

export interface ChartSpec {
  id: string;
  title: string;
  about: string;
  computed_by: string;
  params: ChartParam[];
}

export interface ChartFigure {
  /** May hold TeX between dollars, as may the note. */
  label: string;
  value: number | null;
  text: string;
  unit: string;
  note: string;
}

export interface ChartData {
  x_label: string;
  /** Whether x is a length in µm, shown in the unit chosen app-wide. */
  x_length: boolean;
  y_label: string;
  /** Whether y is drawn on a logarithmic scale. */
  y_log: boolean;
  y_range: [number, number] | null;
  /** Each curve, `colour` the palette's colour it is drawn in when it belongs with another (a TE and a TM mode of one order), or null for the next in turn. */
  series: { label: string; points: [number, number][]; dashed: boolean; colour: number | null }[];
  marker: number | null;
  figures: ChartFigure[];
  note: string;
  /** Pictures of a field over a cross-section, drawn side by side in place of the curves when there are any. */
  maps: { label: string; raster: Raster; outline: [number, number][][] }[];
}

/** A lesson's device drawn (studio/src-tauri/src/diagrams.rs; the drawings in components/diagrams). */
export interface DiagramSpec {
  id: string;
  title: string;
  /** What the schematic shows; may hold TeX between dollars. */
  caption: string;
  /** The built-in job (jobs/<file>) whose structure the 3D view shows, if any. */
  job: string | null;
  /** What the 3D view shows. */
  caption_3d: string;
}

/** A shelf of lessons in the order to read them (academy/tracks.yaml); every lesson is on exactly one. */
export interface Track {
  id: string;
  title: string;
  /** One line: what its lessons are about. */
  summary: string;
  /** A shelf beside the tracks, not a step on the way through them. */
  shelf: boolean;
  /** Its lessons' ids, in order. */
  lessons: string[];
}

/** A way through the tracks for one kind of reader. */
export interface LearningPath {
  id: string;
  title: string;
  summary: string;
  /** Its lessons' ids, in order. */
  lessons: string[];
  /** The numbered sections to read of a lesson read only in part, by its id; the others whole. */
  sections: Record<string, number[]>;
}

export interface Academy {
  lessons: Lesson[];
  tracks: Track[];
  paths: LearningPath[];
  diagrams: DiagramSpec[];
  charts: ChartSpec[];
}

/** Where a lesson stands on a list of lessons (a track's or a path's): its place, 1 to n, and its neighbours. */
export interface Place {
  k: number;
  n: number;
  previous: string | null;
  next: string | null;
}

/** A lesson's place among `ids`, or null when it isn't one of them. */
export function placeIn(ids: string[], id: string): Place | null {
  const k = ids.indexOf(id);
  return k < 0 ? null : { k: k + 1, n: ids.length, previous: ids[k - 1] ?? null, next: ids[k + 1] ?? null };
}

/** The track a lesson is on. */
export const trackOf = (data: Academy, id: string) => data.tracks.find((t) => t.lessons.includes(id)) ?? null;

/** The path followed, when the lesson is on it. */
export const pathOf = (data: Academy, id: string) => {
  const p = data.paths.find((x) => x.id === academy.path);
  return p && p.lessons.includes(id) ? p : null;
};

export const LEVELS: Level[] = ["introductory", "intermediate", "advanced"];

/** A subsection's heading split into its number and its question: "2. Why …?" is ["2", "Why …?"]. */
export function numbered(title: string): [string | null, string] {
  const m = /^(\d+)\.\s+(.*)$/.exec(title);
  return m ? [m[1], m[2]] : [null, title];
}

/** A unit as people read it. */
export const unitName = (unit: string) => (unit === "um" ? "µm" : unit);

export const academy = $state({
  data: null as Academy | null,
  error: "",
  /** The lesson open, by id; none for the overview of the tracks. */
  lesson: null as string | null,
  /** The path being followed, by id: a lesson on it says where it stands on it, and what comes next. */
  path: null as string | null,
  /** A section to open the next lesson at, by its number (a path reading a lesson in part). */
  at: null as number | null,
  /** The sections folded, as "lesson#section": every section is open until folded. */
  closed: [] as string[],
});

let loading: Promise<void> | null = null;

export function loadAcademy(): Promise<void> {
  loading ??= api
    .academy()
    .then((data) => {
      academy.data = data;
    })
    .catch((e) => {
      academy.error = String(e);
      loading = null;
    });
  return loading;
}
