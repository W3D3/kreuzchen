import { invoke } from "@tauri-apps/api/core";
import type { ConnectionInfo, Definition, DefinitionEntry, Draft, DraftSummary, Graded, InstallOutcome, Input, Opened, OpenedDefinition, OpenedResult, RowsResult, SavedRecord, SavedSummary, Summary } from "./types";

export const listDefinitions = () => invoke<DefinitionEntry[]>("list_definitions");
export const getDefinition = (file: string, locked: boolean) => invoke<OpenedDefinition>("get_definition", { file, locked });
export const grade = (file: string, locked: boolean, hash: string, input: Input) => invoke<Graded>("grade", { file, locked, hash, input });
export const dataDir = () => invoke<string>("data_dir");
export const openDefinitionsFolder = () => invoke<void>("open_definitions_folder");
/** Overwrites `name` when given (the entry was saved before), then deletes `draft`, the entry's autosaved draft. */
export const saveResult = (record: SavedRecord, name: string | null, draft: string | null) => invoke<string>("save_result", { record, name, draft });
export const listResults = () => invoke<SavedSummary[]>("list_results");
export const openResult = (name: string) => invoke<OpenedResult>("open_result", { name });
export const deleteResult = (name: string) => invoke<void>("delete_result", { name });
export const saveDraft = (name: string | null, draft: Draft) => invoke<string>("save_draft", { name, draft });
export const listDrafts = () => invoke<DraftSummary[]>("list_drafts");
export const openDraft = (name: string) => invoke<Opened<Draft>>("open_draft", { name });
export const deleteDraft = (name: string) => invoke<void>("delete_draft", { name });
export const exportResult = (name: string, dest: string) => invoke<void>("export_result", { name, dest });
export const importResult = (src: string) => invoke<string>("import_result", { src });
export const validateDefinition = (yaml: string) => invoke<Summary>("validate_definition", { yaml });
export const definitionToYaml = (def: Definition) => invoke<string>("definition_to_yaml", { def });
export const yamlToDefinition = (yaml: string) => invoke<Definition>("yaml_to_definition", { yaml });
export const gradeDraft = (yaml: string, input: Input) => invoke<Graded>("grade_draft", { yaml, input });
export const readDefinitionSource = (file: string, locked: boolean) => invoke<string>("read_definition_source", { file, locked });
export const saveDefinition = (file: string, locked: boolean, yaml: string, expectedHash: string | null) =>
  invoke<string>("save_definition", { file, locked, yaml, expectedHash });
export const deleteDefinition = (file: string, locked: boolean) => invoke<void>("delete_definition", { file, locked });
export const duplicateDefinition = (file: string, locked: boolean) => invoke<string>("duplicate_definition", { file, locked });
export const registryConnection = () => invoke<ConnectionInfo | null>("registry_connection");
export const registryConnect = (url: string, user: string, key: string) => invoke<void>("registry_connect", { url, user, key });
export const registryDisconnect = () => invoke<void>("registry_disconnect");
export const registryRows = () => invoke<RowsResult>("registry_rows");
export const registryInstall = (id: string, replaceLocal: boolean) => invoke<InstallOutcome>("registry_install", { id, replaceLocal });
export const registryRemove = (file: string) => invoke<void>("registry_remove", { file });
