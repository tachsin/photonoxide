"""A design's process flow through Python: the flow and what the rules find in it are the ones a
``"fabrication"`` job's run records, to the bit, for the three first designs (the SOI strip, the
thin-film lithium niobate modulator, the TiN heater) and a rib; the export reads back to the
same flow, its cross-sections parse as SVG with the layers from the top down, and its traveller
holds every step and every source."""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

import pytest

import photonoxide as phox

DESIGNS = [
    "waveguide",
    "TFLN Mach-Zehnder modulator",
    "thermo-optic phase shifter",
    "rib waveguide",
]
SVG = "{http://www.w3.org/2000/svg}"


def bits(x: Any) -> Any:
    """`x` with every float as its bits, so that equal means equal to the bit."""
    if isinstance(x, dict):
        return {k: bits(v) for k, v in x.items()}
    if isinstance(x, list):
        return [bits(v) for v in x]
    if isinstance(x, float):
        return float.hex(x)
    return x


@pytest.mark.parametrize("kind", DESIGNS)
def test_a_flow_is_its_jobs_record_to_the_bit(kind: str, tmp_path: Path) -> None:
    f = phox.device_fabrication(kind)
    run = phox.run_job(phox.device_fabrication_job(kind), runs_dir=tmp_path)
    assert run.stopped is None
    flow = next(e for e in run.events if e["type"] == "flow")["flow"]
    assessment = next(e for e in run.events if e["type"] == "flow_assessment")["assessment"]
    assert bits(f.flow) == bits(flow)
    assert bits(f.assessment) == bits(assessment)
    assert f.flow["round_trip"]["differing"] == 0 and f.flow["round_trip"]["structure_equal"]
    # the run's folder holds the export: the same texts
    assert (run.dir / "flow.json").read_text(encoding="utf-8") == f.json
    assert (run.dir / "flow.md").read_text(encoding="utf-8") == f.markdown
    assert len(f.svgs) == len(f.steps)
    for k, svg in enumerate(f.svgs):
        assert (run.dir / f"flow-{k + 1:02d}.svg").read_text(encoding="utf-8") == svg


def test_the_three_first_designs_have_the_steps_the_write_up_lists() -> None:
    # docs/methods/fabrication.md: 5, 8 and 12 steps
    steps = {kind: len(phox.device_fabrication(kind).steps) for kind in DESIGNS[:3]}
    assert steps == {
        "waveguide": 5,
        "TFLN Mach-Zehnder modulator": 8,
        "thermo-optic phase shifter": 12,
    }


def test_the_export_reads_back_to_the_same_flow(tmp_path: Path) -> None:
    for kind in DESIGNS:
        files = phox.export_fabrication(kind, out_dir=tmp_path / kind.replace(" ", "-"))
        f = phox.device_fabrication(kind)
        assert [p.name for p in files[:2]] == ["flow.json", "flow.md"]
        assert len(files) == 2 + len(f.steps)
        data = json.loads(files[0].read_text(encoding="utf-8"))
        assert list(data) == ["device", "parameters", "cut", "flow", "assessment", "stacks"]
        assert data["device"] == kind
        assert bits(data["flow"]) == bits(f.flow)
        assert bits(data["assessment"]) == bits(f.assessment)
        assert json.loads(f.json) == data
        # the wafer after each step, from the top down: the cladding first, the substrate last
        assert len(data["stacks"]) == len(f.steps)
        for stack in data["stacks"]:
            assert stack[0]["name"] == "cladding" and stack[-1]["name"] == "substrate"
            assert all(row["thickness_um"] > 0 for row in stack[1:-1])


def test_each_cross_section_parses_as_svg_with_its_layers_from_the_top_down(tmp_path: Path) -> None:
    for kind in DESIGNS:
        files = phox.export_fabrication(kind, out_dir=tmp_path / kind.replace(" ", "-"))
        data = json.loads(files[0].read_text(encoding="utf-8"))
        svgs = files[2:]
        for k, path in enumerate(svgs):
            root = ET.parse(path).getroot()
            assert root.tag == f"{SVG}svg"
            title = root.find(f"{SVG}title")
            assert title is not None and title.text is not None
            assert title.text.startswith(f"Step {k + 1} of {len(svgs)}: ")
            drawing = root.find(f"{SVG}g[@id='cross-section']")
            assert drawing is not None and len(drawing.findall(f"{SVG}rect")) >= 3
            legend = root.find(f"{SVG}g[@id='layers']")
            assert legend is not None
            lines = [t.text or "" for t in legend.iter(f"{SVG}text")]
            assert lines[0] == "Layers, from the top down, the substrate last"
            assert [line.split(":")[0] for line in lines[1:]] == [
                row["name"] for row in data["stacks"][k]
            ]
        # the finished wafer draws each patterned layer's shapes
        last = ET.parse(svgs[-1]).getroot().find(f"{SVG}g[@id='cross-section']")
        assert last is not None and last.findall(f"{SVG}polygon")


def test_the_traveller_has_every_step_and_every_source() -> None:
    for kind in DESIGNS:
        f = phox.device_fabrication(kind)
        md, n = f.markdown, len(f.steps)
        for k, step in enumerate(f.steps, 1):
            assert f"### Step {k} of {n}: {step['name']}\n" in md
            assert step["reason"] in md and step["does"] in md
            assert f"flow-{k:02d}.svg" in md
        for reference in f.flow["references"]:
            assert reference["citation"] in md
            assert reference["doi"] in md
        for dimension in f.accuracy:
            assert dimension["text"] in md
        assert "Layers are listed from the top down, the substrate last" in md


def test_a_flow_names_what_is_wrong() -> None:
    with pytest.raises(ValueError, match="laser-writing"):
        phox.device_fabrication("waveguide", prefer=["laser-writing"])
    with pytest.raises(ValueError, match="both preferred and avoided"):
        phox.device_fabrication("waveguide", prefer=["ebl-hsq"], avoid=["ebl-hsq"])
    with pytest.raises(ValueError):
        phox.device_fabrication("waveguide", values={"girth": 1.0})
    with pytest.raises(TypeError):
        phox.device_fabrication("waveguide", prefer="ebl-hsq")  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        phox.device_fabrication_job("waveguide", format="ini")
    yaml = phox.device_fabrication_job("waveguide", prefer=["ebl-hsq"], format="yaml")
    assert "kind: fabrication" in yaml and "ebl-hsq" in yaml
    # a preference takes its process
    lithography = phox.device_fabrication("waveguide", prefer=["ebl-hsq"]).steps[1]["process"]
    assert lithography == "ebl-hsq"
