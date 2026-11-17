import { mount } from "svelte";
import EditorProto from "./EditorProto.svelte";
import FlowProto from "./FlowProto.svelte";
import ThemeSwitch from "./ThemeSwitch.svelte";

// #proto: editor; #proto-entry / #proto-result: entry and result screens.
export default (target: HTMLElement) => {
  mount(ThemeSwitch, { target: document.body });
  return mount(location.hash === "#proto" ? EditorProto : FlowProto, { target });
};
