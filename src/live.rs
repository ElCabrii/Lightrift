use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Player {
    pub champion: String,
    pub name: String,
    pub team: String,
    pub level: u64,
    pub dead: bool,
    pub kills: u64,
    pub deaths: u64,
    pub assists: u64,
    pub cs: u64,
    pub items: Vec<(u32, u64)>,
    pub runes: Vec<u32>,
    pub primary: String,
    pub secondary: String,
    pub full_runes: bool,
}

impl Player {
    pub fn item_gold(&self, catalog: &crate::data::Catalog) -> Option<u64> {
        // A completed item's total price already includes its recipe. Only count
        // components that are still held separately, and multiply stack counts.
        self.items.iter().try_fold(0_u64, |total, (id, count)| {
            let price = catalog.items.get(id)?.gold["total"].as_u64()?;
            total.checked_add(price.checked_mul(*count)?)
        })
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub seconds: f64,
    pub players: Vec<Player>,
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}
fn id(v: &Value) -> Option<u32> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
}
fn same_player(player: &Value, active: &Value) -> bool {
    // Match exact identifiers, never champion names (custom games may contain duplicates).
    for key in ["riotId", "summonerName"] {
        if let (Some(a), Some(b)) = (player[key].as_str(), active[key].as_str())
            && !a.is_empty()
            && !b.is_empty()
        {
            return a == b;
        }
    }
    false
}

pub fn parse(v: &Value) -> Result<Game, String> {
    let players = v["allPlayers"]
        .as_array()
        .filter(|p| !p.is_empty() && p.len() <= 64)
        .ok_or("Waiting for the in-game player list.")?;
    let active = &v["activePlayer"];
    Ok(Game {
        seconds: v["gameData"]["gameTime"]
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.0)
            .unwrap_or(0.0),
        players: players
            .iter()
            .map(|p| {
                let is_me = same_player(p, active);
                let full = is_me && active["fullRunes"]["generalRunes"].is_array();
                let r = if full {
                    &active["fullRunes"]
                } else {
                    &p["runes"]
                };
                let mut runes = Vec::new();
                if full {
                    for key in ["generalRunes", "statRunes"] {
                        if let Some(list) = r[key].as_array() {
                            runes.extend(list.iter().filter_map(|r| id(&r["id"])));
                        }
                    }
                } else if let Some(keystone) = id(&r["keystone"]["id"]) {
                    runes.push(keystone);
                }
                runes.truncate(9);
                Player {
                    champion: p["rawChampionName"]
                        .as_str()
                        .and_then(|s| s.strip_prefix("game_character_displayname_"))
                        .unwrap_or(p["championName"].as_str().unwrap_or("Unknown"))
                        .to_string(),
                    name: p["riotId"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .or(p["summonerName"].as_str())
                        .unwrap_or("Player")
                        .to_string(),
                    team: text(&p["team"]),
                    level: p["level"].as_u64().unwrap_or(1),
                    dead: p["isDead"].as_bool().unwrap_or(false),
                    kills: p["scores"]["kills"].as_u64().unwrap_or(0),
                    deaths: p["scores"]["deaths"].as_u64().unwrap_or(0),
                    assists: p["scores"]["assists"].as_u64().unwrap_or(0),
                    cs: p["scores"]["creepScore"].as_u64().unwrap_or(0),
                    items: p["items"]
                        .as_array()
                        .map(|items| {
                            let mut items: Vec<_> = items
                                .iter()
                                .filter_map(|i| {
                                    Some((
                                        i["slot"].as_u64().unwrap_or(0),
                                        id(&i["itemID"])?,
                                        i["count"].as_u64().unwrap_or(1),
                                    ))
                                })
                                .collect();
                            items.sort_by_key(|i| i.0);
                            items
                                .into_iter()
                                .take(7)
                                .map(|(_, id, count)| (id, count))
                                .collect()
                        })
                        .unwrap_or_default(),
                    runes,
                    primary: text(&r["primaryRuneTree"]["displayName"]),
                    secondary: text(&r["secondaryRuneTree"]["displayName"]),
                    full_runes: full,
                }
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn full_runes_are_only_assigned_to_exact_active_player() {
        let p = json!({"championName":"Jinx","rawChampionName":"game_character_displayname_Jinx","riotId":"Test#1","team":"ORDER","scores":{"kills":3,"deaths":2,"assists":9,"creepScore":123},"items":[{"itemID":3031,"slot":2,"count":1},{"itemID":1055,"slot":0,"count":1}],"runes":{"keystone":{"id":8005}}});
        let mut other = p.clone();
        other["riotId"] = json!("Test#2");
        other["team"] = json!("CHAOS");
        let game = parse(&json!({"allPlayers":[p,other],"activePlayer":{"riotId":"Test#1","currentGold":421.0,"fullRunes":{"generalRunes":[{"id":8005},{"id":9101}],"statRunes":[{"id":5005}]}},"gameData":{"gameTime":600.0}})).unwrap();
        assert!(game.players[0].full_runes);
        assert!(!game.players[1].full_runes);
        assert_eq!(game.players[0].runes, [8005, 9101, 5005]);
        assert_eq!(game.players[1].runes, [8005]);
        assert_eq!(game.players[0].items, [(1055, 1), (3031, 1)]);
        assert_eq!(game.players[0].cs, 123);
    }
    #[test]
    fn missing_identity_never_receives_active_player_data() {
        let game = parse(&json!({"allPlayers":[{}],"activePlayer":{"currentGold":999}})).unwrap();
        assert!(!game.players[0].full_runes);
        assert!(parse(&Value::Null).is_err());
        assert!(parse(&json!({"allPlayers":[]})).is_err());
        assert!(!same_player(
            &json!({"riotId":"Same#2","summonerName":"Same"}),
            &json!({"riotId":"Same#1","summonerName":"Same"})
        ));
    }
    #[test]
    fn item_gold_counts_held_items_components_and_stacks_without_recipe_double_counting() {
        let catalog = crate::data::Catalog::load();
        let mut game = parse(&json!({"allPlayers":[{"items":[
            {"itemID":3031,"count":1,"slot":0},
            {"itemID":1036,"count":1,"slot":1},
            {"itemID":2003,"count":2,"slot":2},
            {"itemID":3340,"count":1,"slot":6}
        ]}],"activePlayer":{"currentGold":99999}}))
        .unwrap();
        // Infinity Edge (3500) + held Long Sword (350) + two potions (100).
        assert_eq!(game.players[0].item_gold(&catalog), Some(3950));
        game.players[0].items = vec![(3031, 1)];
        assert_eq!(game.players[0].item_gold(&catalog), Some(3500));
        game.players[0].items.clear();
        assert_eq!(game.players[0].item_gold(&catalog), Some(0));
        game.players[0].items = vec![(1036, 1), (u32::MAX, 1)];
        assert_eq!(game.players[0].item_gold(&catalog), None);
    }
}
