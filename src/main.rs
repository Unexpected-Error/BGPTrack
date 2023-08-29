

// Logs and Errors
use anyhow::{Context, Result};
use async_ringbuf::{AsyncHeapRb};
#[allow(unused_imports)]
use log::{info, warn};

// DB
#[allow(unused_imports)]
use sqlx::{Connection, Row};
use tokio::io::{AsyncRead, AsyncReadExt, BufReader, ReadBuf};
use uuid::Uuid;
mod db_writer;

use db_writer::{open_db};
use crate::bgp::{collect_bgp, parse_bgp};
use crate::db_writer::delete_all;
use tokio::time::{sleep, Duration};
use crossbeam_channel::{unbounded}; 
// BGP
mod bgp;

// use crate::db_writer::{insert_announcement};
// use bgp::{collect_bgp, parse_bgp};


// let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();
    env_logger::init();
    let pool = open_db().await?;
    delete_all(&pool).await?;
    // sleep(Duration::from_millis(2000)).await;
    let mut conn = pool.acquire().await?;
    let mut cpin = conn.copy_in_raw("COPY Announcement FROM STDIN").await?;
    let (sender, receiver) = unbounded();
    
    
    
    tokio::task::spawn_blocking(move || {
        parse_bgp(collect_bgp(1692223200, 1692223953), sender)
        // 15 min of data 1692223200 1692223953
    })
    .await
    .with_context(|| "Collecting BGP data panicked")??;
    
    {
        use std::time::Instant;
        for data in receiver.iter() {
            let now = Instant::now();
            cpin.send(data).await?;
            let elapsed = now.elapsed();
            println!("Elapsed: {:.2?}", elapsed);
        }
        cpin.finish().await?;
    }
    Ok(())
}
// let mut map = StringPatriciaMap::new();
// map.insert("1100", "ASN6500");
// map.insert("1110", "ASN6501");
// 　22 days to download 3 days of data and 3 days to reasearch methods
// TODO: Add parralelled insert using pool
// 20MB network   | 190 MB disk || 15 minutes data | 290 minutes processing time 
// 175 GB network | 1.7 TB      || 3 months data   | 4.8 years processing time 
// TODO: Collect timing and size stats
// TODO: Use docker to automate

