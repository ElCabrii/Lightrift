//! Read the core-build table published on the public champion page.
//! Decode bounded JSON records only; never execute page scripts.
use super::*;
const PAGE_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct CoreOption {
    pub ids: Vec<u32>,
    pub games: u64,
    pub pick_rate: f64,
    pub win_rate: f64,
}
pub(super) fn fetch(
    http: &Client,
    request: &Request,
    position: &str,
    patch: &str,
) -> Result<Vec<CoreOption>, String> {
    let url = format!(
        "https://op.gg/lol/champions/{}/build/{position}?tier=all&region=global",
        request.champion.to_lowercase()
    );
    let response = http
        .get(url)
        .header("User-Agent", "Lightrift/1.0 (League build companion)")
        .header("Accept", "text/html")
        .timeout(Duration::from_secs(10))
        .send()
        .map_err(|_| "public page could not be reached")?;
    if !response.status().is_success() {
        return Err(format!(
            "public page returned HTTP {}",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(PAGE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "public page could not be read")?;
    let html = std::str::from_utf8(&bytes).map_err(|_| "public page encoding changed")?;
    parse(html, &request.name, position, patch)
}

fn visit(
    v: &Value,
    depth: usize,
    budget: &mut usize,
    callback: &mut impl FnMut(&Value),
) -> Result<(), String> {
    if depth > 64 || *budget == 0 {
        return Err("public page structure exceeds limits".into());
    }
    *budget -= 1;
    callback(v);
    match v {
        Value::Array(values) => {
            for value in values {
                visit(value, depth + 1, budget, callback)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                visit(value, depth + 1, budget, callback)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn expand(
    v: &Value,
    records: &BTreeMap<String, Value>,
    depth: usize,
    budget: &mut usize,
) -> Result<Value, String> {
    if depth > 64 || *budget == 0 {
        return Err("public page reference exceeds limits".into());
    }
    *budget -= 1;
    if let Some(s) = v.as_str().and_then(|s| s.strip_prefix('$')) {
        let key = s.strip_prefix('L').unwrap_or(s);
        if !key.is_empty()
            && key.chars().all(|c| c.is_ascii_hexdigit())
            && let Some(target) = records.get(key)
        {
            return expand(target, records, depth + 1, budget);
        }
    }
    Ok(match v {
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|x| expand(x, records, depth + 1, budget))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(k, x)| Ok((k.clone(), expand(x, records, depth + 1, budget)?)))
                .collect::<Result<_, String>>()?,
        ),
        _ => v.clone(),
    })
}
fn text_content(v: &Value, out: &mut Vec<String>) {
    // Called only after depth/budget-limited expansion.
    if let Some(a) = v.as_array() {
        if a.first().and_then(Value::as_str) == Some("$") {
            if let Some(children) = a.get(3).and_then(|p| p.get("children")) {
                text_content(children, out);
            }
        } else {
            for child in a {
                text_content(child, out);
            }
        }
    } else if let Some(s) = v.as_str().filter(|s| !s.starts_with('$')) {
        out.push(s.to_owned());
    } else if v.is_number() {
        out.push(v.to_string());
    }
}
fn rate(s: &str) -> Option<f64> {
    s.trim()
        .strip_suffix('%')?
        .parse::<f64>()
        .ok()
        .filter(|r| r.is_finite() && (0.0..=100.0).contains(r))
        .map(|r| r / 100.0)
}
pub(super) fn parse(
    html: &str,
    champion: &str,
    position: &str,
    patch: &str,
) -> Result<Vec<CoreOption>, String> {
    if html.len() > PAGE_LIMIT {
        return Err("public page exceeds size limit".into());
    }
    let mut flight = String::new();
    for script in html.split("<script").skip(1) {
        let Some((_, body)) = script.split_once('>') else {
            continue;
        };
        let Some((body, _)) = body.split_once("</script>") else {
            continue;
        };
        let Some(json) = body
            .trim()
            .strip_prefix("self.__next_f.push(")
            .and_then(|s| s.trim_end_matches(';').strip_suffix(')'))
        else {
            continue;
        };
        if let Ok(Value::Array(data)) = serde_json::from_str::<Value>(json)
            && data.first().and_then(Value::as_u64) == Some(1)
            && let Some(text) = data.get(1).and_then(Value::as_str)
        {
            flight.push_str(text);
        }
    }
    let mut records = BTreeMap::new();
    for line in flight.lines() {
        if let Some((key, json)) = line.split_once(':')
            && let Ok(value) = serde_json::from_str::<Value>(json)
        {
            records.insert(key.to_owned(), value);
        }
    }
    let mut matched = false;
    let mut rows = BTreeMap::new();
    let mut budget = 200_000;
    for record in records.values() {
        visit(record, 0, &mut budget, &mut |v| {
            if v.is_object()
                && v.get("championName")
                    .and_then(Value::as_str)
                    .is_some_and(|name| canonical_champion(name) == canonical_champion(champion))
                && v["position"] == position
                && v["patch"] == patch
                && v["tier"] == "all"
                && v["region"] == "global"
                && v["type"] == "ranked"
            {
                matched = true;
            }
            if let Some(a) = v.as_array()
                && a.get(1).and_then(Value::as_str) == Some("tr")
                && let Some(index) = a
                    .get(2)
                    .and_then(Value::as_str)
                    .and_then(|s| s.strip_prefix("core_items_"))
                    .and_then(|s| s.parse::<usize>().ok())
                && index < 10
            {
                rows.insert(index, v.clone());
            }
        })?;
    }
    if !matched {
        return Err("champion, lane, patch or rank context could not be verified".into());
    }
    let mut options: Vec<CoreOption> = Vec::new();
    for row in rows.values() {
        let row = expand(row, &records, 0, &mut budget)?;
        let mut cells = Vec::new();
        visit(&row, 0, &mut budget, &mut |v| {
            if v.as_array()
                .is_some_and(|a| a.get(1).and_then(Value::as_str) == Some("td"))
            {
                cells.push(v.clone());
            }
        })?;
        if cells.len() != 3 {
            continue;
        }
        let mut ids = Vec::new();
        visit(&cells[0], 0, &mut budget, &mut |v| {
            if v.is_object()
                && v["metaType"] == "item"
                && let Some(id) = v["metaId"].as_u64().and_then(|n| u32::try_from(n).ok())
            {
                ids.push(id);
            }
        })?;
        if ids.len() != 3
            || ids.contains(&0)
            || ids.iter().collect::<std::collections::BTreeSet<_>>().len() != 3
            || options.iter().any(|o| o.ids == ids)
        {
            continue;
        }
        let (mut picks, mut wins) = (Vec::new(), Vec::new());
        text_content(&cells[1], &mut picks);
        text_content(&cells[2], &mut wins);
        let Some(pick_rate) = picks.iter().find_map(|s| rate(s)) else {
            continue;
        };
        let Some(win_rate) = wins.iter().find_map(|s| rate(s)) else {
            continue;
        };
        let Some(games) = picks
            .iter()
            .find_map(|s| s.trim().replace(',', "").parse::<u64>().ok())
            .filter(|n| *n > 0)
        else {
            continue;
        };
        options.push(CoreOption {
            ids,
            games,
            pick_rate,
            win_rate,
        });
    }
    if options.is_empty() {
        return Err("no complete core-build rows found".into());
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    const PAGE: &str = include_str!("../../tests/fixtures/opgg-ahri-core-page.html");
    #[test]
    fn published_core_rows_resolve_references_and_keep_their_own_samples() {
        let options = parse(PAGE, "Ahri", "mid", "16.19").unwrap();
        assert_eq!(options.len(), 5);
        assert_eq!(options[0].ids, [3118, 4645, 3157]);
        assert_eq!(options[1].ids, [3118, 4645, 3089]);
        assert_eq!(options[0].games, 25618);
        assert!((options[0].win_rate - 0.5238).abs() < 0.00001);
        assert_eq!(options[1].games, 11690);
        assert_eq!(options[2].ids.len(), 3); // split across streamed records on the real page
    }
    #[test]
    fn changed_page_context_or_structure_never_makes_up_alternatives() {
        for (champ, lane, patch) in [
            ("Jinx", "mid", "16.19"),
            ("Ahri", "top", "16.19"),
            ("Ahri", "mid", "16.20"),
        ] {
            assert!(parse(PAGE, champ, lane, patch).is_err());
        }
        assert!(parse("<html>Unavailable</html>", "Ahri", "mid", "16.19").is_err());
        assert!(parse(&"x".repeat(PAGE_LIMIT + 1), "Ahri", "mid", "16.19").is_err());
        let records = BTreeMap::from([("a".into(), json!("$La"))]);
        assert!(expand(&json!("$La"), &records, 0, &mut 100).is_err());
    }
}
