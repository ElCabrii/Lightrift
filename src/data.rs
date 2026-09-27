use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub const PATCH: &str = include_str!("../assets/patch.txt");

#[derive(Clone, Deserialize)]
pub struct Sprite {
    pub sprite: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}
#[derive(Clone, Deserialize)]
pub struct Champion {
    pub id: String,
    pub key: String,
    pub name: String,
    pub title: String,
    pub tags: Vec<String>,
    pub blurb: String,
    pub image: Sprite,
    pub stats: Value,
}
#[derive(Clone, Deserialize)]
pub struct Item {
    pub name: String,
    pub description: String,
    pub image: Sprite,
    pub gold: Value,
    pub maps: BTreeMap<String, bool>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, rename = "inStore")]
    pub in_store: Option<bool>,
    #[serde(default, rename = "requiredChampion")]
    pub required_champion: Option<String>,
}
#[derive(Clone, Deserialize)]
pub struct Rune {
    pub id: u32,
    pub name: String,
    #[serde(rename = "shortDesc")]
    pub description: String,
}
#[derive(Clone, Deserialize)]
pub struct Slot {
    pub runes: Vec<Rune>,
}
#[derive(Clone, Deserialize)]
pub struct RuneTree {
    pub id: u32,
    pub name: String,
    pub slots: Vec<Slot>,
}
#[derive(Clone, Deserialize)]
pub struct Spell {
    pub key: String,
    pub name: String,
    pub description: String,
    pub image: Sprite,
    pub modes: Vec<String>,
}
pub struct Catalog {
    pub champions: Vec<Champion>,
    pub items: BTreeMap<u32, Item>,
    pub runes: Vec<RuneTree>,
    pub spells: Vec<Spell>,
}
impl Catalog {
    pub fn load() -> Self {
        let champs: Value = serde_json::from_str(include_str!("../assets/champion.json")).unwrap();
        let mut champions: Vec<Champion> =
            serde_json::from_value::<BTreeMap<String, Champion>>(champs["data"].clone())
                .unwrap()
                .into_values()
                .collect();
        champions.sort_by(|a, b| a.name.cmp(&b.name));
        let items: Value = serde_json::from_str(include_str!("../assets/item.json")).unwrap();
        let spells: Value = serde_json::from_str(include_str!("../assets/summoner.json")).unwrap();
        Self {
            champions,
            items: serde_json::from_value(items["data"].clone()).unwrap(),
            runes: serde_json::from_str(include_str!("../assets/runesReforged.json")).unwrap(),
            spells: serde_json::from_value::<BTreeMap<String, Spell>>(spells["data"].clone())
                .unwrap()
                .into_values()
                .filter(|s| s.modes.iter().any(|m| m == "CLASSIC"))
                .collect(),
        }
    }
    pub fn champion(&self, id: &str) -> Option<&Champion> {
        self.champions.iter().find(|c| c.id == id)
    }
    pub fn by_key(&self, key: i64) -> Option<&Champion> {
        self.champions
            .iter()
            .find(|c| c.key.parse::<i64>().ok() == Some(key))
    }
    pub fn tree(&self, id: u32) -> &RuneTree {
        self.runes
            .iter()
            .find(|t| t.id == id)
            .unwrap_or(&self.runes[0])
    }
    pub fn new_build(&self, id: &str) -> Build {
        let p = self.tree(8000);
        let s = self.tree(8400);
        Build {
            id: new_build_id(),
            champion: id.into(),
            name: "My build".into(),
            role: "Any role".into(),
            primary: p.id,
            secondary: s.id,
            primary_runes: p.slots.iter().map(|s| s.runes[0].id).collect(),
            secondary_runes: vec![s.slots[1].runes[0].id, s.slots[2].runes[0].id],
            shards: vec![5005, 5008, 5001],
            spells: vec![4, 12],
            starter: vec![],
            core: vec![],
            situational: vec![],
            notes: String::new(),
            skill_order: Vec::new(),
            patch: PATCH.trim().into(),
        }
    }
    pub fn validate(&self, b: &Build) -> Result<(), String> {
        if self.champion(&b.champion).is_none() {
            return Err("Unknown champion".into());
        }
        let p = self
            .runes
            .iter()
            .find(|t| t.id == b.primary)
            .ok_or("Unknown primary tree")?;
        let s = self
            .runes
            .iter()
            .find(|t| t.id == b.secondary)
            .ok_or("Unknown secondary tree")?;
        if p.id == s.id
            || b.primary_runes.len() != 4
            || b.secondary_runes.len() != 2
            || b.shards.len() != 3
        {
            return Err("Choose four primary runes, two secondary runes, and three shards.".into());
        }
        for (slot, id) in p.slots.iter().zip(&b.primary_runes) {
            if !slot.runes.iter().any(|r| r.id == *id) {
                return Err("A primary rune does not match its row.".into());
            }
        }
        let rows: BTreeSet<usize> = b
            .secondary_runes
            .iter()
            .filter_map(|id| {
                s.slots
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(_, slot)| slot.runes.iter().any(|r| r.id == *id))
                    .map(|(i, _)| i)
            })
            .collect();
        if rows.len() != 2 {
            return Err("Choose secondary runes from two different rows.".into());
        }
        for (i, id) in b.shards.iter().enumerate() {
            if !SHARDS[i].iter().any(|(n, _)| n == id) {
                return Err("Invalid stat shard".into());
            }
        }
        if b.spells.len() != 2
            || b.spells[0] == b.spells[1]
            || b.spells.iter().any(|id| {
                !self
                    .spells
                    .iter()
                    .any(|s| s.key.parse::<u32>().ok() == Some(*id))
            })
        {
            return Err("Choose two different summoner spells.".into());
        }
        for ids in [&b.starter, &b.core, &b.situational] {
            if ids.len() > 6
                || ids.iter().any(|id| {
                    !self
                        .items
                        .get(id)
                        .is_some_and(|it| available_item(*id, it, &b.champion))
                })
            {
                return Err("Build contains an unavailable item or too many items.".into());
            }
        }
        if b.name.trim().is_empty() || b.name.chars().count() > 60 {
            return Err("Build name must be 1–60 characters.".into());
        }
        Ok(())
    }
}
pub const SHARDS: [&[(u32, &str)]; 3] = [
    &[
        (5008, "Adaptive force"),
        (5005, "Attack speed"),
        (5007, "Ability haste"),
    ],
    &[
        (5008, "Adaptive force"),
        (5010, "Move speed"),
        (5001, "Scaling health"),
    ],
    &[
        (5011, "Health"),
        (5013, "Tenacity / slow resist"),
        (5001, "Scaling health"),
    ],
];
pub fn available_item(id: u32, item: &Item, champion: &str) -> bool {
    // The bundled catalog marks some alternate-mode variants as map 11 too.
    // This planner targets the standard item namespace, not those variants.
    id < 100_000
        && item.maps.get("11") == Some(&true)
        && item.gold["purchasable"].as_bool() == Some(true)
        && item.in_store != Some(false)
        && item
            .required_champion
            .as_ref()
            .is_none_or(|s| s == champion)
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Build {
    #[serde(default)]
    pub id: String,
    pub champion: String,
    pub name: String,
    pub role: String,
    pub primary: u32,
    pub secondary: u32,
    pub primary_runes: Vec<u32>,
    pub secondary_runes: Vec<u32>,
    pub shards: Vec<u32>,
    pub spells: Vec<u32>,
    pub starter: Vec<u32>,
    pub core: Vec<u32>,
    pub situational: Vec<u32>,
    pub notes: String,
    #[serde(default)]
    pub skill_order: Vec<String>,
    pub patch: String,
}
impl Build {
    pub fn overlay_skills(&self) -> Vec<&str> {
        // Read the exact format exported by 0.2 without altering old notes.
        let skills: Vec<_> = if self.skill_order.is_empty() {
            self.notes
                .split_once("Skill sequence: ")
                .and_then(|(_, tail)| tail.split_once('.'))
                .map(|(order, _)| order.split('→').map(str::trim).collect())
                .unwrap_or_default()
        } else {
            self.skill_order.iter().map(String::as_str).collect()
        };
        if skills.len() > 18 || skills.iter().any(|s| !matches!(*s, "Q" | "W" | "E" | "R")) {
            Vec::new()
        } else {
            skills
        }
    }
    pub fn rune_payload(&self) -> Value {
        json!({"name":format!("Lightrift: {} / {}",self.champion,self.name),"primaryStyleId":self.primary,"subStyleId":self.secondary,"selectedPerkIds":self.primary_runes.iter().chain(&self.secondary_runes).chain(&self.shards).copied().collect::<Vec<_>>(),"current":true})
    }
    pub fn item_set(&self, key: i64) -> Value {
        json!({"uid":format!("rift-{}",self.champion),"title":format!("Lightrift: {} / {}",self.champion,self.name),"type":"custom","map":"SR","mode":"CLASSIC","associatedChampions":[key],"associatedMaps":[11],"preferredItemSlots":[],"blocks":[{"type":"Starter","items":self.starter.iter().map(|id|json!({"id":id.to_string(),"count":1})).collect::<Vec<_>>()},{"type":"Core","items":self.core.iter().map(|id|json!({"id":id.to_string(),"count":1})).collect::<Vec<_>>()},{"type":"Situational","items":self.situational.iter().map(|id|json!({"id":id.to_string(),"count":1})).collect::<Vec<_>>()}]})
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub favorites: BTreeSet<String>,
    pub builds: Vec<Build>,
    pub league_path: String,
    pub compact: bool,
    pub active_builds: BTreeMap<String, String>,
    pub overlay: OverlaySettings,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlaySettings {
    pub opacity: u8,
    pub collapsed: bool,
    pub enabled: bool,
    pub items: bool,
    pub skills: bool,
    pub runes: bool,
}
impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            opacity: 92,
            collapsed: false,
            enabled: true,
            items: true,
            skills: true,
            runes: true,
        }
    }
}
pub fn new_build_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "{:x}-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}
impl Settings {
    pub fn remove_build(&mut self, id: &str) -> Option<Build> {
        let index = self.builds.iter().position(|b| b.id == id)?;
        let removed = self.builds.remove(index);
        self.active_builds.retain(|_, active| active != id);
        Some(removed)
    }
    pub fn migrate(&mut self) {
        let mut seen = BTreeSet::new();
        for b in &mut self.builds {
            if b.skill_order.is_empty() {
                b.skill_order = b.overlay_skills().into_iter().map(str::to_owned).collect();
            }
            if b.id.is_empty() || !seen.insert(b.id.clone()) {
                b.id = new_build_id();
                seen.insert(b.id.clone());
            }
        }
    }
    pub fn preferred_build(
        &self,
        champion: &str,
        role: Option<&str>,
        catalog: &Catalog,
    ) -> Option<&Build> {
        let valid = |b: &&Build| b.champion == champion && catalog.validate(b).is_ok();
        let remembered = self.active_builds.get(champion);
        self.builds
            .iter()
            .filter(valid)
            .find(|b| Some(&b.id) == remembered && role.is_none_or(|r| b.role == r))
            .or_else(|| role.and_then(|r| self.builds.iter().filter(valid).find(|b| b.role == r)))
            .or_else(|| {
                self.builds
                    .iter()
                    .filter(valid)
                    .find(|b| Some(&b.id) == remembered)
            })
            .or_else(|| self.builds.iter().find(valid))
    }
}
pub fn data_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let directory = exe.parent().unwrap_or(Path::new("."));
    resolve_data_dir(
        directory,
        directory.join("installed.flag").is_file(),
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .as_deref(),
    )
}
fn resolve_data_dir(directory: &Path, installed: bool, local: Option<&Path>) -> PathBuf {
    if installed && let Some(local) = local {
        return local.join("Lightrift");
    }
    directory.join("data")
}
pub fn load_settings() -> Result<Settings, String> {
    let path = data_dir().join("settings.json");
    if !path.exists() {
        return Ok(Settings::default());
    }
    let mut settings: Settings =
        serde_json::from_slice(&fs::read(path).map_err(|_| "Could not read settings")?)
            .map_err(|_| "Settings are damaged; the existing file has been preserved.")?;
    settings.migrate();
    Ok(settings)
}
pub fn save_settings(s: &Settings) -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let existing = dir.join("settings.json");
    let backup = dir.join("settings-before-0.4.json");
    if existing.exists() && !backup.exists() {
        fs::copy(&existing, &backup)
            .map_err(|e| format!("Could not back up existing settings: {e}"))?;
    }
    let tmp = dir.join("settings.tmp");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(tmp, dir.join("settings.json")).map_err(|e| e.to_string())
}
pub fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in s.chars() {
        match c {
            '<' => {
                inside = true;
                out.push(' ')
            }
            '>' => inside = false,
            _ => {
                if !inside {
                    out.push(c)
                }
            }
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn installed_storage_is_separate_from_portable_data_and_binaries() {
        let exe = std::path::Path::new("application");
        let local = std::path::Path::new("profile");
        assert_eq!(
            super::resolve_data_dir(exe, true, Some(local)),
            local.join("Lightrift")
        );
        assert_eq!(
            super::resolve_data_dir(exe, false, Some(local)),
            exe.join("data")
        );
        assert_eq!(super::resolve_data_dir(exe, true, None), exe.join("data"));
    }
    #[test]
    fn deletion_uses_id_and_clears_active_selection_across_reload() {
        let c = super::Catalog::load();
        let first = c.new_build("Jinx");
        let mut second = c.new_build("Jinx");
        second.name = first.name.clone();
        second.notes = "Keep this variant".into();
        let other = c.new_build("Ahri");
        let mut s = super::Settings::default();
        s.builds = vec![first.clone(), second.clone(), other.clone()];
        s.active_builds.insert("Jinx".into(), first.id.clone());
        s.active_builds.insert("Ahri".into(), other.id.clone());
        assert_eq!(s.remove_build(&first.id).unwrap().id, first.id);
        assert!(!s.active_builds.contains_key("Jinx"));
        assert_eq!(s.active_builds["Ahri"], other.id);
        let mut restored: super::Settings =
            serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        restored.migrate();
        assert_eq!(restored.builds.len(), 2);
        assert_eq!(
            restored.preferred_build("Jinx", None, &c).unwrap().notes,
            "Keep this variant"
        );
        assert!(restored.remove_build(&first.id).is_none());
        restored.remove_build(&second.id).unwrap();
        assert!(restored.preferred_build("Jinx", None, &c).is_none());
        assert_eq!(restored.builds[0].id, other.id);
    }
    #[test]
    fn legacy_overlay_sequence_migrates_without_changing_notes() {
        let c = super::Catalog::load();
        let mut value = serde_json::to_value(c.new_build("Jinx")).unwrap();
        value.as_object_mut().unwrap().remove("skill_order");
        value["notes"] = serde_json::json!(
            "Source: OP.GG. Skill sequence: Q → W → E → Q → Q → R. Boots are listed separately."
        );
        let mut settings: super::Settings =
            serde_json::from_value(serde_json::json!({"builds": [value.clone()]})).unwrap();
        settings.migrate();
        assert_eq!(
            settings.builds[0].overlay_skills(),
            ["Q", "W", "E", "Q", "Q", "R"]
        );
        assert_eq!(settings.builds[0].notes, value["notes"]);
        assert_eq!(settings.overlay.opacity, 92);
        let roundtrip: super::Settings =
            serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();
        assert_eq!(
            roundtrip.builds[0].skill_order,
            settings.builds[0].skill_order
        );
    }
    #[test]
    fn overlay_never_shifts_invalid_skill_levels() {
        let mut b = super::Catalog::load().new_build("Ahri");
        b.skill_order = vec!["Q".into(), "invalid".into(), "R".into()];
        assert!(b.overlay_skills().is_empty());
        b.skill_order = vec!["Q".into(); 19];
        assert!(b.overlay_skills().is_empty());
        b.skill_order.clear();
        b.notes = "My matchup notes: Q then W".into();
        assert!(b.overlay_skills().is_empty());
    }
    use super::*;
    #[test]
    fn legacy_builds_migrate_without_losing_content() {
        let catalog = Catalog::load();
        let mut legacy = serde_json::to_value(catalog.new_build("Ahri")).unwrap();
        legacy.as_object_mut().unwrap().remove("id");
        legacy["notes"] = json!("Keep my notes");
        let mut settings: Settings = serde_json::from_value(
            json!({"builds":[legacy.clone(),legacy],"league_path":"G:/League"}),
        )
        .unwrap();
        settings.migrate();
        assert!(!settings.builds[0].id.is_empty());
        assert_ne!(settings.builds[0].id, settings.builds[1].id);
        assert_eq!(settings.builds[0].notes, "Keep my notes");
        let ids: Vec<_> = settings.builds.iter().map(|b| b.id.clone()).collect();
        settings.migrate();
        assert_eq!(
            ids,
            settings
                .builds
                .iter()
                .map(|b| b.id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(settings.league_path, "G:/League");
    }
    #[test]
    fn draft_role_prefers_matching_build_over_other_remembered_role() {
        let c = Catalog::load();
        let mut mid = c.new_build("Ahri");
        mid.role = "Mid".into();
        let mut support = c.new_build("Ahri");
        support.role = "Support".into();
        let mut s = Settings::default();
        s.active_builds.insert("Ahri".into(), support.id.clone());
        s.builds = vec![mid.clone(), support.clone()];
        assert_eq!(
            s.preferred_build("Ahri", Some("Mid"), &c).unwrap().id,
            mid.id
        );
        assert_eq!(s.preferred_build("Ahri", None, &c).unwrap().id, support.id);
    }
    #[test]
    fn bundled_catalog_is_valid() {
        let c = Catalog::load();
        assert!(c.champions.len() > 150);
        for ch in &c.champions {
            assert!(c.validate(&c.new_build(&ch.id)).is_ok())
        }
    }
    #[test]
    fn secondary_runes_must_use_distinct_rows() {
        let c = Catalog::load();
        let mut b = c.new_build("Ahri");
        b.secondary_runes = vec![
            c.tree(b.secondary).slots[1].runes[0].id,
            c.tree(b.secondary).slots[1].runes[1].id,
        ];
        assert!(c.validate(&b).is_err())
    }
    #[test]
    fn item_set_is_champion_scoped_and_runes_complete() {
        let c = Catalog::load();
        let b = c.new_build("Jinx");
        assert_eq!(b.item_set(222)["associatedChampions"], json!([222]));
        assert_eq!(
            b.rune_payload()["selectedPerkIds"]
                .as_array()
                .unwrap()
                .len(),
            9
        )
    }
    #[test]
    fn rejects_injected_or_invalid_build_data() {
        let c = Catalog::load();
        let mut b = c.new_build("Ahri");
        b.core.push(999999);
        assert!(c.validate(&b).is_err());
        b.core.clear();
        b.shards[0] = 999999;
        assert!(c.validate(&b).is_err())
    }
    #[test]
    fn standard_planner_excludes_alternate_mode_variants() {
        let c = Catalog::load();
        assert!(available_item(8020, &c.items[&8020], "Jinx"));
        assert!(!available_item(328020, &c.items[&328020], "Jinx"));
        let mut b = c.new_build("Jinx");
        b.core.push(328020);
        assert!(c.validate(&b).is_err());
    }
}
