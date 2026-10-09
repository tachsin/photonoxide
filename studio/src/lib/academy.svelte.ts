// The Academy (studio/src-tauri/src/academy.rs and charts.rs): its lessons, loaded once, and what
// the page remembers between visits: the lesson open, how deep it reads, and the sections opened.

import { api } from "./api";

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
  | { kind: "answer"; markdown: string };

export interface LessonSection {
  title: string;
  /** Its anchor, from the title. */
  id: string;
  depth: Depth;
  blocks: LessonBlock[];
}

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
  /** As a chart block writes it: "um", "nm", "dB/cm", or "" for a number. */
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
  y_range: [number, number] | null;
  series: { label: string; points: [number, number][]; dashed: boolean }[];
  marker: number | null;
  figures: ChartFigure[];
  note: string;
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

export interface Academy {
  lessons: Lesson[];
  diagrams: DiagramSpec[];
  charts: ChartSpec[];
}

/** The depths in order: reading to "theory" opens intuition and theory. */
export const DEPTHS: Depth[] = ["intuition", "theory", "research"];

export const LEVELS: Level[] = ["introductory", "intermediate", "advanced"];

/** A unit as people read it. */
export const unitName = (unit: string) => (unit === "um" ? "µm" : unit);

export const academy = $state({
  data: null as Academy | null,
  error: "",
  /** The lesson open, by id. */
  lesson: null as string | null,
  /** How deep each lesson reads, by its id. */
  depth: {} as Record<string, Depth>,
  /** Sections opened one by one beyond that depth, as "lesson#section". */
  opened: [] as string[],
  /** Sections closed one by one within it. */
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
