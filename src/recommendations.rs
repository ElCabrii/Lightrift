use crate::data::{Build, Catalog, PATCH, data_dir};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const ENDPOINT: &str = "https://mcp-api.op.gg/mcp";
const LIMIT: u64 = 1024 * 1024;
mod core_options;
pub use core_options::CoreOption;
pub const ROLES: &[(&str, &str)] = &[
    ("Top", "top"),
    ("Jungle", "jungle"),
    ("Mid", "mid"),
    ("Bottom", "adc"),
    ("Support", "support"),
];
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Clone)]
pub struct Request {
    pub champion: String,
    pub name: String,
    pub role: String,
    pub force: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub champion: String,
    pub role: String,
    pub fetched_at: u64,
    pub data: Value,
    #[serde(default)]
    pub core_options: Vec<CoreOption>,
    #[serde(default)]
    pub options_checked: bool,
    #[serde(skip)]
    pub notice: String,
}
impl Recommendation {
    pub fn to_build_option(
        &self,
        catalog: &Catalog,
        selected: Option<usize>,
    ) -> Result<Build, String> {
        let mut build = self.to_build(catalog)?;
        if let Some(index) = selected {
            let option = self
                .core_options
                .get(index)
                .ok_or("Choose an available core build.")?;
            build.core = option.ids.clone();
            build.name = format!("OP.GG {} {} · Core {}", self.role, self.patch(), index + 1);
            build.notes.push_str(&format!(" Core option {}: OP.GG public champion page, Global, All ranks, patch {}. Core-only sample: {} games, {:.2}% win rate, {:.2}% pick rate. Other components use the shared service recommendation; these are not combined-loadout statistics.", index + 1, self.patch(), option.games, option.win_rate * 100.0, option.pick_rate * 100.0));
            catalog.validate(&build)?;
        }
        Ok(build)
    }
    pub fn patch(&self) -> &str {
        self.data["trends"]["win"]["version"]
            .as_str()
            .unwrap_or("Unknown")
    }
    pub fn to_build(&self, catalog: &Catalog) -> Result<Build, String> {
        let patch = PATCH
            .trim()
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".");
        if self.patch() != patch {
            return Err(format!(
                "Recommendation patch {} differs from bundled {}. View it as reference until the catalog is updated.",
                self.patch(),
                patch
            ));
        }
        let mut b = catalog.new_build(&self.champion);
        b.name = format!("OP.GG {} {}", self.role, self.patch());
        b.role = self.role.clone();
        let r = &self.data["runes"];
        b.primary = number(&r["primary_page_id"])?;
        b.secondary = number(&r["secondary_page_id"])?;
        b.primary_runes = numbers(&r["primary_rune_ids"])?;
        b.secondary_runes = numbers(&r["secondary_rune_ids"])?;
        b.shards = numbers(&r["stat_mod_ids"])?;
        b.spells = numbers(&self.data["summoner_spells"]["ids"])?;
        b.starter = numbers(&self.data["starter_items"]["ids"])?;
        b.core = numbers(&self.data["core_items"]["ids"])?;
        // Keep source ordering for the core; boots are a separate situational block entry.
        b.situational = numbers(&self.data["boots"]["ids"])?;
        // Never skip invalid entries: that would shift the displayed level numbers.
        b.skill_order = self.data["skills"]["order"]
            .as_array()
            .and_then(|a| {
                a.iter()
                    .map(|v| v.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
            })
            .unwrap_or_default();
        if b.overlay_skills().is_empty() {
            b.skill_order.clear();
        }
        let skills = b.skill_order.join(" → ");
        b.notes = format!(
            "Source: OP.GG public champion analysis · Ranked · {} · patch {}. Retrieved at Unix time {}. Region not specified by provider. Skill sequence: {}. Boots are listed in Situational; adjust purchase timing for the match.",
            self.role,
            self.patch(),
            self.fetched_at,
            skills
        );
        catalog.validate(&b)?;
        Ok(b)
    }
}
fn number(v: &Value) -> Result<u32, String> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("Provider returned an invalid identifier.".into())
}
pub fn numbers(v: &Value) -> Result<Vec<u32>, String> {
    v.as_array()
        .ok_or("Provider did not supply this build component.".to_string())?
        .iter()
        .map(number)
        .collect()
}

