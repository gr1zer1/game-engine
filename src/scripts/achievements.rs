use crate::achievements::AchievementManager;

pub fn trigger(manager: &mut AchievementManager, trigger_id: &str) -> Vec<String> {
    manager.trigger(trigger_id)
}

pub fn grant(manager: &mut AchievementManager, achievement_id: &str) -> bool {
    match manager.grant(achievement_id) {
        Ok(is_new) => is_new,
        Err(err) => {
            eprintln!("achievement grant failed: {err}");
            false
        }
    }
}

pub fn is_unlocked(manager: &AchievementManager, achievement_id: &str) -> bool {
    manager.is_unlocked(achievement_id)
}
