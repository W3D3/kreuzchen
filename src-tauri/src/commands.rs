use crate::{
    Paths,
    definitions::{self, DefinitionEntry, OpenedDefinition, Summary},
    editor,
    registry::{self, Connection, InstallOutcome, RowsResult},
    store::{self, Draft, DraftSummary, QRef, SavedRecord, SavedSummary},
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

/// A stored record with the installed definition it belongs to, if any.
#[derive(Serialize)]
pub struct Opened<T> {
    pub record: T,
    pub current: Option<DefinitionEntry>,
    /// The installed definition differs from the one the record was made with.
    pub outdated: bool,
}

fn with_current<T>(paths: &Paths, record: T, q: &QRef) -> Opened<T> {
    let current =
        definitions::find_by_id(&paths.definitions, &paths.registry, &q.id, Some(&q.hash));
    let outdated = current.as_ref().is_some_and(|c| c.hash != q.hash);
    Opened {
        record,
        current,
        outdated,
    }
}

#[tauri::command]
pub fn save_result(
    paths: State<Paths>,
    record: SavedRecord,
    draft: Option<String>,
) -> Result<String, String> {
    store::save_finished(&paths.results, &paths.drafts, &record, draft.as_deref())
}

#[tauri::command]
pub fn list_results(paths: State<Paths>) -> Vec<SavedSummary> {
    store::list(&paths.results)
}

#[tauri::command]
pub fn open_result(paths: State<Paths>, name: String) -> Result<Opened<SavedRecord>, String> {
    let record: SavedRecord = store::load(&paths.results, &name)?;
    let q = record.questionnaire.clone();
    Ok(with_current(&paths, record, &q))
}

#[tauri::command]
pub fn delete_result(paths: State<Paths>, name: String) -> Result<(), String> {
    store::delete(&paths.results, &name)
}

#[tauri::command]
pub fn save_draft(
    paths: State<Paths>,
    name: Option<String>,
    draft: Draft,
) -> Result<String, String> {
    store::save_draft(&paths.drafts, name.as_deref(), &draft)
}

#[tauri::command]
pub fn list_drafts(paths: State<Paths>) -> Vec<DraftSummary> {
    store::list_drafts(&paths.drafts)
}

#[tauri::command]
pub fn open_draft(paths: State<Paths>, name: String) -> Result<Opened<Draft>, String> {
    let draft: Draft = store::load(&paths.drafts, &name)?;
    let q = draft.questionnaire.clone();
    Ok(with_current(&paths, draft, &q))
}

#[tauri::command]
pub fn delete_draft(paths: State<Paths>, name: String) -> Result<(), String> {
    store::delete(&paths.drafts, &name)
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

#[derive(Serialize)]
pub struct ConnectionInfo {
    pub url: String,
    pub user: String,
}

#[tauri::command]
pub fn registry_connection(paths: State<Paths>) -> Option<ConnectionInfo> {
    registry::load_connection(&paths.data).map(|c| ConnectionInfo {
        url: c.url,
        user: c.user,
    })
}

#[tauri::command]
pub async fn registry_connect(
    paths: State<'_, Paths>,
    url: String,
    user: String,
    key: String,
) -> Result<(), String> {
    let c = Connection::new(&url, &user, &key)?;
    registry::fetch_list(&c).await?;
    registry::save_connection(&paths.data, &c)
}

#[tauri::command]
pub fn registry_disconnect(paths: State<Paths>) {
    registry::clear_connection(&paths.data)
}

#[tauri::command]
// Tauri requires `Result` from async commands that borrow `State`; remote errors arrive in `error`.
pub async fn registry_rows(paths: State<'_, Paths>) -> Result<RowsResult, String> {
    let installed = definitions::scan(&paths.registry);
    let c = registry::load_connection(&paths.data);
    Ok(registry::rows_result(c.as_ref(), &installed).await)
}

#[tauri::command]
pub async fn registry_install(
    paths: State<'_, Paths>,
    id: String,
    replace_local: bool,
) -> Result<InstallOutcome, String> {
    let c = registry::load_connection(&paths.data).ok_or("Keine Registry verbunden.")?;
    registry::install(&c, &paths, &id, replace_local).await
}

#[tauri::command]
pub fn registry_remove(paths: State<Paths>, file: String) -> Result<(), String> {
    registry::remove(&paths.registry, &file)
}

#[cfg(test)]
mod tests {
    use super::{refuse_locked, with_current};
    use crate::{Paths, store::QRef};
    use std::fs;

    #[test]
    fn opened_draft_tracks_changed_and_removed_definition() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        let paths = Paths {
            data: p.into(),
            definitions: p.join("definitionen"),
            results: p.join("auswertungen"),
            drafts: p.join("entwuerfe"),
            registry: p.join("registry"),
        };
        fs::create_dir(&paths.definitions).unwrap();
        fs::create_dir(&paths.registry).unwrap();
        let yaml = include_str!("../../examples/definitionen/beispiel.yaml");
        let file = paths.definitions.join("beispiel.yaml");
        fs::write(&file, yaml).unwrap();
        let hash = crate::definitions::scan(&paths.definitions)[0].hash.clone();
        let q = |hash: &str| QRef {
            id: "beispiel".into(),
            title: "Beispiel".into(),
            version: "1".into(),
            hash: hash.into(),
        };

        let same = with_current(&paths, (), &q(&hash));
        assert_eq!(same.current.unwrap().file, "beispiel.yaml");
        assert!(!same.outdated);

        let changed = with_current(&paths, (), &q("alt"));
        assert!(changed.current.is_some() && changed.outdated);

        fs::remove_file(&file).unwrap();
        let gone = with_current(&paths, (), &q(&hash));
        assert!(gone.current.is_none() && !gone.outdated);
    }

    #[test]
    fn locked_save_and_delete_are_refused() {
        assert_eq!(
            refuse_locked(true),
            Err("Fragebögen aus der Registry können nicht geändert werden.".into())
        );
        assert_eq!(refuse_locked(false), Ok(()));
    }
}
