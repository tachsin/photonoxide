<script lang="ts">
  // Saves a sweep shown on a device's page as a study job (photonoxide::job::study_from_device):
  // the design as swept, at the sweep's fidelity, its one axis exactly the sweep's (the
  // parameter, from, to and the points, each value from + (to − from) k / (n − 1) as the page
  // took it). Run by `photonoxide run`, headless or farmed, the job's points are the sweep's to
  // the bit (studio/src-tauri/src/devices/tests.rs).
  import { Save } from "@lucide/svelte";
  import { ask } from "@tauri-apps/plugin-dialog";

  import { api, type Compute, type DeviceSpec, type StudyIn } from "../../lib/api";
  import { app, toast } from "../../lib/app.svelte";
  import { plain } from "../../lib/deviceview";

  let {
    spec,
    fidelity,
    parameter,
    from,
    to,
    points,
    name,
    disabled = false,
  }: { spec: DeviceSpec; fidelity: string; parameter: string; from: number; to: number; points: number; name: string; disabled?: boolean } = $props();

  /** The device's own model on its own grids and band: what the sweep computes by. */
  const OWN: Compute = { fdtd: null, band: null, step_nm: null, step_z_nm: null, margin_um: null, samples: null, pml_cells: null, fdfd_2d: false, timeout_minutes: null };
  let saving = $state(false);

  async function save() {
    saving = true;
    const file = name.toLowerCase().replace(/[^a-z0-9._-]+/g, "-");
    try {
      const study: StudyIn = { kind: "sweep", axes: [{ parameter, from, to, points }], corners: [] };
      const text = await api.deviceStudyJob({ ...spec, fidelity }, OWN, study, file);
      let path: string;
      try {
        path = await api.saveJob(text, "yaml", false);
      } catch (e) {
        if (!String(e).startsWith("exists:")) throw e;
        if (!(await ask(`jobs/${file}.yaml exists. Replace it?`, { title: "Replace the job?", kind: "warning" }))) return;
        path = await api.saveJob(text, "yaml", true);
      }
      app.workspaceVersion++;
      toast(`Saved ${path.split(/[\\/]/).pop()}: photonoxide run runs this sweep, --workers farms its points`, "success");
    } catch (e) {
      toast(plain(String(e)), "error");
    } finally {
      saving = false;
    }
  }
</script>

<button class="btn btn-ghost btn-xs gap-1" disabled={disabled || saving} onclick={save} title="Save this sweep as a study job in jobs/: the design as swept, at the {fidelity} model, {points} points of {parameter} from {from} to {to}">
  <Save size={12} /> Save as a study job
</button>
