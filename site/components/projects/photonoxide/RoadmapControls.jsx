"use client";

import { ChevronsDownUp, ChevronsUpDown } from "lucide-react";
import { useEffect, useState } from "react";

/** The roadmap's collapsible parts: each milestone and each section is a `<details data-roadmap>`. */
const SELECTOR = "details[data-roadmap]";

/** Open the part a link points at (`#054-geometry-and-devices`), and every part around it. */
function openTarget() {
  const id = decodeURIComponent(window.location.hash.slice(1));
  const target = id ? document.getElementById(id) : null;
  if (!target) return;
  let details = target.tagName === "DETAILS" ? target : target.closest("details");
  const inside = details !== target;
  while (details) {
    details.open = true;
    details = details.parentElement?.closest("details") ?? null;
  }
  // a target that was hidden in a closed part couldn't be scrolled to
  if (inside) target.scrollIntoView({ block: "start" });
}

/**
 * Expand all and Collapse all for the roadmap, and a link to a milestone
 * opening it. The parts are native `<details>`, so without JavaScript they
 * still open and close one by one; these buttons only appear once the page
 * runs.
 */
export default function RoadmapControls() {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    setReady(true);
    openTarget();
    window.addEventListener("hashchange", openTarget);
    return () => window.removeEventListener("hashchange", openTarget);
  }, []);

  if (!ready) return null;

  const setAll = (open) => {
    for (const details of document.querySelectorAll(SELECTOR)) details.open = open;
  };

  return (
    <div role="group" aria-label="Every milestone and section" className="flex flex-wrap gap-2">
      <button type="button" className="btn btn-sm btn-ghost border-base-content/15" onClick={() => setAll(true)}>
        <ChevronsUpDown size={15} aria-hidden />
        Expand all
      </button>
      <button type="button" className="btn btn-sm btn-ghost border-base-content/15" onClick={() => setAll(false)}>
        <ChevronsDownUp size={15} aria-hidden />
        Collapse all
      </button>
    </div>
  );
}
