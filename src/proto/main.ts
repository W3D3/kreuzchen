import "./proto.css";
import { mount } from "svelte";
import EditorProto from "./EditorProto.svelte";

export default (target: HTMLElement) => mount(EditorProto, { target });
