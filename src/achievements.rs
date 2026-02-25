use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AchievementDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub trigger: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AchievementState {
    pub definition: AchievementDefinition,
    pub unlocked: bool,
}

#[derive(Clone, Debug)]
pub struct AchievementSnapshotItem {
    pub name: String,
    pub description: String,
    pub unlocked: bool,
}

#[derive(Clone, Debug)]
pub struct AchievementNotification {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AchievementRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub unlocked: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AchievementFileFormat {
    List(Vec<AchievementRecord>),
    WithRoot {
        achievements: Vec<AchievementRecord>,
    },
}

pub struct AchievementManager {
    achievements: Vec<AchievementState>,
    id_lookup: HashMap<String, usize>,
    trigger_lookup: HashMap<String, Vec<String>>,
    notifications: VecDeque<AchievementNotification>,
    dirty: bool,
}

impl AchievementManager {
    pub fn from_definitions(definitions: Vec<AchievementDefinition>) -> Result<Self, String> {
        let records = definitions
            .into_iter()
            .map(|definition| AchievementRecord {
                id: definition.id,
                name: definition.name,
                description: definition.description,
                trigger: definition.trigger,
                unlocked: false,
            })
            .collect();

        Self::from_records(records)
    }

    fn from_records(records: Vec<AchievementRecord>) -> Result<Self, String> {
        let mut achievements = Vec::with_capacity(records.len());
        let mut id_lookup = HashMap::with_capacity(records.len());
        let mut trigger_lookup: HashMap<String, Vec<String>> = HashMap::new();

        for record in records {
            let id = record.id.trim();
            if id.is_empty() {
                return Err("achievement id must not be empty".to_owned());
            }
            if id_lookup.contains_key(id) {
                return Err(format!("duplicate achievement id: {id}"));
            }

            let normalized = AchievementDefinition {
                id: id.to_owned(),
                name: record.name,
                description: record.description,
                trigger: record
                    .trigger
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty()),
            };

            if let Some(trigger) = normalized.trigger.as_deref() {
                trigger_lookup
                    .entry(trigger.to_owned())
                    .or_default()
                    .push(normalized.id.clone());
            }

            id_lookup.insert(normalized.id.clone(), achievements.len());
            achievements.push(AchievementState {
                definition: normalized,
                unlocked: record.unlocked,
            });
        }

        Ok(Self {
            achievements,
            id_lookup,
            trigger_lookup,
            notifications: VecDeque::new(),
            dirty: false,
        })
    }

    pub fn load_from_json_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .map_err(|err| format!("failed to read achievements file {}: {err}", path.display()))?;

        let parsed: AchievementFileFormat = serde_json::from_str(&raw).map_err(|err| {
            format!(
                "failed to parse achievements json {}: {err}",
                path.display()
            )
        })?;

        let records = match parsed {
            AchievementFileFormat::List(list) => list,
            AchievementFileFormat::WithRoot { achievements } => achievements,
        };

        Self::from_records(records)
    }

    pub fn snapshot(&self) -> Vec<AchievementSnapshotItem> {
        self.achievements
            .iter()
            .map(|entry| AchievementSnapshotItem {
                name: entry.definition.name.clone(),
                description: entry.definition.description.clone(),
                unlocked: entry.unlocked,
            })
            .collect()
    }

    pub fn is_unlocked(&self, achievement_id: &str) -> bool {
        let Some(index) = self.id_lookup.get(achievement_id).copied() else {
            return false;
        };

        self.achievements
            .get(index)
            .map(|entry| entry.unlocked)
            .unwrap_or(false)
    }

    pub fn trigger(&mut self, trigger_id: &str) -> Vec<String> {
        let Some(target_ids) = self.trigger_lookup.get(trigger_id).cloned() else {
            return Vec::new();
        };

        let mut unlocked_ids = Vec::new();
        for achievement_id in target_ids {
            if self.grant_internal(&achievement_id) {
                unlocked_ids.push(achievement_id);
            }
        }

        unlocked_ids
    }

    pub fn grant(&mut self, achievement_id: &str) -> Result<bool, String> {
        if !self.id_lookup.contains_key(achievement_id) {
            return Err(format!("achievement not found: {achievement_id}"));
        }

        Ok(self.grant_internal(achievement_id))
    }

    pub fn take_notifications(&mut self) -> Vec<AchievementNotification> {
        self.notifications.drain(..).collect()
    }

    pub fn save_to_json_file(&mut self, path: impl AsRef<Path>) -> Result<bool, String> {
        if !self.dirty {
            return Ok(false);
        }

        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create achievements directory {}: {err}",
                    parent.display()
                )
            })?;
        }

        let records: Vec<AchievementRecord> = self
            .achievements
            .iter()
            .map(|entry| AchievementRecord {
                id: entry.definition.id.clone(),
                name: entry.definition.name.clone(),
                description: entry.definition.description.clone(),
                trigger: entry.definition.trigger.clone(),
                unlocked: entry.unlocked,
            })
            .collect();

        let json = serde_json::to_string_pretty(&records)
            .map_err(|err| format!("failed to serialize achievements: {err}"))?;

        fs::write(path, json).map_err(|err| {
            format!(
                "failed to write achievements json {}: {err}",
                path.display()
            )
        })?;

        self.dirty = false;
        Ok(true)
    }

    fn grant_internal(&mut self, achievement_id: &str) -> bool {
        let Some(index) = self.id_lookup.get(achievement_id).copied() else {
            return false;
        };

        let Some(entry) = self.achievements.get_mut(index) else {
            return false;
        };

        if entry.unlocked {
            return false;
        }

        entry.unlocked = true;
        self.dirty = true;
        self.notifications.push_back(AchievementNotification {
            name: entry.definition.name.clone(),
            description: entry.definition.description.clone(),
        });

        true
    }
}
