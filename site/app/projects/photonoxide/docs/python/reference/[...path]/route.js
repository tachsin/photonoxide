import { getPythonReferencePage } from "@/lib/projects/photonoxide/github";

/**
 * The Python API reference's pages, as pdoc made them: `.../docs/python/reference/photonoxide.html`
 * and `.../photonoxide/errors.html`. They are whole HTML files (their styles inline, no scripts),
 * served at pdoc's own paths, so its relative links between them work as they are; where they
 * come from is in `lib/projects/photonoxide/python-reference.js`. The landing page,
 * `.../docs/python/reference`, is in `../page.js`.
 */
export async function GET(_request, { params }) {
  const { path } = await params;
  const html = path?.every((s) => s.endsWith(".html") || !s.includes("."))
    ? await getPythonReferencePage(path.join("/"))
    : null;
  if (!html) {
    return new Response("This page of the Python API reference isn't available.", {
      status: 404,
      headers: { "content-type": "text/plain; charset=utf-8" },
    });
  }
  return new Response(html, {
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "public, max-age=3600, s-maxage=86400",
      "x-content-type-options": "nosniff",
    },
  });
}
