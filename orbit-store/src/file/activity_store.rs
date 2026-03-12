use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use orbit_types::{Activity, OrbitError};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub(crate) struct ActivityFileStore {
    root: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct FileWorkInsert {
    pub id: String,
    pub spec_type: String,
    pub description: String,
    pub instruction: String,
    pub input_schema_json: serde_json::Value,
    pub output_schema_json: serde_json::Value,
    pub artifact_path_template: Option<String>,
    pub skill_refs: Vec<String>,
    pub identity_id: Option<String>,
    pub assigned_to: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityFileDocument {
    schema_version: u8,
    id: String,
    spec_type: String,
    description: String,
    #[serde(default)]
    instruction: String,
    input_schema_json: serde_json::Value,
    output_schema_json: serde_json::Value,
    artifact_path_template: Option<String>,
    skill_refs: Vec<String>,
    #[serde(default)]
    identity_id: Option<String>,
    #[serde(default)]
    assigned_to: Option<String>,
    #[serde(default)]
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ActivityFileStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn ensure_layout(&self) -> Result<(), OrbitError> {
        fs::create_dir_all(self.active_dir()).map_err(|e| OrbitError::Io(e.to_string()))?;
        fs::create_dir_all(self.inactive_dir()).map_err(|e| OrbitError::Io(e.to_string()))?;
        Ok(())
    }

    pub(crate) fn insert_work(&self, params: &FileWorkInsert) -> Result<Activity, OrbitError> {
        self.ensure_layout()?;
        if self.get_activity(&params.id)?.is_some() {
            return Err(OrbitError::InvalidInput(format!(
                "activity already exists: {}",
                params.id
            )));
        }

        let now = Utc::now();
        let doc = ActivityFileDocument {
            schema_version: 1,
            id: params.id.clone(),
            spec_type: params.spec_type.clone(),
            description: params.description.clone(),
            instruction: params.instruction.clone(),
            input_schema_json: params.input_schema_json.clone(),
            output_schema_json: params.output_schema_json.clone(),
            artifact_path_template: params.artifact_path_template.clone(),
            skill_refs: params.skill_refs.clone(),
            identity_id: params.identity_id.clone(),
            assigned_to: params.assigned_to.clone(),
            created_by: params.created_by.clone(),
            created_at: now,
            updated_at: now,
        };
        self.write_doc_at(&self.active_doc_path(&doc.id), &doc)?;
        Ok(doc_to_work(doc, true))
    }

    pub(crate) fn list_activities(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<Activity>, OrbitError> {
        let mut activities = self.list_dir_docs(&self.active_dir(), true)?;
        if include_inactive {
            activities.extend(self.list_dir_docs(&self.inactive_dir(), false)?);
        }
        activities.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(activities)
    }

    pub(crate) fn get_activity(&self, id: &str) -> Result<Option<Activity>, OrbitError> {
        let active = self.active_doc_path(id);
        if active.exists() {
            let doc = self.read_doc_at(&active)?;
            return Ok(Some(doc_to_work(doc, true)));
        }
        let inactive = self.inactive_doc_path(id);
        if inactive.exists() {
            let doc = self.read_doc_at(&inactive)?;
            return Ok(Some(doc_to_work(doc, false)));
        }
        Ok(None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_activity(
        &self,
        id: &str,
        description: Option<String>,
        instruction: Option<String>,
        input_schema_json: Option<serde_json::Value>,
        output_schema_json: Option<serde_json::Value>,
        artifact_path_template: Option<Option<String>>,
        skill_refs: Option<Vec<String>>,
        identity_id: Option<Option<String>>,
        assigned_to: Option<Option<String>>,
        is_active: Option<bool>,
    ) -> Result<Activity, OrbitError> {
        self.ensure_layout()?;
        let (path, current_active) = if self.active_doc_path(id).exists() {
            (self.active_doc_path(id), true)
        } else if self.inactive_doc_path(id).exists() {
            (self.inactive_doc_path(id), false)
        } else {
            return Err(OrbitError::InvalidInput(format!(
                "activity not found: {id}"
            )));
        };
        let mut doc = self.read_doc_at(&path)?;
        if let Some(v) = description {
            doc.description = v;
        }
        if let Some(v) = instruction {
            doc.instruction = v;
        }
        if let Some(v) = input_schema_json {
            doc.input_schema_json = v;
        }
        if let Some(v) = output_schema_json {
            doc.output_schema_json = v;
        }
        if let Some(v) = artifact_path_template {
            doc.artifact_path_template = v;
        }
        if let Some(v) = skill_refs {
            doc.skill_refs = v;
        }
        if let Some(v) = identity_id {
            doc.identity_id = v;
        }
        if let Some(v) = assigned_to {
            doc.assigned_to = v;
        }
        doc.updated_at = Utc::now();

        let new_active = is_active.unwrap_or(current_active);
        if new_active != current_active {
            // Move the file to the new location.
            let new_path = if new_active {
                self.active_doc_path(id)
            } else {
                self.inactive_doc_path(id)
            };
            self.write_doc_at(&new_path, &doc)?;
            fs::remove_file(&path).map_err(|e| OrbitError::Io(e.to_string()))?;
            return Ok(doc_to_work(doc, new_active));
        }
        self.write_doc_at(&path, &doc)?;
        Ok(doc_to_work(doc, new_active))
    }

    pub(crate) fn disable_activity(&self, id: &str) -> Result<bool, OrbitError> {
        self.ensure_layout()?;
        let active = self.active_doc_path(id);
        if active.exists() {
            let mut doc = self.read_doc_at(&active)?;
            doc.updated_at = Utc::now();
            let inactive = self.inactive_doc_path(id);
            self.write_doc_at(&inactive, &doc)?;
            fs::remove_file(&active).map_err(|e| OrbitError::Io(e.to_string()))?;
            return Ok(true);
        }

        let inactive = self.inactive_doc_path(id);
        if inactive.exists() {
            let mut doc = self.read_doc_at(&inactive)?;
            doc.updated_at = Utc::now();
            self.write_doc_at(&inactive, &doc)?;
            return Ok(true);
        }

        Ok(false)
    }

    fn list_dir_docs(&self, dir: &Path, active: bool) -> Result<Vec<Activity>, OrbitError> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut paths = fs::read_dir(dir)
            .map_err(|e| OrbitError::Io(e.to_string()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| is_yaml(path))
            .collect::<Vec<_>>();
        paths.sort();

        let mut activities = Vec::new();
        for path in paths {
            let doc = self.read_doc_at(&path)?;
            activities.push(doc_to_work(doc, active));
        }
        Ok(activities)
    }

    fn read_doc_at(&self, path: &Path) -> Result<ActivityFileDocument, OrbitError> {
        let raw = fs::read_to_string(path).map_err(|e| OrbitError::Io(e.to_string()))?;
        serde_yaml::from_str::<ActivityFileDocument>(&raw).map_err(|e| {
            OrbitError::Store(format!("invalid activity file '{}': {e}", path.display()))
        })
    }

    fn write_doc_at(&self, path: &Path, doc: &ActivityFileDocument) -> Result<(), OrbitError> {
        let yaml = serde_yaml::to_string(doc).map_err(|e| OrbitError::Store(e.to_string()))?;
        write_atomic(path, &yaml)
    }

    fn active_doc_path(&self, id: &str) -> PathBuf {
        self.active_dir().join(format!("{id}.yaml"))
    }

    fn inactive_doc_path(&self, id: &str) -> PathBuf {
        self.inactive_dir().join(format!("{id}.yaml"))
    }

    fn active_dir(&self) -> PathBuf {
        self.root.join("active")
    }

    fn inactive_dir(&self) -> PathBuf {
        self.root.join("inactive")
    }
}

fn doc_to_work(doc: ActivityFileDocument, is_active: bool) -> Activity {
    Activity {
        id: doc.id,
        spec_type: doc.spec_type,
        description: doc.description,
        instruction: doc.instruction,
        input_schema_json: doc.input_schema_json,
        output_schema_json: doc.output_schema_json,
        artifact_path_template: doc.artifact_path_template,
        skill_refs: doc.skill_refs,
        identity_id: doc.identity_id,
        assigned_to: doc.assigned_to,
        created_by: doc.created_by,
        is_active,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }
}

fn write_atomic(path: &Path, content: &str) -> Result<(), OrbitError> {
    let parent = path.parent().ok_or_else(|| {
        OrbitError::Io(format!("cannot determine parent for '{}'", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|e| OrbitError::Io(e.to_string()))?;

    let mut tmp = path.to_path_buf();
    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    tmp.set_extension(format!("yaml.tmp.{nanos}"));
    fs::write(&tmp, content).map_err(|e| OrbitError::Io(e.to_string()))?;
    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(OrbitError::Io(err.to_string()));
    }
    Ok(())
}

fn is_yaml(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
}
