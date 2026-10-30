import { ExternalLink } from "lucide-react";
import { redirect } from "next/navigation";
import Breadcrumbs from "@/components/projects/Breadcrumbs";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import { projectsMetadata } from "@/lib/projects/metadata";
import { blobUrl, getPythonReferencePage } from "@/lib/projects/photonoxide/github";
import {
  DOCS_PATH,
  PHOTONOXIDE_OG_IMAGE,
  PHOTONOXIDE_PATH,
  PYTHON_DOCS_PATH,
  PYTHON_REFERENCE_PATH,
} from "@/lib/projects/photonoxide/meta";

const DESCRIPTION =
  "The photonoxide package's API reference: every public class, function and attribute of the Python package, with its docstring, generated from the package of the latest release.";

export const metadata = projectsMetadata({
  title: "Python API reference",
  fullTitle: "photonoxide's Python API reference",
  description: DESCRIPTION,
  path: PYTHON_REFERENCE_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Docs", path: DOCS_PATH },
  { name: "Python", path: PYTHON_DOCS_PATH },
  { name: "API reference", path: PYTHON_REFERENCE_PATH },
];

const GUIDE = "docs/python.md";

/**
 * The reference's first page, `photonoxide.html`, when the latest release has the reference
 * attached; until then (a release made before it was attached) a page that says so, with the
 * guide and the docstrings' source to read meanwhile.
 */
export default async function PythonReferencePage() {
  if (await getPythonReferencePage("photonoxide.html")) {
    redirect(`${PYTHON_REFERENCE_PATH}/photonoxide.html`);
  }
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Docs</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">Python API reference</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
        <p className="mt-4 flex flex-wrap gap-2">
          <a href={PYTHON_DOCS_PATH} className="proj-pill">
            The Python guide
          </a>
          <a
            href={blobUrl("python/photonoxide/__init__.py")}
            target="_blank"
            rel="noopener noreferrer"
            className="proj-pill"
          >
            The docstrings on GitHub
            <ExternalLink size={12} aria-hidden />
          </a>
        </p>
      </header>
      <div className="mt-10">
        <SourceUnavailable href={blobUrl(GUIDE)} linkLabel="The guide on GitHub" />
      </div>
    </main>
  );
}
