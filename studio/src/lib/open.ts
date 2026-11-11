// Opening a link in the system's browser (or mail client), saying so when it can't: the opener
// plugin's refusal (a URL its scope doesn't allow) or a failure to start the browser would
// otherwise be a silent, unhandled promise. A paper's DOI link opens the paper's page in the
// studio instead (Papers): what it is, where photonoxide cites it, and its publisher a click away.
import { openUrl as open } from "@tauri-apps/plugin-opener";

import { toast } from "./app.svelte";
import { openPaperLink } from "./papers.svelte";

/** The DOI a doi.org link points to, if it is one. */
export function doiOf(url: string): string | null {
  const m = /^https?:\/\/(?:dx\.)?doi\.org\/(10\.\S+)$/i.exec(url.trim());
  return m ? decodeURIComponent(m[1]) : null;
}

/**
 * Opens `url` with the system's default application, and says so in a toast if it can't. A
 * doi.org link opens the paper's page in Papers, unless `external` (the paper's own page, going
 * on to its publisher).
 */
export async function openUrl(url: string, { external = false }: { external?: boolean } = {}): Promise<void> {
  const doi = external ? null : doiOf(url);
  if (doi) return openPaperLink(doi);
  try {
    await open(url);
  } catch (e) {
    toast(`Couldn't open ${url}: ${e}`, "error");
  }
}
