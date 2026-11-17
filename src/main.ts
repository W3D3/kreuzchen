import "@fontsource/atkinson-hyperlegible/400.css";
import "@fontsource/atkinson-hyperlegible/700.css";
import "@fontsource/ibm-plex-mono/400.css";
import "./app.css";
import { mount } from "svelte";
import { applyTheme, loadTheme } from "./lib/theme";
import App from "./App.svelte";

// Before mounting, so the first paint already has the chosen theme.
applyTheme(document.documentElement, loadTheme());

const target = document.getElementById("app")!;
// shortcut: temporary switch for the shadcn-svelte look-and-feel prototype; drop with src/proto.
if (location.hash.startsWith("#proto")) import("./proto/main").then((m) => m.default(target));
else mount(App, { target });
