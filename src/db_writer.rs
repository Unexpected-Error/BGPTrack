pub(crate) mod types {
    // Import Special Types
    use ipnetwork::IpNetwork;
    pub(crate) type UnixTimeStamp = i32;
    
    // sqlx stuff
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
    pub(crate) static DELIMITER: &str = ","; // helps keep delimiter constant between copy command and display impl
    use std::{fmt, ops}; // a bunch of hacky stuff
    
    #[allow(dead_code)]
    #[derive(sqlx::FromRow, Debug, Clone, PartialEq, PartialOrd)]
    pub(crate) struct Announcement {
        pub(crate) id: uuid::Uuid,
        pub(crate) asn: i64,
        pub(crate) withdrawal: bool,
        pub(crate) timestamp: f64,
        pub(crate) prefix: IpNetwork,
        pub(crate) as_path_segments: Vec<ASPathSeg>,
    }
    impl ops::Deref for APSegments {
        type Target = Vec<ASPathSeg>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    pub(crate) struct APSegments(pub(crate) Vec<ASPathSeg>);
    impl fmt::Display for APSegments {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "\"{{{}}}\"",
                self.iter()
                    .map(|x| format!("{}", x))
                    .collect::<Vec<String>>()
                    .join(DELIMITER)
            )
        }
    }

    #[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
    #[sqlx(type_name = "as_path_segment")]
    pub(crate) struct ASPathSeg {
        pub(crate) seq: bool,
        pub(crate) confed: bool,
        pub(crate) as_path: Vec<i64>,
    }

    impl PgHasArrayType for ASPathSeg {
        fn array_type_info() -> PgTypeInfo {
            PgTypeInfo::with_name("_as_path_segment")
        }
    }
    impl fmt::Display for ASPathSeg {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let tmp = self
                .as_path
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(&*("\\".to_string() + DELIMITER));
            write!(
                f,
                "({}\\{DELIMITER}{}\\{DELIMITER}\\\"\"\\{{{}\\}}\\\"\")",
                self.seq, self.confed, tmp
            )
        }
    }
}

// types
use std::net::IpAddr;
use ipnetwork::IpNetwork;
use crate::db_writer::types::{ASPathSeg, Announcement, UnixTimeStamp};
use time::OffsetDateTime;

// errors, logs, tools, etc
use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use async_stream::{stream, try_stream};
use fix_hidden_lifetime_bug::fix_hidden_lifetime_bug;
use futures::{pin_mut, Stream};
use futures::stream::StreamExt;


lazy_static! {
    pub(crate) static ref PG_URL: String = {
        use dotenvy::{dotenv};
        dotenv().expect(".env file not found");
        dotenvy::var("DATABASE_URL").expect("DATABASE_URL not found")
    };
}

pub(crate) async fn open_db() -> Result<sqlx::PgPool, anyhow::Error> {

    debug!("Spinning up db conn...");
    
    let pool = sqlx::postgres::PgPool::connect(&**PG_URL)
        .await
        .context("Failed while connecting to pg db")?;
    
    warn!("Running migrations...");
    
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed while migrating")?;
    
    Ok(pool)
}

/// Given a PG database Pool [`sqlx::PgPool`], drops all AS (Oranges)
/// WARNING: GIVES NO WARNING BEFORE DELETING ALL DATA
///
/// # Examples
/// ```
/// delete_all(&pool).await? // Bye bye data
/// `
#[allow(unreachable_code, unused)]
pub(crate) async fn delete_all(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    panic!("GO AWAY, ITS HAPPENED TOO MANY TIMES");

    {
        warn!("Truncating Announcement Table");
    }
    sqlx::query("TRUNCATE Announcement").execute(pool).await?;
    Ok(())
}

/// Given a ip address [`IpAddr`] and a PG database Pool [`sqlx::PgPool`], finds if any announcements relating to that ip
/// # Examples
/// ```
/// search("1.0.0.1".parse()?, &pool).await? // Any announcements for a prefix containing 1.0.0.1?
/// ```
pub(crate) async fn ip_search(
    ip: IpAddr,
    pool: &sqlx::PgPool,
) -> Result<Vec<Announcement>, sqlx::Error> {
    let res = sqlx::query_as!(
        Announcement,
        r#"SELECT id, asn, withdrawal, timestamp, prefix, as_path_segments as "as_path_segments: Vec<ASPathSeg>" FROM Announcement as a WHERE (a.prefix >> $1) AND a.withdrawal = false;"#,
        IpNetwork::from(ip)
    )
        .fetch_all(pool)
        .await?;
    Ok(res)
}
/*
/// Enum with impl'd fn for all ways of processing a iter of [`Announcement`]
///
/// &self.process() handles picking the fn for the given enum
#[derive(Subcommand)]
pub(crate) enum Processor {
    /// If the same asn announced and withdrew the same prefix multiple times,
    /// get the overall time range that asn was active with the prefix
    OverallTimeRange,
    Raw
}
impl Processor {
    /// Picks correct fn for the given variant of [`Processor`]
    pub(crate) fn process<S: Stream<Item = Vec<ShortLivedRec>>>(input: S)
    -> impl Stream<Item = u32> {
        match self {
            Processor::OverallTimeRange => Self::overall_time_range(thing),
            Processor::Raw => Self::collect(thing)
        }
    }
    fn collect(
        thing: impl Iterator<Item = (IpNetwork, OffsetDateTime, OffsetDateTime, i64)>,
    ) -> Vec<(IpNetwork, OffsetDateTime, OffsetDateTime, i64)> {
        thing
            .collect::<Vec<(IpNetwork, OffsetDateTime, OffsetDateTime, i64)>>()
    }
    // Same prefix and asn, but different ann pointing to the same withdraw? Merge all ann/withdraws from a asn about a prefix
    fn overall_time_range(
        thing: impl Iterator<Item = (IpNetwork, OffsetDateTime, OffsetDateTime, i64)>,
    ) -> Vec<(IpNetwork, OffsetDateTime, OffsetDateTime, i64)> {
        thing
            .sorted_by_key(|&data| data.0)
            .coalesce(|x, y| {
                if x.0 == y.0 && x.3 == y.3 && !(x.1 == y.1 && x.2 == y.2) {
                    debug!("COLLAPSED\n{:?} and\n{:?}", x, y);
                    Ok((y.0, y.1.min(x.1), y.2.max(x.2), y.3))
                } else {
                    Err((x, y))
                }
            })
            .collect::<Vec<(IpNetwork, OffsetDateTime, OffsetDateTime, i64)>>()
    }
}*/

