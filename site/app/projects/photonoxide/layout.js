import ProjectSubNav from "@/components/projects/ProjectSubNav";
import { EXTRA_SECTIONS, PHOTONOXIDE_LINKS } from "@/lib/projects/photonoxide/meta";
import { getProject } from "@/lib/projects/registry";

const project = getProject("photonoxide");

/** The registry's sections, then this folder's own pages (meta.js) it doesn't list yet. */
const SECTIONS = [
  ...project.sections,
  ...EXTRA_SECTIONS.filter((e) => !project.sections.some((s) => s.path === e.path)),
];

const LINKS = [
  { label: "GitHub", href: PHOTONOXIDE_LINKS.github },
  { label: "docs.rs", href: PHOTONOXIDE_LINKS.docsRs },
];

export default function PhotonoxideLayout({ children }) {
  return (
    <>
      <ProjectSubNav name={project.name} href={project.href} sections={SECTIONS} links={LINKS} />
      {children}
    </>
  );
}
