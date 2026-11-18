use crate::{
    Paths,
    definitions::{self, DefinitionEntry, OpenedDefinition, Summary},
    editor,
    registry::{self, Connection, InstallOutcome, Progress, RowsResult},
    report::{self, Report},
    store::{self, Draft, DraftSummary, QRef, SavedRecord, SavedSummary},
};
use kreuzchen_engine::{
    Grading, Input,
    schema::{Definition, Norms},
};
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

/// A saved result as `Opened`, plus the norm tables of its definition for the PDF profile.
#[derive(Serialize)]
pub struct OpenedResult {
    #[serde(flatten)]
    pub opened: Opened<SavedRecord>,
    /// `None` when the definition is gone, changed since grading, or has no norms.
    pub norms: Option<Norms>,
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
    name: Option<String>,
    draft: Option<String>,
) -> Result<String, String> {
    store::save_finished(
        &paths.results,
        &paths.drafts,
        &record,
        name.as_deref(),
        draft.as_deref(),
    )
}

#[tauri::command]
pub fn list_results(paths: State<Paths>) -> Vec<SavedSummary> {
    store::list(&paths.results)
}

#[tauri::command]
pub fn open_result(paths: State<Paths>, name: String) -> Result<OpenedResult, String> {
    open_saved(&paths, &name)
}

fn open_saved(paths: &Paths, name: &str) -> Result<OpenedResult, String> {
    let record: SavedRecord = store::load(&paths.results, name)?;
    let q = record.questionnaire.clone();
    let opened = with_current(paths, record, &q);
    // Only the version that graded it: other tables would put the saved values on a foreign axis.
    let norms = opened
        .current
        .as_ref()
        .filter(|_| !opened.outdated)
        .and_then(|c| definitions::open(paths.dir(c.locked), &c.file).ok())
        .and_then(|(q, _)| q.def.norms);
    Ok(OpenedResult { opened, norms })
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
// `dest` comes from the OS save dialog.
pub fn export_pdf(report: Report, dest: String) -> Result<(), String> {
    std::fs::write(dest, report::pdf(&report))
        .map_err(|e| format!("PDF konnte nicht gespeichert werden: {e}"))
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
    on_progress: tauri::ipc::Channel<Progress>,
) -> Result<InstallOutcome, String> {
    let c = registry::load_connection(&paths.data).ok_or("Keine Registry verbunden.")?;
    // A closed channel only loses the progress display, never the install.
    registry::install(&c, &paths, &id, replace_local, |p| {
        let _ = on_progress.send(p);
    })
    .await
}

#[tauri::command]
pub fn registry_remove(paths: State<Paths>, file: String) -> Result<(), String> {
    registry::remove(&paths.registry, &file)
}

#[cfg(test)]
mod tests {
    use super::{definitions, open_saved, refuse_locked, with_current};
    use crate::{
        Paths,
        store::{self, QRef},
    };
    use kreuzchen_engine::Input;
    use std::{collections::BTreeMap, fs};

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
    fn saved_result_brings_the_norms_of_the_unchanged_definition() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths {
            data: tmp.path().into(),
            definitions: tmp.path().join("definitionen"),
            results: tmp.path().join("auswertungen"),
            drafts: tmp.path().join("entwuerfe"),
            registry: tmp.path().join("registry"),
        };
        for dir in [&paths.definitions, &paths.results, &paths.registry] {
            fs::create_dir_all(dir).unwrap();
        }
        let yaml = include_str!("../../examples/definitionen/beispiel_normen.yaml");
        let def = paths.definitions.join("beispiel_normen.yaml");
        fs::write(&def, yaml).unwrap();
        let q = kreuzchen_engine::load(yaml).unwrap();
        let input = Input {
            fields: BTreeMap::new(),
            answers: BTreeMap::new(),
        };
        let (_, hash) = definitions::open(&paths.definitions, "beispiel_normen.yaml").unwrap();
        let save = |hash: &str| {
            let record: store::SavedRecord = serde_json::from_value(serde_json::json!({
                "format": 1, "kennung": "K", "created_at": "2026-11-17T14:30:00",
                "questionnaire": { "id": "beispiel_normen", "title": "B", "version": "1", "hash": hash },
                "input": input, "grading": q.grade(&input).unwrap(), "text": "",
            }))
            .unwrap();
            store::save(&paths.results, &record).unwrap()
        };
        let same = save(&hash);
        let changed = save("alt");

        assert_eq!(open_saved(&paths, &same).unwrap().norms, q.def.norms);
        // Saved values come from the old tables; the new ones must not frame them.
        let opened = open_saved(&paths, &changed).unwrap();
        assert!(opened.opened.outdated);
        assert_eq!(opened.norms, None);

        fs::remove_file(def).unwrap();
        assert_eq!(open_saved(&paths, &same).unwrap().norms, None);
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
