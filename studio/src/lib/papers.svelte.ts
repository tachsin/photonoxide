// Papers: a paper a page names, opened by its DOI.

import { openUrl } from "./open";

/** Opens the paper with DOI `doi`. */
export function openPaperLink(doi: string) {
  void openUrl(`https://doi.org/${doi}`);
}
