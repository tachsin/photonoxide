// What ships inside the program, loaded once: the examples and the built-in jobs, each job with
// its model for the previews.

import { api, type Catalog, type JobExample } from "./api";
import { fromModel, type JobModel } from "./job";

export const catalog = $state({
  loaded: false,
  data: null as Catalog | null,
  models: {} as Record<string, JobModel>,
  error: "",
});

let loading: Promise<void> | null = null;

export function loadCatalog(): Promise<void> {
  loading ??= (async () => {
    try {
      const data = await api.catalog();
      catalog.data = data;
      for (const j of data.jobs) catalog.models[j.file] = await modelOf(j.text);
    } catch (e) {
      catalog.error = String(e);
    }
    catalog.loaded = true;
  })();
  return loading;
}

/** A job's text as the builder's model (the backend parses the TOML). */
export async function modelOf(text: string): Promise<JobModel> {
  const check = await api.checkJob(text);
  return fromModel(check.model ?? {}, text);
}

export function jobExample(file: string): JobExample | undefined {
  return catalog.data?.jobs.find((j) => j.file === file);
}
