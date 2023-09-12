#![feature(async_closure)]
// Logs and Errors
use anyhow::{Context, Result};
use async_stream::stream;
use fern::colors::{Color, ColoredLevelConfig};
#[allow(unused_imports)]
use log::{debug, error, info, warn};

// bgp parsing
mod bgp;
use crate::bgp::{EOF, EOW};
use bgp::{collect_bgp, parse_bgp};

// db
mod db_writer;
use db_writer::{delete_all, find_short_lived, open_db, types::DELIMITER};
use sqlx::{Connection, PgPool};

// bag of tools
use crate::db_writer::{ip_search, PotentialHijack};
use clap::{Parser, Subcommand};
use crossbeam_channel::bounded;
use futures::{pin_mut, stream, StreamExt};
use itertools::{Itertools, MinMaxResult};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

// Seclytics API
mod seclytics_api;
use seclytics_api::cidr_is_malicious;
#[derive(Subcommand)]
enum Job {
    #[command(about = "WIPES DATABASE, then repopulates")]
    ReloadData,
    #[command(about = "Collects all short lived announcements (<15 minutes)")]
    FindShortlived {
        #[arg(short, long, value_parser = clap::value_parser!(i64).range(0..), help="How many rows to return. Leave blank for all")]
        limit: Option<i64>,
        // #[command(subcommand)]
        // format: Processor,
    },
    #[command(about = "Collects all announcements for a prefix")]
    SearchIP { ip: String },
    
    #[command(about = "Runs arbitrary commands, testing new code only")]
    Test,
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
    #[arg(short, long)]
    quiet: bool,
}



#[tokio::main]
async fn main() -> Result<()> {
    // console_subscriber::init(); // use with tokio-console

    let args = Args::parse();

    // Set up appropriate logging level
    set_up_logging(if args.quiet {
        log::LevelFilter::Off
    } else if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    })?;

    // Setup pool of connections to db
    let pool = open_db().await?;

    match args.command {
        Job::ReloadData => {
            reload_data(pool).await?;
        }
        Job::FindShortlived { limit } => {
            // additionally warn if we're scanning the whole database, this may take literally forever :/
            if limit.is_none() {
                warn!("Searching without any limits, may take a really long time")
            } else {
                warn!("Long running query starting...");
            }
            
            
            // start can be 0, and stop `std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() + 60`
            // for when start/stop are implemented arguments to the program
            let data = find_short_lived(900, 1692223200, 1692226800, limit, 3600, &pool).await;
            pin_mut!(data);
            
            let potentials = data.collect::<Vec<Result<PotentialHijack>>>().await;
            
            // Vec<Results<_>> -> Results<Vec<_>> -> Vec<_>
            let potentials: Vec<PotentialHijack> = potentials
                .into_iter()
                .collect::<Result<Vec<PotentialHijack>>>()
                .context("was attempting to move results out of Vec")?;

            // Sort Potential Hijacks by asn for easier matching
            let p_iter = potentials
                .into_iter()
                .sorted_unstable_by_key(|x| x.asn)
                .collect_vec();
            let mut p_iter = p_iter.iter().peekable();
            

            let asn_group_gen = stream! {
                let mut flag: bool = true;
                loop {
                    if flag {
                        flag = false;
                        match p_iter.peek() {
                            None => {
                                break;
                            }
                            Some(x) => {
                                let asn = x.asn; //implicit copy to prevent double mut ref
                                let ans = p_iter.peeking_take_while(|&y| {y.asn == asn}).collect_vec();
                                if ans.len() == 0 {continue;}
                                yield ans;
                            }
                        }

                    } else {
                        match p_iter.next().as_ref() {
                            None => {
                                break;
                            }
                            Some(x) => {
                                let ans = p_iter.peeking_take_while(|&y| {x.asn == y.asn}).collect_vec();
                                if ans.len() == 0 {continue;}
                                yield ans;
                            }
                        }


                    }
                }
            };
            pin_mut!(asn_group_gen);

            while let Some(asn_group) = asn_group_gen.next().await {
                debug!("AS{} had {} short lived ann", asn_group[0].asn, asn_group.len()); // AS38254
                if asn_group[0].asn == 29049 {
                    'inner: for ann in asn_group {
                        if ann.prefix == "103.100.227.0/24".parse()? && cidr_is_malicious(ann.prefix).await? {
                            warn!("Malicious prefix: {} from asn {}",ann.prefix, ann.asn);
                            break 'inner;
                        }
                    }
                }
            }
            
        }
        Job::SearchIP { ip } => {
            match ip_search(ip.parse()?, &pool).await {
                Ok(n) => {
                    match n.iter().minmax_by_key(|k| k.timestamp) {
                        //a.timestamp.partial_cmp(&b.timestamp).unwrap()
                        MinMaxResult::NoElements => {
                            info!("Sorry, no announcements found for {ip}")
                        }
                        MinMaxResult::OneElement(data) => {
                            info!("Got data:\n{data:#?}")
                        }
                        MinMaxResult::MinMax(min, max) => {
                            info!("Got data:\n{max:#?}\n{min:#?}")
                        }
                    };
                }
                Err(n) => {
                    error!("Something went wrong: {n}");
                }
            };
        }
        Job::Test => {
            error!("No tests to run!");
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
