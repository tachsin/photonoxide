# Handoff: the materials catalogue's gaps and crystal tags

Branch `materials-gaps`. Paused on 2026-10-03; delete this file before merging.

**Two goals, from the owner:**
1. **Gaps are not dead ends.** Notes like "r41(x): Adachi (1985) has no electro optic section; no
   primary measurement of AlGaAs r41 in hand" must go. For each gap: (a) re-read the papers in hand
   IN FULL (long reviews hide sections: adachi-1985.pdf page by page; shoji-1997 for LiNbO₃ d22/d15
   and InP; jazbinsek-zgonik-2002 for MgO:LN; ohashi-1993 for AlGaAs d14; berseth-1992,
   sugie-tada-1976, suzuki-tada-1984; ueno-1997 and ahler-2026(+supp) for InGaP r41) and add any
   number that is there, with tests against the printed value; (b) for what remains, name the best
   current source (DOI checked on Crossref; OpenAlex/arXiv/Unpaywall for access): open → use it;
   closed → `- [ ]` entry under "## Materials catalogue: next" in `G:\My Drive\photonoxide-papers\README.md`;
   (c) word gaps neutrally everywhere ("r₄₁(x): from <paper>, coming"), shown in the studio as a calm
   "coming" chip, not a warning. Known gaps: AlGaAs r41(x) and d14(x), LiNbO₃ d22 and d15, MgO:LN's
   r, InGaP r41, InP absolute d14, InGaP 1.8–1.9 eV, AlGaN n(x).
2. **Scientific crystal tags.** Hermann–Mauguin with overbars plus Schoenflies (Si m3̄m (O_h), Fd3̄m;
   LiNbO₃ 3m (C₃ᵥ), R3c; GaAs/AlGaAs/InGaP/InP 4̄3m (T_d), F4̄3m; AlN/AlGaN 6mm (C₆ᵥ), P6₃mc;
   amorphous SiO₂/Si₃N₄ ∞∞m). "centrosymmetric: no χ(2), no Pockels" becomes a tag
   "centrosymmetric (inversion centre)" whose detail explains: rank-3 polar tensors (χ(2), r) are odd
   under inversion and vanish in the bulk (Neumann's principle); caveats: dipole approximation
   (quadrupole/magnetic-dipole terms), surfaces and interfaces, inhomogeneous strain (strained-silicon
   χ(2)), a static field (EFISH, χ(3)E₀), Kerr allowed. Non-centrosymmetric tags state what symmetry
   allows (GaAs: d14 = d25 = d36, r41 = r52 = r63; LiNbO₃: d33, d31 = d15, d22; r33, r13, r22, r42),
   consistent with the tensors on the page. Cite standard sources (Nye 1957, ITA Vol. A, Boyd 2008
   Sec. 1.5–1.6, Roberts 1992).

**Done** (unbuilt): `src/material/catalogue/tags.rs` (a `Tag` type: kind, label, detail, source; tags
written from the point group and from the same `pattern` table the tensors follow, so tags and
tensors can't disagree; source constants NYE, ROBERTS, ITA), its `mod` line. The paper re-reading
and the gap rewording haven't started in code.

**Next:** finish tags.rs and `Entry::tags`; replace the old tag strings and the "missing" notes in
the catalogue; the studio's Materials page (hover cards for tags, "coming" chips; KaTeX via
MathText); docs/methods/catalogue.md; tests. Then the paper re-reading (goal 1).

**Rules:** numbers only from primary papers read in full; tests against printed values; validation
cases in the GitHub-safe LaTeX convention; additive API (`cargo semver-checks check-release -p
photonoxide --baseline-rev v0.4.0`); green CI incl. `pnpm build`; conventional commits, no AI
attribution; no tags. Papers: `G:\My Drive\photonoxide-papers`.

## Progress at the second pause (2026-10-04)

Rebased on main (e458386). Uncommitted work was committed as-is with this note; it has not been
built or tested since the last edits.

**Done:**
- `src/material/catalogue/tags.rs` (335 lines): the scientific tags (Hermann–Mauguin with
  overbars, Schoenflies, centrosymmetric statement with its physics and caveats, what each
  non-centrosymmetric class allows), sourced (Nye 1957, ITA Vol. A, Roberts 1992, Boyd 2008).
- `src/material/catalogue/coming.rs` (new, 206 lines): a `Coming` table (property, source,
  citation, DOI, what the papers in hand say, with places), from which `Entry::missing` is written,
  so they can't disagree. Sources named so far: Shoji, Kondo & Ito 2002 (d coefficients incl.
  LN d22/d15, InP), Glick, Reinhart & Martin 1988 (AlGaAs r41), Miller, Nordland & Bridenbaugh
  1971 (LN d vs melt composition), Akiyama/Nakano/Shoji 2017 (LN and MgO:LN r), Yonekura 2007
  (LN r22), Kato et al. 1994 (AlGaInP/InGaP index), Brunner 1997 and Sanford (AlGaN n(x)).
- Entries reworded neutrally (entries.rs), references added (references.rs), new tests (tests.rs,
  +187 lines) and checks (checks.rs), one validation case (validation.rs, docs/validation.md).
- Studio: the Materials page shows a "Coming" row of chips with hover details (Materials.svelte,
  api.ts, studio/src-tauri/src/materials.rs).
- Papers README (G:\My Drive\photonoxide-papers): "## Materials catalogue: next" lists the closed
  papers above as `- [ ]`; Ulsig et al. 2024 and Thiel et al. 2024 (open) were saved and ticked.

**Next:**
1. Build and test (`cargo test --workspace`, `pnpm build`); fix what broke.
2. Fix one inconsistency: coming.rs says "Sanford et al. 2005" but the README entry is Sanford et al.,
   J. Appl. Phys. 94, 2980 (2003), DOI 10.1063/1.1598276: check which is right and make both agree.
3. Use the two open papers just saved: Ulsig 2024 (GaAs/AlGaAs χ(2) platforms) and Thiel 2024
   (InGaP-on-insulator): add what they measure, with tests against their printed values.
4. Goal 1's remaining part: re-read the papers in hand IN FULL (adachi-1985.pdf page by page, and
   the others listed above) and record in coming.rs what each says, with page/table; add any
   number that is actually there.
5. Check the Materials page in the app (tags with hover details, Coming chips, both themes, nm/µm);
   the µm/nm switch (#107) is on main: keep the page's lengths converting.
6. Delete this HANDOFF.md, PR, CI green, merge.

**Added after the pause:** the owner's rule: cite Boyd's *Nonlinear Optics* **4th edition** (2020,
`boyd-2020.pdf`, DOI 10.1016/C2015-0-05510-1) instead of the 3rd (2008) wherever possible: tags.rs
and coming.rs cite "Boyd 2008, Sec. 1.5–1.6": re-check the section numbers in the 4th edition and
update. Bertaccini & Durastante 2018 (`chapman-2018.pdf`) is in the folder too (iterative methods
and preconditioning; for the 3D preconditioner work, not this branch).
