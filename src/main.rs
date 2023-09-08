// Logs and Errors
use anyhow::{Context, Result};
use fern::colors::{Color, ColoredLevelConfig};
#[allow(unused_imports)]
use log::{debug, error, info, warn};

// bgp parsing
mod bgp;
use crate::bgp::{EOF, EOW};
use bgp::{collect_bgp, parse_bgp};

// db
mod db_writer;
use db_writer::{delete_all, find_short_lived, open_db, types::DELIMITER, Processor};
use sqlx::{Connection, PgPool};

// bag of tools
use crate::db_writer::ip_search;
use clap::{Parser, Subcommand};
use crossbeam_channel::bounded;
use itertools::{Itertools, MinMaxResult};

#[derive(Subcommand)]
enum Job {
    #[command(about="WIPES DATABSE, then repopulates")]
    ReloadData,
    #[command(about="Collects all short lived announcements (<15 minutes)")]
    FindShortlived {
        #[arg(short, long, value_parser = clap::value_parser!(i64).range(0..), help="How many rows to return. Leave blank for all")]
        limit: Option<i64>,

        #[command(subcommand)]
        format: Processor,
    },
    #[command(about="Collects all announcements for a prefix")]
    SearchIP {
        ip: String,
    },
}

#[derive(Parser)]
#[command(name = "BGPTrack")]
#[command(author = "Adam T.")]
#[command(version)]
#[command(about = "BGP hijack detection tool", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Job,
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // console_subscriber::init(); // use with tokio-console
    // if atty::is(atty::Stream::Stdout) {
    //     println!("I'm a terminal");
    // } else {
    //     println!("I'm not");
    // }
    let args = Args::parse();
    set_up_logging(if !args.verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Debug
    })?;
    let pool = open_db().await?;

    match args.command {
        Job::ReloadData => {
            reload_data(pool).await?;
        }
        Job::FindShortlived { format, limit } => {
            if limit.is_none() {warn!("Searching without any limits, may take a really long time")}
            else {warn!("Long running query starting...");}
            let data = find_short_lived(900, 1692223200, 1692226800, limit, format, &pool).await?;
            // start can be 0, and stop `std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() + 60` 
            // for when start/stop are implemented arguments to the program
            if data.len() < 750
            // lets not blow up the computer...
            {
                data.iter()
                    .sorted_unstable_by_key(|point| (point.3, point.0, point.1, point.2))
                    .for_each(|datapoint| {
                        info!(
                            "ip: {},\tfirst seen: {} {},\tlast seen: {} {},\tASN: {}",
                            datapoint.0,
                            datapoint.1.date(),
                            datapoint.1.time(),
                            datapoint.2.date(),
                            datapoint.2.time(),
                            datapoint.3
                        );
                    });
            }
            info!("Dataset length = {}", data.len());
        }
        Job::SearchIP { ip } => {
            match ip_search(ip.parse()?, &pool).await {
                Ok(n) => {
                    match n.iter().minmax_by_key(|k| {k.timestamp}) { //a.timestamp.partial_cmp(&b.timestamp).unwrap()

                        MinMaxResult::NoElements => {info!("Sorry, no announcements found for {ip}")}
                        MinMaxResult::OneElement(data) => {info!("Got data:\n{data:#?}")}
                        MinMaxResult::MinMax(min, max) => {info!("Got data:\n{max:#?}\n{min:#?}")}
                    };
                    
                }
                Err(n) => {
                    error!("Something went wrong: {n}");
                }
            };
        }
    }

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
fn set_up_logging(logging_level: log::LevelFilter) -> Result<()> {
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
        .chain(fern::log_file("output.log")?)
        .level(logging_level)
        .level_for("sqlx", log::LevelFilter::Error)
        .chain(std::io::stdout())
        .apply()?;

    debug!("finished setting up logging!");

    Ok(())
}
