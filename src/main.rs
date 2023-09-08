// Logs and Errors
use anyhow::{Context, Result};
use fern::colors::{Color, ColoredLevelConfig};
#[allow(unused_imports)]
use log::{info, warn, debug, error};

// bgp parsing
mod bgp;
use bgp::{collect_bgp, parse_bgp};
use crate::bgp::{EOF, EOW};

// db
mod db_writer;
use db_writer::{delete_all, open_db,find_short_lived, Processor, types::DELIMITER};
use sqlx::{Connection, PgPool};

// bag of tools
use crossbeam_channel::bounded;
use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Job {
    ReloadData,
    FindShortlived
}


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Job,
}


#[tokio::main]
async fn main() -> Result<()> {
    // console_subscriber::init(); // use with tokio-console 
    set_up_logging()?;
    let args = Args::parse();
    

    let pool = open_db().await?;

    let data = find_short_lived(900, 1692223200, 1692226800, 10,Processor::OverallTimeRange, &pool).await?;
    info!("data");
    for datapoint in data {
        info!("{datapoint:?}");
    }
    //reload_data(pool).await?;
    Ok(())
}

async fn reload_data(pool: PgPool) -> Result<()> {
    delete_all(&pool).await?;
    
    let mut conn = pool.acquire().await?;

    let (sender, receiver) = bounded::<Vec<u8>>(0);

    let handle1 = tokio::task::spawn_blocking(move || {
        parse_bgp(collect_bgp(1692223200, 1692230400), sender)?;
        anyhow::Ok(())
        // 15 min of data 1692223200 1692223953
        // 1 hours 1692226800
        // 2 hours 1692230400
        // 1 day 1692309600
        // 3 days 1692482400
    });
    let handle2 = tokio::task::spawn(async move {
        'Outer: loop {
            let mut cpin = conn
                .copy_in_raw(&*format!(
                    "COPY Announcement FROM STDIN (DELIMITER '{DELIMITER}', FORMAT csv)"
                ))
                .await?;
            let testcase = Vec::from(EOW);
            let testcase2 = Vec::from(EOF);
            use std::time::Instant;
            while let Ok(data) = receiver.recv() {
                if data == testcase {
                    break;
                }
                if data == testcase2 {
                    break 'Outer;
                }
                let now = Instant::now();
                match cpin.send(data).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error while sending, {e}")
                    }
                }
                let elapsed = now.elapsed();
                info!(">>> Sent data in: {:.2?}", elapsed);
            }
            match cpin.finish().await {
                Ok(n) => {
                    info!(">>> Finished copying successfully, copied {n} rows")
                }
                Err(e) => {
                    error!(">>> Error while finishing copy, {e}")
                }
            }
            match conn.ping().await {
                Ok(_) => {
                    info!(">>> Connection still valid")
                }
                Err(_) => {
                    info!(">>> Connection invalid valid")
                }
            }
        }
        anyhow::Ok(())
    });
    handle1
        .await
        .with_context(|| ">>> Parsing BGP data panicked")??;
    handle2
        .await
        .with_context(|| ">>> Saving BGP data panicked")??;
    Ok(())
}
// 15 minutes of data | 287 MB on disk | 42 sec || 0.04666666667 time ratio

// 1 day | 28GB | ~ 1 hour

/// what it says on the tin. Call asap on main
fn set_up_logging() -> Result<()>{
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::BrightYellow)
        .info(Color::White)
        .debug(Color::White)
        .trace(Color::BrightBlack);
    
    let colors_level = colors_line.info(Color::Green);
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date} {level} {target} {color_line}] {message}\x1B[0m",
                color_line = format_args!(
                    "\x1B[{}m",
                    colors_line.get_color(&record.level()).to_fg_str()
                ),
                date = humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                target = record.target(),
                level = colors_level.color(record.level()),
                message = message,
            ));
        })
        .level(log::LevelFilter::Debug)
        .level_for("sqlx", log::LevelFilter::Error)
        .chain(std::io::stdout())
        .apply()?;

    debug!("finished setting up logging!");
    
    Ok(())
}