pub type Reply = (String, String, Result<Recommendation, String>);
pub fn worker(ctx: eframe::egui::Context) -> (mpsc::Sender<Request>, mpsc::Receiver<Reply>) {
    let (tx, rx) = mpsc::channel::<Request>();
    let (out, replies) = mpsc::channel();
    std::thread::spawn(move || {
        let http = Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build();
        for request in rx {
            let result = http
                .as_ref()
                .map_err(|_| "Could not create provider connection.".into())
                .and_then(|h| fetch(h, &request));
            if out.send((request.champion, request.role, result)).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    (tx, replies)
}
fn fetch(http: &Client, request: &Request) -> Result<Recommendation, String> {
    if !request.champion.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid champion.".into());
    }
    let position = ROLES
        .iter()
        .find(|(r, _)| *r == request.role)
        .ok_or("Choose a lane first.")?
        .1;
    let cache_dir = data_dir().join("recommendations");
    let path = cache_dir.join(format!("{}-{position}.json", request.champion));
    let cached: Option<Recommendation> = fs::read(&path)
        .ok()
        .filter(|b| b.len() <= LIMIT as usize)
        .and_then(|b| serde_json::from_slice::<Recommendation>(&b).ok())
        .filter(|c| c.champion == request.champion && c.role == request.role);
    if !request.force
        && let Some(c) = &cached
        && now().saturating_sub(c.fetched_at) < 6 * 3600
        && c.options_checked
    {
        return Ok(c.clone());
    }
    match online(http, request, position) {
        Ok(value) => {
            if fs::create_dir_all(&cache_dir).is_ok()
                && let Ok(bytes) = serde_json::to_vec(&value)
            {
                let tmp = path.with_extension("tmp");
                if fs::write(&tmp, bytes).is_ok() {
                    let _ = fs::rename(tmp, &path);
                }
            }
            Ok(value)
        }
        Err(e) => {
            if let Some(mut c) = cached {
                c.notice = format!("{e} Showing previously downloaded data.");
                Ok(c)
            } else {
                Err(e)
            }
        }
    }
}
fn online(http: &Client, request: &Request, position: &str) -> Result<Recommendation, String> {
    let name = request
        .name
        .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
        .to_uppercase();
    let fields = [
        "champion",
        "position",
        "data.summary.average_stats.{win_rate,pick_rate,play}",
        "data.core_items.{ids[],pick_rate,play,win}",
        "data.starter_items.{ids[],pick_rate,play,win}",
        "data.boots.{ids[],pick_rate,play,win}",
        "data.runes.{primary_page_id,primary_rune_ids[],secondary_page_id,secondary_rune_ids[],stat_mod_ids[],pick_rate,play,win}",
        "data.summoner_spells.{ids[],pick_rate,play,win}",
        "data.skills.{order[],pick_rate,play,win}",
        "data.trends.win.{created_at,version}",
    ];
    let response = http.post(ENDPOINT).header("Accept", "application/json, text/event-stream")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lol_get_champion_analysis","arguments":{"champion":name,"position":position,"game_mode":"ranked","lang":"en_US","desired_output_fields":fields}}}))
        .send().map_err(|_| "OP.GG could not be reached. Try again later.")?;
    if !response.status().is_success() {
        return Err(format!(
            "OP.GG returned HTTP {}. Try again later.",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read provider response.")?;
    if bytes.len() > LIMIT as usize {
        return Err("Provider response was too large.".into());
    }
    let rpc: Value =
        serde_json::from_slice(&bytes).map_err(|_| "Provider response format is unsupported.")?;
    let parsed = decode_rpc(&rpc)?;
    let returned = canonical_champion(parsed["champion"].as_str().unwrap_or(""));
    if (returned != canonical_champion(&name) && returned != canonical_champion(&request.champion))
        || parsed["position"].as_str() != Some(&position.to_uppercase())
    {
        return Err("Provider returned a different champion or lane.".into());
    }
    if !parsed["data"]["summary"]["average_stats"].is_object() {
        return Err("No statistics available for this champion and lane.".into());
    }
    let (core_options, notice) = match core_options::fetch(
        http,
        request,
        position,
        parsed["data"]["trends"]["win"]["version"]
            .as_str()
            .unwrap_or(""),
    ) {
        Ok(options) => (options, String::new()),
        Err(e) => (
            Vec::new(),
            format!(
                "Core alternatives unavailable: {e} The service recommendation is still available."
            ),
        ),
    };
    Ok(Recommendation {
        champion: request.champion.clone(),
        role: request.role.clone(),
        fetched_at: now(),
        data: parsed["data"].clone(),
        notice,
        core_options,
        options_checked: true,
    })
}
fn canonical_champion(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}
pub fn decode_rpc(rpc: &Value) -> Result<Value, String> {
    if rpc.get("error").is_some() || rpc["result"]["isError"] == true {
        return Err("Provider could not supply this champion and lane.".into());
    }
    let text = rpc["result"]["content"]
        .as_array()
        .and_then(|a| a.iter().find_map(|c| c["text"].as_str()))
        .ok_or("Provider response was empty.")?;
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Ok(value);
    }
    decode_compact(text)
}
// The documented service currently emits class declarations and positional records.
// Parse that bounded data grammar; never evaluate provider text as code.
fn decode_compact(text: &str) -> Result<Value, String> {
    let mut schemas = BTreeMap::new();
    let mut record = Vec::new();
    for line in text.lines() {
        if let Some(line) = line.strip_prefix("class ") {
            let (name, fields) = line.split_once(':').ok_or("Invalid provider schema.")?;
            if schemas
                .insert(
                    name.trim().to_string(),
                    fields
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<_>>(),
                )
                .is_some()
            {
                return Err("Duplicate provider schema.".into());
            }
        } else if !line.trim().is_empty() {
            record.push(line);
        }
    }
    let record = record.join("\n");
    let mut parser = Parser {
        text: &record,
        pos: 0,
        schemas,
    };
    let value = parser.value(0)?;
    parser.space();
    if parser.pos != record.len() {
        return Err("Trailing provider data.".into());
    }
    Ok(value)
}
struct Parser<'a> {
    text: &'a str,
    pos: usize,
    schemas: BTreeMap<String, Vec<String>>,
}
impl Parser<'_> {
    fn space(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }
    fn consume(&mut self, byte: u8) -> bool {
        self.space();
        if self.text.as_bytes().get(self.pos) == Some(&byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self, depth: u32) -> Result<Value, String> {
        if depth > 32 {
            return Err("Provider data is too deeply nested.".into());
        }
        self.space();
        let start = self.pos;
        let first = *self
            .text
            .as_bytes()
            .get(self.pos)
            .ok_or("Truncated provider data.")?;
        if first == b'"' {
            self.pos += 1;
            let mut escaped = false;
            while let Some(&b) = self.text.as_bytes().get(self.pos) {
                self.pos += 1;
                if !escaped && b == b'"' {
                    return serde_json::from_str(&self.text[start..self.pos])
                        .map_err(|_| "Invalid provider string.".into());
                }
                escaped = !escaped && b == b'\\';
            }
            return Err("Unterminated provider string.".into());
        }
        if self.consume(b'[') {
            return Ok(Value::Array(self.sequence(b']', depth)?));
        }
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_alphanumeric() || b"_-.+".contains(b))
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err("Unexpected provider token.".into());
        }
        let token = &self.text[start..self.pos];
        if self.consume(b'(') {
            let fields = self
                .schemas
                .get(token)
                .cloned()
                .ok_or("Unknown provider record.")?;
            let values = self.sequence(b')', depth)?;
            if fields.len() != values.len() {
                return Err("Provider record fields do not match.".into());
            }
            return Ok(Value::Object(fields.into_iter().zip(values).collect()));
        }
        serde_json::from_str(token).map_err(|_| "Invalid provider value.".into())
    }
    fn sequence(&mut self, end: u8, depth: u32) -> Result<Vec<Value>, String> {
        let mut values = vec![];
        if self.consume(end) {
            return Ok(values);
        }
        loop {
            values.push(self.value(depth + 1)?);
            if self.consume(end) {
                return Ok(values);
            }
            if !self.consume(b',') {
                return Err("Invalid provider separator.".into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_response_converts_only_compatible_complete_builds() {
        let rpc: Value =
            serde_json::from_str(include_str!("../tests/fixtures/opgg-jinx.json")).unwrap();
        let decoded = decode_rpc(&rpc).unwrap();
        let c = Catalog::load();
        let mut r = Recommendation {
            champion: "Jinx".into(),
            role: "Bottom".into(),
            fetched_at: 1,
            data: decoded["data"].clone(),
            notice: String::new(),
            core_options: Vec::new(),
            options_checked: false,
        };
        let b = r.to_build(&c).unwrap();
        assert_eq!(b.core, vec![2523, 3085, 3031]);
        assert_eq!(b.primary_runes[0], 8008);
        assert_eq!(b.spells, vec![4, 21]);
        r.core_options.push(CoreOption {
            ids: vec![3031, 3085, 2523],
            games: 1000,
            pick_rate: 0.1,
            win_rate: 0.5,
        });
        let alternative = r.to_build_option(&c, Some(0)).unwrap();
        assert_eq!(alternative.core, [3031, 3085, 2523]);
        assert_eq!(alternative.primary_runes, b.primary_runes);
        assert_eq!(alternative.spells, b.spells);
        assert_ne!(alternative.id, b.id);
        assert!(alternative.name.contains("Core 1"));
        assert!(r.to_build_option(&c, Some(1)).is_err());
        r.core_options[0].ids[0] = 999999;
        assert!(r.to_build_option(&c, Some(0)).is_err());
        r.data["trends"]["win"]["version"] = json!("0.0");
        assert!(r.to_build(&c).is_err());
        r.data["trends"]["win"]["version"] = json!("16.19");
        r.data["runes"]["stat_mod_ids"] = json!([999999, 5008, 5011]);
        assert!(r.to_build(&c).is_err());
    }
    #[test]
    fn provider_champion_names_ignore_punctuation_only() {
        assert_eq!(canonical_champion("KAI_SA"), canonical_champion("Kai'Sa"));
        assert_ne!(canonical_champion("Jinx"), canonical_champion("Jhin"));
    }
    #[test]
    #[ignore = "requires public network access to OP.GG"]
    fn live_provider_builds_validate_without_a_riot_key() {
        let c = Catalog::load();
        let http = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap();
        for (champion, role) in [("Jinx", "Bottom"), ("Ahri", "Mid"), ("Kaisa", "Bottom")] {
            let name = c.champion(champion).unwrap().name.clone();
            let position = ROLES.iter().find(|(r, _)| *r == role).unwrap().1;
            let result = online(
                &http,
                &Request {
                    champion: champion.into(),
                    name,
                    role: role.into(),
                    force: true,
                },
                position,
            )
            .expect("live provider response");
            result
                .to_build(&c)
                .expect("recommendation matches current catalog");
            assert!(
                !result.core_options.is_empty(),
                "{champion}: {}",
                result.notice
            );
            for index in 0..result.core_options.len() {
                result
                    .to_build_option(&c, Some(index))
                    .expect("core option matches current catalog");
            }
            println!(
                "{champion}: patch {}, {} core builds validated",
                result.patch(),
                result.core_options.len()
            );
        }
    }
    #[test]
    fn compact_records_support_arrays_strings_and_reject_code() {
        assert_eq!(
            decode_compact("class A: ids,name\nA([1,2],\"a\\\"b\")").unwrap(),
            json!({"ids":[1,2],"name":"a\"b"})
        );
        for bad in [
            "class A: x\nA(1,2)",
            "class A: x\nA(1);run()",
            "class A: x\nUnknown(1)",
            "class A: x\nA([1,)",
        ] {
            assert!(decode_compact(bad).is_err());
        }
    }
    #[test]
    fn provider_error_is_not_a_recommendation() {
        assert!(
            decode_rpc(&json!({"result":{"isError":true,"content":[{"text":"failed"}]}})).is_err()
        );
    }
}
