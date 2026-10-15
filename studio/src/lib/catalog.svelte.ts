// What ships inside the program, loaded once: the examples and the built-in jobs.

import { api, type Catalog, type JobExample, type JobFormat } from "./api";
import type { JobModel } from "./job";

export const catalog = $state({
  loaded: false,
  data: null as Catalog | null,
  error: "",
});

let loading: Promise<void> | null = null;

export function loadCatalog(): Promise<void> {
  loading ??= (async () => {
    try {
      const data = await api.catalog();
      catalog.data = data;
    } catch (e) {
      catalog.error = String(e);
    }
    catalog.loaded = true;
  })();
  return loading;
}

/** A job's text, in `format`, as the Custom device's form holds it (the program reads it). */
export function modelOf(text: string, format: JobFormat = "toml"): Promise<JobModel> {
  return api.jobForm(text, format);
}

export function jobExample(file: string): JobExample | undefined {
  return catalog.data?.jobs.find((j) => j.file === file);
}
