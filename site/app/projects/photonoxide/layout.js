import ProjectSubNav from "@/components/projects/ProjectSubNav";
import { PHOTONOXIDE_LINKS } from "@/lib/projects/photonoxide/meta";
import { getProject } from "@/lib/projects/registry";

const project = getProject("photonoxide");

const LINKS = [
  { label: "GitHub", href: PHOTONOXIDE_LINKS.github },
  { label: "docs.rs", href: PHOTONOXIDE_LINKS.docsRs },
];

export default function PhotonoxideLayout({ children }) {
  return (
    <>
      <ProjectSubNav name={project.name} href={project.href} sections={project.sections} links={LINKS} />
      {children}
    </>
  );
}
