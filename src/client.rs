use crate::data::{Build, Catalog, PATCH, data_dir};
use reqwest::{Method, blocking::Client};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

pub enum Command {
    Refresh(String, bool),
    Detail(String),
    Runes(String, Build),
    SaveRunes(String, Build),
    Items(String, Build),
    Spells(String, Build),
}
pub enum Event {
    Snapshot(Result<Snapshot, String>),
    Detail(String, Result<Value, String>),
    Applied(Result<String, String>),
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub phase: String,
    pub display_name: String,
    pub draft: Value,
    pub game: Option<crate::live::Game>,
    pub game_error: String,
}
struct Lock {
    port: u16,
    password: String,
}
fn parse_lock(s: &str) -> Result<Lock, String> {
    let fields: Vec<_> = s.trim().split(':').collect();
    if fields.len() != 5
        || fields[0] != "LeagueClient"
        || fields[4] != "https"
        || fields[3].is_empty()
    {
        return Err("The League lockfile is invalid.".into());
    }
    let port = fields[2]
        .parse::<u16>()
        .map_err(|_| "Invalid League port")?;
    if port == 0 {
        return Err("Invalid League port".into());
    }
    Ok(Lock {
        port,
        password: fields[3].into(),
    })
}
fn lock_path(custom: &str) -> Option<PathBuf> {
    let mut candidates = vec![];
    if !custom.trim().is_empty() {
        let p = PathBuf::from(custom.trim());
        candidates.push(if p.is_dir() { p.join("lockfile") } else { p });
    }
    if let Some(pd) = std::env::var_os("ProgramData") {
        let p = PathBuf::from(pd).join("Riot Games/RiotClientInstalls.json");
        if let Ok(bytes) = fs::read(p)
            && let Ok(v) = serde_json::from_slice::<Value>(&bytes)
            && let Some(m) = v["associated_client"].as_object()
        {
            for key in m.keys() {
                let p = Path::new(key);
                let dir = if p.extension().is_some() {
                    p.parent().unwrap_or(p)
                } else {
                    p
                };
                candidates.push(dir.join("lockfile"));
            }
        }
    }
    for drive in ['C', 'D', 'E', 'F', 'G'] {
        candidates.push(PathBuf::from(format!(
            "{drive}:/Riot Games/League of Legends/lockfile"
        )));
    }
    candidates.into_iter().find(|p| p.is_file())
}
struct Lcu {
    http: Client,
    lock: Lock,
}
impl Lcu {
    fn connect(custom: &str) -> Result<Self, String> {
        let p=lock_path(custom).ok_or("League client is not connected. Open League, or set its installation folder in Settings.")?;
        let lock =
            parse_lock(&fs::read_to_string(p).map_err(|_| "Could not read League lockfile")?)?;
        let cert = reqwest::Certificate::from_pem(include_bytes!("../assets/riotgames.pem"))
            .map_err(|_| "Invalid bundled Riot certificate")?;
        let http = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .add_root_certificate(cert)
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|_| "Could not create local client")?;
        Ok(Self { http, lock })
    }
    fn request(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, String> {
        let url = format!("https://127.0.0.1:{}{path}", self.lock.port);
        let mut req = self
            .http
            .request(method, url)
            .basic_auth("riot", Some(&self.lock.password));
        if let Some(body) = body {
            req = req.json(body)
        }
        let r = req.send().map_err(|e| -> String {
            if e.is_timeout() {
                "League client timed out.".into()
            } else {
                format!("Secure League connection failed: {:?}", e.without_url())
            }
        })?;
        let status = r.status();
        if !status.is_success() {
            return Err(format!(
                "League returned HTTP {} for {path}.",
                status.as_u16()
            ));
        }
        let text = r.text().map_err(|_| "Could not read League response")?;
        if text.is_empty() {
            Ok(Value::Null)
        } else {
            serde_json::from_str(&text).map_err(|_| "Invalid League response".into())
        }
    }
    fn get(&self, path: &str) -> Result<Value, String> {
        self.request(Method::GET, path, None)
    }
}
fn snapshot(path: &str, include_game: bool) -> Result<Snapshot, String> {
    let lcu = Lcu::connect(path)?;
    let phase = lcu
        .get("/lol-gameflow/v1/gameflow-phase")?
        .as_str()
        .unwrap_or("Unknown")
        .to_string();
    let me = lcu
        .get("/lol-summoner/v1/current-summoner")
        .unwrap_or_default();
    let draft = if phase == "ChampSelect" {
        sanitize_draft(&lcu.get("/lol-champ-select/v1/session")?)
    } else {
        Value::Null
    };
    let (game, game_error) = if phase == "InProgress" && include_game {
        match live_game(&lcu.http) {
            Ok(game) => (Some(game), String::new()),
            Err(e) => (None, e),
        }
    } else {
        (None, String::new())
    };
    Ok(Snapshot {
        phase,
        display_name: me["gameName"]
            .as_str()
            .or(me["displayName"].as_str())
            .unwrap_or("Connected player")
            .into(),
        draft,
        game,
        game_error,
    })
}

