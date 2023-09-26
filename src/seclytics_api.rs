use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;
// errors, logs, tools, etc
use anyhow::{anyhow, Result};
use fxhash::FxHashSet;
use ipnetwork::IpNetwork;
use itertools::Itertools;

use log::{debug, error, info, trace, warn};
use reqwest::Client;
use serde_json::json;
use url::Url;

lazy_static::lazy_static! {
    static ref SECLYTICS_API_TOKEN: String = {
        use dotenvy::{dotenv};
        dotenv().expect(".env file not found");
        dotenvy::var("SECLYTICS_API_TOKEN").expect("SECLYTICS_API_TOKEN not found")
    };
    static ref SECLYTICS_API_ENDPOINT: String = {
        use dotenvy::{dotenv};
        dotenv().expect(".env file not found");
        dotenvy::var("SECLYTICS_API_ENDPOINT").expect("SECLYTICS_API_ENDPOINT not found")
    };
}
pub(crate) async fn asn_is_malicious(
    asn: i64,
    cidr: Vec<IpNetwork>,
    client: &Client,
) -> Result<(usize, bool)> {
    let data: serde_json::Value = client
        .get(url(
            &*("asns/".to_string() + &*asn.to_string()),
            [
                // ("ids".to_string(), &*asn.iter().map(|x| x.0.to_string()).join(",")),
                ("access_token".to_string(), ""),
            ],
        )?)
        .send()
        .await?
        .json()
        .await?;

    let mal_asn = if let Some(categories) = data["global_threat_context"]["categories"].as_array() {
        categories.contains(&json!("malicious"))
    } else {
        false
    };
    if data["global_threat_context"]["cidrs"] == json!(null) {
        warn!("Could not find data for AS{asn}");
        return Ok((0, mal_asn));
    }
    let known_bad: HashSet<IpNetwork> = data["global_threat_context"]["cidrs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| match x {
            serde_json::Value::String(x) => Some(
                IpNetwork::from_str(x)
                    .ok()
                    .expect(&*format!("couldn't parse, {x}")),
            ),
            _ => None,
        })
        .collect();
    debug!("{} known bad cidrs for AS{asn}", known_bad.len());

    Ok((
        cidr.iter()
            .filter(|prefix| known_bad.contains(prefix))
            .count(),
        mal_asn,
    ))
}

fn url<const N: usize>(path: &str, mut options: [(String, &str); N]) -> Result<String>
where
    [(); N + 1]:,
{
    // try and remove later with generic_const_expr

    // let sol: [(String, &str); N+1] = [options, [("".to_string(),"")]].iter().flat_map(|s| s.iter()).collect();
    if options[N - 1].0 != "access_token".to_string() {
        error!("Did not leave room for apikey, abort");
        panic!()
    } else {
        options[N - 1].1 = &**SECLYTICS_API_TOKEN
    }

    let api = format!(
        "{}{path}?{}",
        *SECLYTICS_API_ENDPOINT,
        options.map(|(s1, s2)| { s1 + "=" + s2 }).join("&")
    );
    trace!("URL constructed, {}", api);
    Ok(api)
}
