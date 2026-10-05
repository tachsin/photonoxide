// What ships inside the program, loaded once: the examples and the built-in jobs.

import { api, type Catalog, type JobExample, type JobFormat } from "./api";
import { fromModel, type JobModel } from "./job";

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

/** A job's text, in `format`, as the builder's model (the backend parses it). */
export async function modelOf(text: string, format: JobFormat = "toml"): Promise<JobModel> {
  const check = await api.checkJob(text, format);
  return fromModel(check.model ?? {}, text);
}

export function jobExample(file: string): JobExample | undefined {
  return catalog.data?.jobs.find((j) => j.file === file);
}
