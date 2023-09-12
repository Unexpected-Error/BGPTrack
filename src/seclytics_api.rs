// errors, logs, tools, etc
use anyhow::{anyhow, Context, Result};
use async_stream::{stream, try_stream};
use clap::Subcommand;
use fix_hidden_lifetime_bug::fix_hidden_lifetime_bug;
use futures::stream::StreamExt;
use futures::{pin_mut, Stream};
use ipnetwork::IpNetwork;
use itertools::Itertools;
use log::{debug, info, warn};
use serde_json::json;
use url::{ParseOptions, Url};

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
pub(crate) async fn cidr_is_malicious(cidr: IpNetwork) -> Result<bool> {
    let data: serde_json::Value = reqwest::Client::new()
        .get(url(
            &*format!(
                "cidrs/{cidr_base}/{cidr_mask}/ips/",
                cidr_base = cidr.ip(),
                cidr_mask = cidr.prefix()
            ),
            [],
        )?)
        .send()
        .await?
        .json()
        .await?;
    if data[0]["context"]["categories"] != json!(null) {
        Ok(true)
    } else {
        Ok(false)
    }
}

fn url<const n: usize>(path: &str, options: [(&str, &str); n]) -> Result<Url> {
    // let mut api = (*SECLYTICS_API_ENDPOINT).clone();
    let mut api = Url::options()
        .base_url(Some(&Url::parse(&**SECLYTICS_API_ENDPOINT)?))
        .parse(path)?;

    for option in options {
        api.set_query(Some(&*(option.0.to_string() + "=" + option.1)));
    }
    api.set_query(Some(
        &*("access_token=".to_string() + (*SECLYTICS_API_TOKEN).as_str()),
    ));

    warn!("URL constructed, {}", api);
    Ok(api)
}
