import "./proto.css";
import { mount } from "svelte";
import EditorProto from "./EditorProto.svelte";
import FlowProto from "./FlowProto.svelte";

// #proto: editor; #proto-entry / #proto-result: entry and result screens.
export default (target: HTMLElement) => mount(location.hash === "#proto" ? EditorProto : FlowProto, { target });
