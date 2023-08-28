// Logs and Errors
use anyhow::{Context, Result};
#[allow(unused_imports)]
use log::{info, warn};

// DB
#[allow(unused_imports)]
use sqlx::{Connection, Row};

mod db_writer;

use db_writer::{open_db, types::Orange};

// BGP
mod bgp;

use crate::db_writer::{insert_announcement};
use bgp::{collect_bgp, parse_bgp};

#[tokio::main]
async fn main() -> Result<()> {
    
    env_logger::init();
    let pool = open_db().await?;
    
    let data = tokio::task::spawn_blocking(move || {
        parse_bgp(collect_bgp(1692223200, 1692223953))
        // 20230816.2200
        //1692811618
        //1692243953
        //1692206400
    })
    .await
    .with_context(|| "Collecting BGP data panicked")??;

    for point in data {
        insert_announcement(point, &pool).await?;
    }

    Ok(())
}


// let mut map = StringPatriciaMap::new();
// map.insert("1100", "ASN6500");
// map.insert("1110", "ASN6501");

// TODO: Add parralelled insert using pool
// 20MB network   | 190 MB disk || 15 minutes data | 290 minutes processing time 
// 175 GB network | 1.7 TB      || 3 months data   | 4.8 years processing time 
// TODO: Collect timing and size stats
// TODO: Use docker to automate
