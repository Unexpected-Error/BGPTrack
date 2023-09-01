use std::net::IpAddr;
// Logs and Errors
use anyhow::{anyhow, Context, Result};

#[allow(unused_imports)]
#[cfg(debug_assertions)]
use log::{info, warn};

// DB
use sqlx::Row;

mod bgp;
use bgp::{collect_bgp, parse_bgp};
mod db_writer;
use db_writer::{delete_all, open_db};

use crate::db_writer::types::{AP_Segments, ASPathSeg, Announcement, DELIMITER};
use crossbeam_channel::unbounded;
use ipnetwork::IpNetwork;
use tokio::io::{stdin, AsyncBufReadExt, AsyncRead, AsyncReadExt};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    {
        console_subscriber::init();
        env_logger::init();
    }

    let pool = open_db().await?;
    delete_all(&pool).await?;

    let mut conn = pool.acquire().await?;

    let (sender, receiver) = unbounded::<Vec<u8>>();

    let handle1 = tokio::task::spawn_blocking(move || {
        parse_bgp(collect_bgp(1692223577, 1692223953), sender)?;
        anyhow::Ok(())
        // 15 min of data 1692223200 1692223953
        // 1 day 1692309600
    });
    let handle2 = tokio::task::spawn(async move {
        let mut cpin = conn
            .copy_in_raw(&*format!(
                "COPY Announcement FROM STDIN (DELIMITER '{DELIMITER}', FORMAT csv)"
            ))
            .await?;
        #[cfg(debug_assertions)]
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
        #[cfg(not(debug_assertions))]
        {
            for data in receiver.iter() {
                cpin.send(data).await?;
            }
            cpin.finish().await?;
        }
        anyhow::Ok(())
    });
    handle1
        .await
        .with_context(|| "Parsing BGP data panicked")??;
    handle2
        .await
        .with_context(|| "Saving BGP data panicked")??;
    Ok(())
}
// 15 minutes of data | 287 MB on disk | 42 sec || 0.04666666667 time ratio

// 1 day | 28GB | ~ 1 hour
