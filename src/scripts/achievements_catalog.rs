use std::{fs, path::Path};

use crate::achievements::AchievementDefinition;

pub const DEFAULT_ACHIEVEMENTS_PATH: &str = "src/data/achievements.json";

pub fn create_all_achievements() -> Vec<AchievementDefinition> {
    vec![
        AchievementDefinition {
            id: "first_launch".to_owned(),
            name: "Первый запуск".to_owned(),
            description: "Запустить игру и перейти в игровой режим.".to_owned(),
            trigger: Some("game_started".to_owned()),
        },
        AchievementDefinition {
            id: "intro_closed".to_owned(),
            name: "Диалог завершён".to_owned(),
            description: "Закрыть стартовый диалог персонажа.".to_owned(),
            trigger: Some("intro_closed".to_owned()),
        },
        AchievementDefinition {
            id: "intro_skipped".to_owned(),
            name: "Быстрый читатель".to_owned(),
            description: "Закрыть стартовый диалог по сигналу SkipWait.".to_owned(),
            trigger: Some("intro_skipped".to_owned()),
        },
        AchievementDefinition {
            id: "script_reward".to_owned(),
            name: "Скриптовая награда".to_owned(),
            description: "Достижение выдано напрямую из скрипта.".to_owned(),
            trigger: None,
        },
    ]
}

pub fn write_achievements_json(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create achievements directory {}: {err}",
                parent.display()
            )
        })?;
    }

    #[derive(serde::Serialize)]
    struct AchievementFileEntry {
        id: String,
        name: String,
        description: String,
        trigger: Option<String>,
        unlocked: bool,
    }

    let entries: Vec<AchievementFileEntry> = create_all_achievements()
        .into_iter()
        .map(|definition| AchievementFileEntry {
            id: definition.id,
            name: definition.name,
            description: definition.description,
            trigger: definition.trigger,
            unlocked: false,
        })
        .collect();

    let json = serde_json::to_string_pretty(&entries)
        .map_err(|err| format!("failed to serialize achievements: {err}"))?;

    fs::write(path, json).map_err(|err| {
        format!(
            "failed to write achievements json {}: {err}",
            path.display()
        )
    })
}

pub fn ensure_achievements_json_exists(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }

    write_achievements_json(path)
}