fn live_game(http: &Client) -> Result<crate::live::Game, String> {
    use std::io::Read;
    // Fixed loopback destination, verified Riot certificate; no lockfile credentials sent.
    let response = http
        .get("https://127.0.0.1:2999/liveclientdata/allgamedata")
        .send()
        .map_err(|_| "Waiting for the game’s local data connection.")?;
    if !response.status().is_success() {
        return Err("The game’s local data is not ready yet.".into());
    }
    let mut bytes = Vec::new();
    response
        .take(2_000_001)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read live game data.")?;
    if bytes.len() > 2_000_000 {
        return Err("Live game response is too large.".into());
    }
    let value = serde_json::from_slice(&bytes).map_err(|_| "Invalid live game response.")?;
    crate::live::parse(&value)
}
// Only retain draft information visible in the client. No teammate identity lookup.
fn sanitize_draft(v: &Value) -> Value {
    let team = |key: &str| {
        v[key].as_array().map(|arr|arr.iter().map(|p|json!({"cellId":p["cellId"],"championId":p["championId"],"championPickIntent":p["championPickIntent"],"assignedPosition":p["assignedPosition"],"spell1Id":p["spell1Id"],"spell2Id":p["spell2Id"]})).collect::<Vec<_>>()).unwrap_or_default()
    };
    json!({"myTeam":team("myTeam"),"theirTeam":team("theirTeam"),"localPlayerCellId":v["localPlayerCellId"],"bans":{"myTeamBans":draft_bans(v,true),"theirTeamBans":draft_bans(v,false)},"timer":v["timer"]})
}
fn draft_bans(v: &Value, ours: bool) -> Vec<i64> {
    let key = if ours { "myTeamBans" } else { "theirTeamBans" };
    if let Some(bans) = v["bans"][key].as_array().filter(|a| !a.is_empty()) {
        return bans
            .iter()
            .take(5)
            .map(|b| b.as_i64().unwrap_or(0))
            .collect();
    }
    let team = if ours { "myTeam" } else { "theirTeam" };
    v["actions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .flatten()
        .filter(|a| {
            a["type"] == "ban"
                && a["completed"] == true
                && v[team]
                    .as_array()
                    .is_some_and(|t| t.iter().any(|p| p["cellId"] == a["actorCellId"]))
        })
        .take(5)
        .map(|a| a["championId"].as_i64().unwrap_or(0))
        .collect()
}
pub fn draft_selection(draft: &Value) -> Option<(i64, String)> {
    let own = draft["myTeam"]
        .as_array()?
        .iter()
        .find(|p| p["cellId"] == draft["localPlayerCellId"])?;
    let champion = own["championId"].as_i64().filter(|id| *id > 0)?;
    let role = match own["assignedPosition"].as_str().unwrap_or("") {
        "top" => "Top",
        "jungle" => "Jungle",
        "middle" => "Mid",
        "bottom" => "Bottom",
        "utility" => "Support",
        _ => "Any role",
    };
    Some((champion, role.into()))
}
fn apply_runes(path: &str, b: &Build, c: &Catalog) -> Result<String, String> {
    c.validate(b)?;
    let lcu = Lcu::connect(path)?;
    let mut payload = temporary_rune_payload(b);
    // Match the client's native recommender: POST a temporary resource, then select
    // the returned page. Never turn a user's saved page into a temporary one.
    let pages = lcu.get("/lol-perks/v1/pages")?;
    let pages = pages.as_array().ok_or("Invalid rune page list")?;
    let created = if let Some(id) = rift_temporary_page(pages) {
        payload["id"] = json!(id);
        lcu.request(
            Method::PUT,
            &format!("/lol-perks/v1/pages/{id}"),
            Some(&payload),
        )?;
        lcu.get(&format!("/lol-perks/v1/pages/{id}"))?
    } else {
        lcu.request(Method::POST, "/lol-perks/v1/pages", Some(&payload))?
    };
    if created["isTemporary"] != true {
        return Err(
            "League did not confirm a temporary rune page. No saved-page overwrite was attempted."
                .into(),
        );
    }
    let id = created["id"]
        .as_i64()
        .ok_or("League returned no temporary page ID.")?;
    lcu.request(Method::PUT, "/lol-perks/v1/currentpage", Some(&json!(id)))?;
    let after = lcu.get("/lol-perks/v1/currentpage")?;
    if !temporary_runes_match(&after, id, &payload) {
        return Err("The active temporary rune page could not be verified.".into());
    }
    Ok("Temporary runes applied and verified. Your saved League pages and Lightrift builds are unchanged.".into())
}
fn temporary_rune_payload(b: &Build) -> Value {
    let mut payload = b.rune_payload();
    payload["name"] = json!(format!("Lightrift: {} / Temporary", b.champion));
    payload["isTemporary"] = json!(true);
    payload["isEditable"] = json!(true);
    payload
}
fn rift_temporary_page(pages: &[Value]) -> Option<i64> {
    pages.iter().find_map(|p| {
        let name = p["name"].as_str()?;
        (p["isTemporary"] == true
            && p["isEditable"] == true
            && (name.starts_with("Lightrift: ") || name.starts_with("Rift: "))
            && name.ends_with(" / Temporary"))
        .then(|| p["id"].as_i64())
        .flatten()
    })
}
fn temporary_runes_match(page: &Value, id: i64, payload: &Value) -> bool {
    page["id"] == id
        && page["isTemporary"] == true
        && page["current"] == true
        && rune_matches(page, payload)
}
fn rune_matches(page: &Value, payload: &Value) -> bool {
    ["primaryStyleId", "subStyleId", "selectedPerkIds"]
        .iter()
        .all(|key| page[key] == payload[key])
}
fn save_runes(path: &str, b: &Build, c: &Catalog) -> Result<String, String> {
    c.validate(b)?;
    let lcu = Lcu::connect(path)?;
    let pages = lcu.get("/lol-perks/v1/pages")?;
    let name = format!("Lightrift: {} / {}", b.champion, b.name);
    let legacy_name = format!("Rift: {} / {}", b.champion, b.name);
    let existing = pages
        .as_array()
        .ok_or("Invalid rune page list")?
        .iter()
        .find(|p| {
            (p["name"].as_str() == Some(&name) || p["name"].as_str() == Some(&legacy_name))
                && p["isEditable"].as_bool() == Some(true)
                && p["isTemporary"] != true
        });
    let mut payload = b.rune_payload();
    payload["isTemporary"] = json!(false);
    if let Some(page) = existing {
        let id = page["id"].as_u64().ok_or("Invalid rune page ID")?;
        payload["id"] = json!(id);
        lcu.request(
            Method::PUT,
            &format!("/lol-perks/v1/pages/{id}"),
            Some(&payload),
        )?;
    } else {
        lcu.request(Method::POST,"/lol-perks/v1/pages",Some(&payload)).map_err(|e|format!("{e} A free rune-page slot may be needed; Lightrift does not delete your other pages."))?;
    }
    let after = lcu.get("/lol-perks/v1/pages")?;
    if !after.as_array().is_some_and(|a| {
        a.iter().any(|p| {
            p["name"] == payload["name"]
                && p["isTemporary"] == false
                && rune_matches(p, &payload)
                && p["current"] == true
        })
    }) {
        return Err(
            "League accepted the request, but the active rune page could not be verified.".into(),
        );
    }
    Ok("Named rune page saved and selected in League.".into())
}
fn apply_items(path: &str, b: &Build, c: &Catalog) -> Result<String, String> {
    c.validate(b)?;
    if b.starter.is_empty() && b.core.is_empty() && b.situational.is_empty() {
        return Err("Add items to your build first.".into());
    }
    let lcu = Lcu::connect(path)?;
    let me = lcu.get("/lol-summoner/v1/current-summoner")?;
    let id = me["summonerId"].as_u64().ok_or("No current summoner")?;
    let endpoint = format!("/lol-item-sets/v1/item-sets/{id}/sets");
    let mut all = lcu.get(&endpoint)?;
    let key = c
        .champion(&b.champion)
        .unwrap()
        .key
        .parse()
        .map_err(|_| "Invalid champion key")?;
    let set = b.item_set(key);
    let sets = all["itemSets"]
        .as_array_mut()
        .ok_or("Unexpected item-set response")?;
    if let Some(index) = sets.iter().position(|s| s["uid"] == set["uid"]) {
        sets[index] = set.clone()
    } else {
        sets.push(set.clone())
    }
    lcu.request(Method::PUT, &endpoint, Some(&all))?;
    let after = lcu.get(&endpoint)?;
    if !after["itemSets"].as_array().is_some_and(|a| {
        a.iter()
            .any(|s| s["uid"] == set["uid"] && s["blocks"] == set["blocks"])
    }) {
        return Err("Item-set update could not be verified.".into());
    }
    Ok("Item set saved and verified in League.".into())
}
fn apply_spells(path: &str, b: &Build, c: &Catalog) -> Result<String, String> {
    c.validate(b)?;
    let lcu = Lcu::connect(path)?;
    let session = lcu.get("/lol-champ-select/v1/session")?;
    let own = session["myTeam"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|p| p["cellId"] == session["localPlayerCellId"])
        })
        .ok_or("Not currently in champion select")?;
    if own["championId"].as_i64().unwrap_or(0)
        != c.champion(&b.champion)
            .unwrap()
            .key
            .parse::<i64>()
            .unwrap_or(-1)
    {
        return Err("Select the same champion in League before applying these spells.".into());
    }
    lcu.request(
        Method::PATCH,
        "/lol-champ-select/v1/session/my-selection",
        Some(&json!({"spell1Id":b.spells[0],"spell2Id":b.spells[1]})),
    )?;
    let after = lcu.get("/lol-champ-select/v1/session")?;
    let own = after["myTeam"]
        .as_array()
        .and_then(|a| a.iter().find(|p| p["cellId"] == after["localPlayerCellId"]))
        .ok_or("Draft ended before verification")?;
    if own["spell1Id"] != b.spells[0] || own["spell2Id"] != b.spells[1] {
        return Err("Summoner spell update could not be verified.".into());
    }
    Ok("Summoner spells applied and verified.".into())
}
fn detail(http: &Client, id: &str) -> Result<Value, String> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Invalid champion".into());
    }
    let dir = data_dir().join("cache").join(PATCH.trim());
    let path = dir.join(format!("{id}.json"));
    if let Ok(bytes) = fs::read(&path)
        && let Ok(v) = serde_json::from_slice::<Value>(&bytes)
    {
        return Ok(v);
    }
    let url = format!(
        "https://ddragon.leagueoflegends.com/cdn/{}/data/en_US/champion/{id}.json",
        PATCH.trim()
    );
    let response = http.get(url).send().map_err(
        |_| "Could not download champion abilities. Your saved build is available offline.",
    )?;
    if !response.status().is_success() {
        return Err(format!("Data Dragon returned HTTP {}", response.status()));
    }
    let v: Value = response.json().map_err(|_| "Invalid champion response")?;
    let data = v["data"][id].clone();
    if data.is_null() {
        return Err("Champion detail is missing".into());
    }
    if fs::create_dir_all(&dir).is_ok() {
        let _ = fs::write(path, serde_json::to_vec(&data).unwrap_or_default());
    }
    Ok(data)
}
pub fn worker(ctx: eframe::egui::Context) -> (Sender<Command>, Receiver<Event>) {
    let (tx, rx) = mpsc::channel();
    let (out, inbox) = mpsc::channel();
    std::thread::spawn(move || {
        let catalog = Catalog::load();
        let http = Client::builder().timeout(Duration::from_secs(12)).build();
        for cmd in rx {
            let event = match cmd {
                Command::Refresh(p, game) => Event::Snapshot(snapshot(&p, game)),
                Command::Detail(id) => {
                    let result = http
                        .as_ref()
                        .map_err(|_| "Network client unavailable".into())
                        .and_then(|h| detail(h, &id));
                    Event::Detail(id, result)
                }
                Command::Runes(p, b) => Event::Applied(apply_runes(&p, &b, &catalog)),
                Command::SaveRunes(p, b) => Event::Applied(save_runes(&p, &b, &catalog)),
                Command::Items(p, b) => Event::Applied(apply_items(&p, &b, &catalog)),
                Command::Spells(p, b) => Event::Applied(apply_spells(&p, &b, &catalog)),
            };
            if out.send(event).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    (tx, inbox)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reuse_only_targets_rifts_editable_temporary_page() {
        let saved = json!({"id":1,"name":"Lightrift: Ahri / Temporary","isEditable":true,"isTemporary":false});
        let native = json!({"id":2,"name":"Recommended","isEditable":true,"isTemporary":true});
        let locked = json!({"id":3,"name":"Lightrift: Ahri / Temporary","isEditable":false,"isTemporary":true});
        assert_eq!(
            rift_temporary_page(&[saved.clone(), native.clone(), locked.clone()]),
            None
        );
        let ours = json!({"id":-100,"name":"Lightrift: Ahri / Temporary","isEditable":true,"isTemporary":true});
        assert_eq!(
            rift_temporary_page(&[saved, native, locked, ours]),
            Some(-100)
        );
        let legacy =
            json!({"id":4,"name":"Rift: Ahri / Temporary","isEditable":true,"isTemporary":true});
        assert_eq!(rift_temporary_page(&[legacy]), Some(4));
    }
    #[test]
    fn applying_runes_creates_a_temporary_resource_without_a_saved_page_id() {
        let c = Catalog::load();
        let b = c.new_build("Jinx");
        let payload = temporary_rune_payload(&b);
        assert!(payload.get("id").is_none());
        assert_eq!(payload["isTemporary"], true);
        assert_eq!(payload["current"], true);
        assert_eq!(
            payload["selectedPerkIds"],
            b.rune_payload()["selectedPerkIds"]
        );
        let mut page = payload.clone();
        page["id"] = json!(-100);
        assert!(temporary_runes_match(&page, -100, &payload));
        assert!(!temporary_runes_match(&page, 12, &payload));
        for (key, value) in [
            ("isTemporary", json!(false)),
            ("current", json!(false)),
            ("subStyleId", json!(0)),
            ("selectedPerkIds", json!([])),
        ] {
            let mut wrong = page.clone();
            wrong[key] = value;
            assert!(!temporary_runes_match(&wrong, -100, &payload));
        }
    }
    #[test]
    fn draft_bans_show_only_completed_actions_on_the_correct_team() {
        let draft = json!({"myTeam":[{"cellId":1}],"theirTeam":[{"cellId":6}],"actions":[[
            {"type":"ban","actorCellId":1,"championId":103,"completed":true},
            {"type":"ban","actorCellId":6,"championId":222,"completed":true},
            {"type":"ban","actorCellId":1,"championId":7,"completed":false},
            {"type":"pick","actorCellId":1,"championId":8,"completed":true}]]});
        let cleaned = sanitize_draft(&draft);
        assert_eq!(cleaned["bans"]["myTeamBans"], json!([103]));
        assert_eq!(cleaned["bans"]["theirTeamBans"], json!([222]));
        assert_eq!(
            draft_bans(&json!({"bans":{"myTeamBans":[103,-1,222]}}), true),
            [103, -1, 222]
        );
    }
    #[test]
    fn draft_auto_open_uses_own_pick_and_maps_role() {
        let mut draft = json!({"localPlayerCellId":1,"myTeam":[{"cellId":2,"championId":222,"assignedPosition":"bottom"},{"cellId":1,"championId":103,"championPickIntent":99,"assignedPosition":"middle"}]});
        assert_eq!(draft_selection(&draft), Some((103, "Mid".into())));
        draft["myTeam"][1]["championId"] = json!(0);
        assert!(draft_selection(&draft).is_none());
        assert!(draft_selection(&Value::Null).is_none());
    }
    #[test]
    #[ignore = "requires the running local League client"]
    fn running_client_connects_without_writes() {
        let result = snapshot("", true).expect("local League connection");
        assert!(!result.phase.is_empty());
        println!(
            "Verified local League connection; phase={}; no writes",
            result.phase
        );
    }
    #[test]
    fn lockfile_rejects_malformed_and_non_https() {
        assert!(parse_lock("LeagueClient:1:5555:secret:https").is_ok());
        for bad in [
            "x:1:5:s:https",
            "LeagueClient:1:0:s:https",
            "LeagueClient:1:443:s:http",
            "LeagueClient:1:443::https",
        ] {
            assert!(parse_lock(bad).is_err())
        }
    }
    #[test]
    fn draft_does_not_retain_identity() {
        let v = json!({"myTeam":[{"cellId":1,"championId":222,"puuid":"private","summonerId":123}],"theirTeam":[],"localPlayerCellId":1});
        let cleaned = sanitize_draft(&v);
        assert!(cleaned["myTeam"][0].get("puuid").is_none());
        assert!(cleaned["myTeam"][0].get("summonerId").is_none());
        assert_eq!(cleaned["myTeam"][0]["championId"], 222)
    }
}