/// Collects all short lived announcements and runs a [`Processor`] on them, returning the results 
/// MAKE SURE TO PIN FOR USE
/// pin_mut!(n);
pub(crate) async fn find_short_lived(
    window: UnixTimeStamp,
    start: UnixTimeStamp,
    stop: UnixTimeStamp,
    limit: Option<i64>,
    yield_window: i32,
    // processor: Processor,
    pool: &sqlx::PgPool,
) -> impl Stream< Item = Result<PotentialHijack/*(IpNetwork, OffsetDateTime, OffsetDateTime, i64)*/> > + '_ {
    try_stream! {
        debug!("Start: {start}, Stop: {stop}, chunk size: {}", yield_window as usize);
        for mut i in &((start+1)..stop).chunks(yield_window as usize) {
            // debug!("Chunk: {i:?}");
            let sub_start = i.next();
            let mut sub_stop = i.last();
            
            debug!("Short lived sub-query window is now between {:?} and {:?}", sub_start, sub_stop );
            
            // rare edge case where the last chunk is exactly 1 sec and thus the iter only has one value to yield
            if sub_start.is_some() && sub_stop.is_none() { 
                sub_stop = sub_start
            }
            
            for potential in PotentialHijack::query_short_lived_window(window, (sub_start.ok_or(anyhow!("No start time"))?-1), sub_stop.ok_or(anyhow!(""))?, limit, pool).await? {
                yield potential;
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PotentialHijack {
    pub(crate) prefix: IpNetwork,
    pub(crate) ann_time: OffsetDateTime,
    pub(crate) wd_time: OffsetDateTime,
    pub(crate) asn: i64
}

impl PotentialHijack {
    async fn query_short_lived_window(window: UnixTimeStamp, start: UnixTimeStamp, stop: UnixTimeStamp, limit: Option<i64>, pool: &sqlx::PgPool) -> Result<Vec<Self>> {
        debug!("Running sub-window of short lived query");
    use std::time::Instant;
    let now = Instant::now();
    let tmp = 
        if let Some(n) = limit {
        sqlx::query_as!(PotentialHijack, 
            r#"
SELECT
      a1.asn                          AS asn,
      a1.prefix                       AS prefix,
      to_timestamp(MIN(a2.timestamp)) AS "wd_time!",
      to_timestamp(a1.timestamp)      AS "ann_time!"
FROM Announcement AS a1
        JOIN Announcement AS a2 ON a1.prefix = a2.prefix
   AND a1.asn = a2.asn
   AND a2.withdrawal = true
   AND ABS(a1.timestamp - a2.timestamp) < $1
   AND a2.timestamp > a1.timestamp
WHERE a1.withdrawal = FALSE
AND a2.timestamp < $2
AND a1.timestamp < $3
AND a2.timestamp >= $4
AND a1.timestamp >= $5
GROUP BY a1.id,
        a1.asn,
        a1.prefix,
        a1.as_path_segments,
        a1.timestamp
LIMIT $6
"#,
            f64::from(window),
            f64::from(stop + window), // beyond the window, no valid withdraws are present
            f64::from(stop),            // end of ann window
            f64::from(start),           // start of ann window, withdraws may be immediate
            f64::from(start),
            (n)
        )
            .fetch_all(pool)
            .await?
    } else {
        sqlx::query_as!(PotentialHijack, 
            r#"
SELECT
      a1.asn                          AS asn,
      a1.prefix                       AS prefix,
      to_timestamp(MIN(a2.timestamp)) AS "wd_time!",
      to_timestamp(a1.timestamp)      AS "ann_time!"
FROM Announcement AS a1
        JOIN Announcement AS a2 ON a1.prefix = a2.prefix
   AND a1.asn = a2.asn
   AND a2.withdrawal = true
   AND ABS(a1.timestamp - a2.timestamp) < $1
   AND a2.timestamp > a1.timestamp
WHERE a1.withdrawal = FALSE
AND a2.timestamp < $2
AND a1.timestamp < $3
AND a2.timestamp >= $4
AND a1.timestamp >= $5
GROUP BY a1.id,
        a1.asn,
        a1.prefix,
        a1.as_path_segments,
        a1.timestamp
"#,
            f64::from(window),
            f64::from(stop + window), // beyond the window, no valid withdraws are present
            f64::from(stop),            // end of ann window
            f64::from(start),           // start of ann window, withdraws may be immediate
            f64::from(start),
        )
            .fetch_all(pool)
            .await?
    };
        let elapsed = now.elapsed();
        info!("Query took: {:.2?}", elapsed);
        Ok(tmp)
}
    pub(crate) async fn filter_by_known_ip(&self) -> bool {
        true
    }
}

