import { invoke } from "@tauri-apps/api/core";
import type { Definition, DefinitionEntry, Graded, Input, OpenedDefinition, OpenedResult, SavedRecord, SavedSummary, Summary } from "./types";

export const listDefinitions = () => invoke<DefinitionEntry[]>("list_definitions");
export const getDefinition = (file: string) => invoke<OpenedDefinition>("get_definition", { file });
export const grade = (file: string, hash: string, input: Input) => invoke<Graded>("grade", { file, hash, input });
export const dataDir = () => invoke<string>("data_dir");
export const openDefinitionsFolder = () => invoke<void>("open_definitions_folder");
export const saveResult = (record: SavedRecord) => invoke<string>("save_result", { record });
export const listResults = () => invoke<SavedSummary[]>("list_results");
export const openResult = (name: string) => invoke<OpenedResult>("open_result", { name });
export const deleteResult = (name: string) => invoke<void>("delete_result", { name });
export const exportResult = (name: string, dest: string) => invoke<void>("export_result", { name, dest });
export const importResult = (src: string) => invoke<string>("import_result", { src });
export const validateDefinition = (yaml: string) => invoke<Summary>("validate_definition", { yaml });
export const definitionToYaml = (def: Definition) => invoke<string>("definition_to_yaml", { def });
export const yamlToDefinition = (yaml: string) => invoke<Definition>("yaml_to_definition", { yaml });
export const readDefinitionSource = (file: string) => invoke<string>("read_definition_source", { file });
export const saveDefinition = (file: string, yaml: string, expectedHash: string | null) =>
  invoke<string>("save_definition", { file, yaml, expectedHash });
export const deleteDefinition = (file: string) => invoke<void>("delete_definition", { file });
export const duplicateDefinition = (file: string) => invoke<string>("duplicate_definition", { file });
