import Breadcrumbs from "@/components/projects/Breadcrumbs";
import JsonLd from "@/components/projects/JsonLd";
import SourceUnavailable from "@/components/projects/SourceUnavailable";
import ValidationRows from "@/components/projects/photonoxide/ValidationRows";
import { breadcrumbList } from "@/lib/projects/json-ld";
import { projectsMetadata } from "@/lib/projects/metadata";
import { PHOTONOXIDE_LINKS, PHOTONOXIDE_OG_IMAGE, PHOTONOXIDE_PATH, VALIDATION_PATH } from "@/lib/projects/photonoxide/meta";
import { areaTitle, getReport } from "@/lib/projects/photonoxide/validation";

const DESCRIPTION =
  "Every check of photonoxide against something it must agree with: closed-form solutions, published results and other codes, each with its tolerance and grid. The library writes this report, and CI fails when a case fails or the report is out of date.";

export const metadata = projectsMetadata({
  title: "Validation",
  fullTitle: "Validation report — photonoxide",
  description: DESCRIPTION,
  path: VALIDATION_PATH,
  image: PHOTONOXIDE_OG_IMAGE,
});

const CRUMBS = [
  { name: "Projects", path: "/projects" },
  { name: "photonoxide", path: PHOTONOXIDE_PATH },
  { name: "Validation", path: VALIDATION_PATH },
];

const TIERS = [
  { tier: "analytic", label: "Analytic", note: "a closed-form solution" },
  { tier: "published", label: "Published", note: "a published measurement or result" },
  { tier: "cross-code", label: "Cross-code", note: "an established code on the same structure" },
];

export default async function ValidationPage() {
  const { ok, cases } = await getReport();
  const areas = [...new Set(cases.map((c) => c.area))];
  const passed = cases.filter((c) => c.passed).length;
  return (
    <main className="proj-container pt-10 pb-16 sm:pt-14">
      <JsonLd data={breadcrumbList(CRUMBS)} />
      <Breadcrumbs items={CRUMBS} />
      <header className="proj-rise max-w-3xl">
        <p className="proj-eyebrow">Validation</p>
        <h1 className="mt-2 font-semibold text-4xl tracking-tight sm:text-5xl">The validation report</h1>
        <p className="proj-lead mt-4 text-lg">{DESCRIPTION}</p>
        <p className="proj-lead mt-3 text-sm">
          Values have six significant digits; a value that should be zero shows as "≤ tolerance" when it is within
          it. The report is{" "}
          <a href={PHOTONOXIDE_LINKS.report} target="_blank" rel="noopener noreferrer" className="link link-primary">
            docs/validation.md
          </a>
          , written by <code>photonoxide validate</code>.
        </p>
      </header>

      {ok ? (
        <>
          <dl className="proj-rise-1 mt-8 grid max-w-3xl grid-cols-2 gap-px overflow-hidden rounded-2xl border border-base-content/10 bg-base-content/10 sm:grid-cols-4">
            <div className="bg-base-100 p-4">
              <dt className="text-base-content/60 text-xs">Cases passing</dt>
              <dd className="mt-1 font-semibold text-2xl">
                {passed} / {cases.length}
              </dd>
            </div>
            {TIERS.map((t) => (
              <div key={t.tier} className="bg-base-100 p-4">
                <dt className="text-base-content/60 text-xs" title={t.note}>
                  {t.label}
                </dt>
                <dd className="mt-1 font-semibold text-2xl">{cases.filter((c) => c.tier === t.tier).length}</dd>
              </div>
            ))}
          </dl>
          {areas.map((area) => (
            <section key={area} className="mt-12" aria-labelledby={`area-${area}`}>
              <h2 id={`area-${area}`} className="font-semibold text-2xl tracking-tight">
                {areaTitle(area)}
              </h2>
              <ValidationRows cases={cases.filter((c) => c.area === area)} />
            </section>
          ))}
        </>
      ) : (
        <div className="mt-10">
          <SourceUnavailable href={PHOTONOXIDE_LINKS.report} linkLabel="The report on GitHub">
            The report is docs/validation.md in the repository.
          </SourceUnavailable>
        </div>
      )}
    </main>
  );
}
