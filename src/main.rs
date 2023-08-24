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

use crate::db_writer::types::Announcement;
use crate::db_writer::{insert, search};
use bgp::{collect_bgp, parse_bgp};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let pool = open_db().await?;
    // info!("Querying...");
    // delete(&pool).await?;
    //
    // // Store test ASN
    // create(&Orange { ASN: 65000, prefixes: vec![] }, &pool).await?;

    //insert_new_ann(/* db_writer::types::Announcement */, 65000, /* &Pool<Postgres> */).await?;

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
        insert(point, &pool).await?;
    }

    Ok(())
}

// async fn insert_new_ann(
//     ann: Announcement,
//     ASN: i64,
//     pool: &sqlx::PgPool,
// ) -> Result<(), anyhow::Error> {
//     sqlx::query!(
//         "UPDATE Orange SET announcements = array_append(announcements, $2) WHERE asn = $1",
//         //try announcements ann as () to use _ as _ on inner
// //        ann as _,
//         ASN,
//         ann as _
//     )
//         .execute(pool)
//         .await.context("here")?;
//     Ok(())
// }

// let mut map = StringPatriciaMap::new();
// map.insert("1100", "ASN6500");
// map.insert("1110", "ASN6501");

// TODO: Add parralelled insert using pool
// TODO: Collect timing and size stats
// TODO: Use docker to automate
