use crate::{
    Paths,
    definitions::{self, DefinitionEntry, OpenedDefinition, Summary},
    editor,
    store::{self, SavedRecord, SavedSummary},
};
use kreuzchen_engine::{Grading, Input, schema::Definition};
use serde::Serialize;
use std::path::Path;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
pub struct Graded {
    pub grading: Grading,
    pub text: String,
}

#[tauri::command]
pub fn list_definitions(paths: State<Paths>) -> Vec<DefinitionEntry> {
    definitions::scan_all(&paths.definitions, &paths.registry)
}

#[tauri::command]
pub fn get_definition(
    paths: State<Paths>,
    file: String,
    locked: bool,
) -> Result<OpenedDefinition, String> {
    let (q, hash) = definitions::open(paths.dir(locked), &file)?;
    Ok(OpenedDefinition {
        file,
        hash,
        definition: q.def,
        locked,
    })
}

#[tauri::command]
pub fn grade(
    paths: State<Paths>,
    file: String,
    locked: bool,
    hash: String,
    input: Input,
) -> Result<Graded, String> {
    let (q, current) = definitions::open(paths.dir(locked), &file)?;
    definitions::check_hash(&current, &hash)?;
    let grading = q.grade(&input).map_err(|e| e.to_string())?;
    let text = q.plain_text(&input, &grading);
    Ok(Graded { grading, text })
}

#[tauri::command]
pub fn data_dir(paths: State<Paths>) -> String {
    paths.definitions.to_string_lossy().into_owned()
}

#[tauri::command]
pub fn open_definitions_folder(app: tauri::AppHandle, paths: State<Paths>) -> Result<(), String> {
    app.opener()
        .open_path(paths.definitions.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct OpenedResult {
    pub record: SavedRecord,
    pub current: Option<DefinitionEntry>,
    pub outdated: bool,
}

#[tauri::command]
pub fn save_result(paths: State<Paths>, record: SavedRecord) -> Result<String, String> {
    store::save(&paths.results, &record)
}

#[tauri::command]
pub fn list_results(paths: State<Paths>) -> Vec<SavedSummary> {
    store::list(&paths.results)
}

#[tauri::command]
pub fn open_result(paths: State<Paths>, name: String) -> Result<OpenedResult, String> {
    let record = store::load(&paths.results, &name)?;
    let q = &record.questionnaire;
    let current =
        definitions::find_by_id(&paths.definitions, &paths.registry, &q.id, Some(&q.hash));
    let outdated = current
        .as_ref()
        .is_some_and(|c| c.hash != record.questionnaire.hash);
    Ok(OpenedResult {
        record,
        current,
        outdated,
    })
}

#[tauri::command]
pub fn delete_result(paths: State<Paths>, name: String) -> Result<(), String> {
    store::delete(&paths.results, &name)
}

#[tauri::command]
// Paths come from OS dialogs; import only accepts valid records.
pub fn export_result(paths: State<Paths>, name: String, dest: String) -> Result<(), String> {
    store::export(&paths.results, &name, Path::new(&dest))
}

#[tauri::command]
// Paths come from OS dialogs; import only accepts valid records.
pub fn import_result(paths: State<Paths>, src: String) -> Result<String, String> {
    store::import(&paths.results, Path::new(&src))
}

#[tauri::command]
pub fn validate_definition(yaml: String) -> Result<Summary, Vec<String>> {
    kreuzchen_engine::load(&yaml)
        .map(|q| definitions::summary(&q))
        .map_err(|errs| errs.iter().map(|e| e.to_string()).collect())
}

#[tauri::command]
pub fn definition_to_yaml(def: Definition) -> String {
    kreuzchen_engine::to_yaml(&def)
}

#[tauri::command]
pub fn yaml_to_definition(yaml: String) -> Result<Definition, String> {
    kreuzchen_engine::parse_definition(&yaml).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn grade_draft(yaml: String, input: Input) -> Result<Graded, String> {
    let q = definitions::load_text(&yaml)?;
    let grading = q.grade(&input).map_err(|e| e.to_string())?;
    let text = q.plain_text(&input, &grading);
    Ok(Graded { grading, text })
}

#[tauri::command]
pub fn read_definition_source(
    paths: State<Paths>,
    file: String,
    locked: bool,
) -> Result<String, String> {
    editor::read_source(paths.dir(locked), &file)
}

fn refuse_locked(locked: bool) -> Result<(), String> {
    if locked {
        Err("Fragebögen aus der Registry können nicht geändert werden.".into())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn save_definition(
    paths: State<Paths>,
    file: String,
    locked: bool,
    yaml: String,
    expected_hash: Option<String>,
) -> Result<String, String> {
    refuse_locked(locked)?;
    editor::save(
        &paths.definitions,
        &paths.registry,
        &file,
        &yaml,
        expected_hash.as_deref(),
    )
}

#[tauri::command]
pub fn delete_definition(paths: State<Paths>, file: String, locked: bool) -> Result<(), String> {
    refuse_locked(locked)?;
    editor::delete(&paths.definitions, &file)
}

#[tauri::command]
pub fn duplicate_definition(
    paths: State<Paths>,
    file: String,
    locked: bool,
) -> Result<String, String> {
    editor::duplicate(&paths.definitions, &paths.registry, locked, &file)
}
