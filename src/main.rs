// Logs and Errors
use std::error::Error;
use anyhow::Context;
#[allow(unused_imports)]
use log::{info, warn};

// DB
#[allow(unused_imports)]
use sqlx::{Connection, Row};

mod db_writer;

use db_writer::{open_db, create, delete, types::{Orange}};

// BGP
mod bgp;

use bgp::{collect_bgp, parse_bgp};
use crate::db_writer::search;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let pool = open_db().await?;
    // info!("Querying...");
    // delete(&pool).await?;
    // 
    // // Store test ASN
    // create(&Orange { ASN: 65000, prefixes: vec![] }, &pool).await?;


    let data = tokio::task::spawn_blocking(
        move || {
            parse_bgp(collect_bgp(1692223200, 1692243953))
        }
    ).await.with_context(|| { "Collecting BGP data panicked" })?;
    // let data = match data {
    //     Ok(x) => { x }
    //     Err(_) => { panic!("???") }
    // };
    // // Find duplicates and don't enter them twice
    // // Same timestamp && Same repeater BGP && Same prefix && same path -> Same announcement
    // create(&data, &pool).await?;
    // dbg!(search("127.0.0.1".parse()?, &pool).await?);
    // dbg!(search("1.0.0.1".parse()?, &pool).await?);


    Ok(())
}


// let mut map = StringPatriciaMap::new();
// map.insert("1100", "ASN6500");
// map.insert("1110", "ASN6501");