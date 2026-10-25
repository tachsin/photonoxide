// Opening a link in the system's browser (or mail client), saying so when it can't: the opener
// plugin's refusal (a URL its scope doesn't allow) or a failure to start the browser would
// otherwise be a silent, unhandled promise.
import { openUrl as open } from "@tauri-apps/plugin-opener";

import { toast } from "./app.svelte";

/** Opens `url` with the system's default application, and says so in a toast if it can't. */
export async function openUrl(url: string): Promise<void> {
  try {
    await open(url);
  } catch (e) {
    toast(`Couldn't open ${url}: ${e}`, "error");
  }
}
