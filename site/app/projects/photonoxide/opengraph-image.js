import { OG_CONTENT_TYPE, OG_SIZE, projectsOgImage } from "@/lib/projects/og-image";
import { PHOTONOXIDE_TAGLINE } from "@/lib/projects/photonoxide/meta";

export const alt = `photonoxide — ${PHOTONOXIDE_TAGLINE}`;
export const size = OG_SIZE;
export const contentType = OG_CONTENT_TYPE;

export default function Image() {
  return projectsOgImage({
    title: "photonoxide",
    description:
      "Mode solvers, FDFD, FDTD, inverse design and layout in one Rust library, with a studio to watch every run live. Validated.",
    chips: ["Rust", "Validated", "MIT OR Apache-2.0"],
    footer: "tachsin.gr/projects/photonoxide",
  });
}
