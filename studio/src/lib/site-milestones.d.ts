// The types of the project site's roadmap parser (site/lib/projects/photonoxide/milestones.js),
// plain JavaScript the site and the studio's Mission page share, so both read ROADMAP.md alike.
declare module "$site/milestones.js" {
  export interface Milestone {
    version: string;
    title: string;
    status: "done" | "next" | "planned";
    items: string[];
    done: number;
    total: number;
  }
  export interface RoadmapBlock {
    heading: string;
    anchor: string;
    body: string;
    milestone: Milestone | null;
  }
  export interface RoadmapSection {
    heading: string;
    anchor: string;
    intro: string;
    blocks: RoadmapBlock[];
  }
  export function parseMilestones(markdown: string, release?: string | null): Milestone[];
  export function splitRoadmap(markdown: string, release?: string | null): { title: string; preface: string; sections: RoadmapSection[] };
  export function checklist(body: string): { done: boolean; name: string; text: string }[];
}
