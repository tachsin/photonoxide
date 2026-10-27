// The window's web view: the studio (App.svelte) on the program's commands, or, opened by
// `photonoxide setup` or `photonoxide uninstall` (src-tauri/src/setup.rs), the setup's window.
// Each is loaded only when it's the one: the studio's modules start talking to the program as
// they load, and the setup's window has none of its commands.

import { mount } from "svelte";

import "./app.css";

const target = document.getElementById("app")!;
if ((window as { __PHOTONOXIDE_SETUP__?: unknown }).__PHOTONOXIDE_SETUP__) {
  import("./Setup.svelte").then(({ default: Setup }) => mount(Setup, { target }));
} else {
  import("./App.svelte").then(({ default: App }) => mount(App, { target }));
}
