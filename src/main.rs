#![feature(async_closure)]
#![feature(generic_const_exprs)]
#![feature(let_chains)]

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
use db_writer::{find_short_lived, open_db, types::DELIMITER};
use sqlx::{PgPool};

// bag of tools
use crate::db_writer::{ip_search, PotentialHijack};
use clap::{Parser, Subcommand};
use crossbeam_channel::bounded;
use futures::{pin_mut, StreamExt};
use itertools::{Itertools, MinMaxResult};

// Seclytics API
mod seclytics_api;
use seclytics_api::asn_is_malicious;

#[derive(Subcommand)]
enum Job {
    #[command(about = "Adds data to table named new_Announcement")]
    GetData,
    #[command(
        about = "Collects all short lived announcements (<15 minutes) from Announcement table"
    )]
    FindShortLived,
    #[command(about = "Collects all announcements for a prefix form Announcement table")]
    SearchIP { ip: String },

    #[command(about = "Runs arbitrary commands, testing new code only")]
    Test,
    #[command(about = "Does nothing and exits")]
    NOP,
}

#[derive(Parser)]
#[command(name = "bgp track")]
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
    // Parse command
    let args = Args::parse();

    // Set up appropriate logging level
    set_up_logging(if args.quiet {
        log::LevelFilter::Off
    } else if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    })?;

    // Start pool of connections to db
    let pool = open_db().await?;

    match args.command {
        Job::NOP => info!("NOP"),
        Job::GetData => {
            reload_data(pool).await?;
        }
        Job::FindShortLived => {
            // start can be 0, and stop `std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() + 60` to scan whole database
            let data = find_short_lived(900, 1660687200, 1660694499, None, 3600, &pool).await;
            pin_mut!(data);

            // using streams 1325437/ 38387634 = 3.45%

            // wait for an collect all the announcements [`PotentialHijack`] from the database
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

            // Make iter peekable for some gymnastics to get iter of announcements grouped by ASN
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
                            Some(x) => { // yield a iter of announcements where the asns are the same
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
                            Some(x) => { // yield a iter of announcements where the asns are the same
                                let ans = p_iter.peeking_take_while(|&y| {x.asn == y.asn}).collect_vec();
                                if ans.len() == 0 {continue;}
                                yield ans;
                            }
                        }


                    }
                }
            };
            pin_mut!(asn_group_gen);

            let wclient = reqwest::Client::new(); // requesting client for Seclytics API

            let mut asn_count = 0;
            let mut bad_asn_count = 0;
            while let Some(asn_group) = asn_group_gen.next().await {
                asn_count += 1;

                let cidrs = asn_group.iter().map(|x| x.prefix).collect_vec();
                let (bad_cidr, bad_asn) =
                    asn_is_malicious(asn_group[0].asn, cidrs, &wclient).await?;

                if bad_asn {
                    bad_asn_count += 1;
                }

                if bad_cidr == 0 {
                    continue;
                }
                if !bad_asn {
                    warn!("Unreported AS{} has {bad_cidr} short lived malicious announcement of ({}% of announcements)",
                        asn_group[0].asn,
                        bad_cidr as f64 / asn_group.len() as f64,
                    )
                } else {
                    info!(
                        "Malicious AS{}: \t{}% of {} ({bad_cidr} bad)",
                        asn_group[0].asn,
                        bad_cidr as f64 / asn_group.len() as f64,
                        asn_group.len()
                    );
                }
            }
            info!("{}/{} Seclytics/ASNs", bad_asn_count, asn_count); //number_of_rows_in_window(1660687200,1660694499, &pool).await?
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
        Job::Test => {}
    }

    Ok(())
}

async fn reload_data(pool: PgPool) -> Result<()> {
    let (sender, receiver) = bounded::<Vec<u8>>(0);

    let handle1 = tokio::task::spawn_blocking(move || {
        parse_bgp(collect_bgp(1_660_751_400, 1_660_773_600), sender)?;
        anyhow::Ok(())
        // 15 min of data 1692223200 1692223953
        // 1 hours 1692226800
        // 2 hours 1692230400 <--
        // 1 day 1692309600
        // 3 days 1692482400

        // 1660687200
        // 1660773600 1 day
        // 1661032800
    });
    let handle2: tokio::task::JoinHandle<_>;
    {
        let pool = pool.clone();
        handle2 = tokio::task::spawn(async move {
            'Outer: loop {
                let mut conn = pool.acquire().await?;
                let mut cpin = conn
                    .copy_in_raw(&*format!(
                        "COPY Announcement_new FROM STDIN (DELIMITER '{DELIMITER}', FORMAT csv)"
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
            }
            anyhow::Ok(())
        });
    }
    handle1
        .await
        .with_context(|| ">>> Parsing BGP data panicked")??;
    handle2
        .await
        .with_context(|| ">>> Saving BGP data panicked")??;

    {
        use std::time::Instant;
        let now = Instant::now();

        sqlx::query!("alter table Announcement_new add primary key (id)")
            .execute(&pool)
            .await?;
        info!(">>> Added p key in: {:.2?}", now.elapsed());

        let now = Instant::now();
        sqlx::query!("CREATE INDEX ASN on Announcement_new (asn)")
            .execute(&pool)
            .await?;
        info!(">>> Added asn index in: {:.2?}", now.elapsed());

        let now = Instant::now();
        sqlx::query!("CREATE INDEX WD on Announcement_new (withdrawal)")
            .execute(&pool)
            .await?;
        info!(">>> Added wd index in: {:.2?}", now.elapsed());

        let now = Instant::now();
        sqlx::query!("CREATE INDEX TS on Announcement_new (timestamp)")
            .execute(&pool)
            .await?;
        info!(">>> Added ts index in: {:.2?}", now.elapsed());
    }
    Ok(())
}
// 15 minutes of data | 287 MB on disk | 42 sec || 0.04666666667 time ratio

// 1 day | 28GB | ~ 1 hour

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
        .level_for("hyper", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;

    debug!("finished setting up logging!");

    Ok(())
}
