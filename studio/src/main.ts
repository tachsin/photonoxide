// The studio window's web view: a Svelte app (App.svelte) on the program's commands.

import { mount } from "svelte";

import App from "./App.svelte";
import "./app.css";

mount(App, { target: document.getElementById("app")! });
