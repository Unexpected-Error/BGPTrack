pub(crate) mod types {
    // Error Handling
    use std::error::Error;
    // Import Special Types
    use ipnetwork::IpNetwork;
    use rayon::prelude::*;
    // Serde Derives
    use serde::{Deserialize, Serialize};
    #[allow(non_camel_case_types)]
    pub type R_ASN = u32;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Orange {
        // check all places orange is used
        pub(crate) asn: i64,
        pub(crate) announcements: Vec<Announcement>,
    }

    pub struct RawAnnouncement {
        pub(crate) time: f64,
        pub(crate) announcing: bool,
        pub(crate) prefix: IpNetwork,
        pub(crate) as_path: Option<Vec<u32>>,
        pub(crate) as_path_is_seq: bool,
    }

    impl RawAnnouncement {
        pub fn to_announcement_as_new(self) -> Result<Announcement, Box<dyn Error>> {
            Ok(Announcement {
                start_time: match self.announcing {
                    true => Some(self.time),
                    false => None,
                },
                stop_time: match self.announcing {
                    false => Some(self.time),
                    true => None,
                },
                prefix: self.prefix,
                as_path: match self.announcing {
                    false => self
                        .as_path
                        .ok_or("NO AS-PATH WHEN ANNOUNCING")?
                        .par_iter()
                        .map(|&a| a as i64)
                        .collect(),
                    true => match self.as_path {
                        None => {
                            vec![]
                        }
                        Some(n) => n.par_iter().map(|&a| a as i64).collect(),
                    },
                },
                as_path_is_seq: self.as_path_is_seq,
            })
        }
    }

    #[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone)]
    #[sqlx(type_name = "announcement")]
    pub struct Announcement {
        pub(crate) start_time: Option<f64>,
        pub(crate) stop_time: Option<f64>,
        pub(crate) prefix: IpNetwork,
        pub(crate) as_path: Vec<i64>,
        pub(crate) as_path_is_seq: bool,
    }
}

use crate::db_writer::types::{Announcement, RawAnnouncement, R_ASN};
use anyhow::Context;
use log::{info, warn};
// use rayon::prelude::*;
// use sqlx::{Row};
use std::error::Error;
use std::net::IpAddr;
use ipnetwork::IpNetwork;
use types::Orange;

const PG_URL: &str = "postgres://postgres:postgrespw@localhost:55000/BGP";
const INSERT: &str = "INSERT INTO Orange (id, prefixes) VALUES ($1, $2)";
const DELETE_ALL: &str = "TRUNCATE Orange";

const OLD_IP_CHECK: &str = "SELECT id, prefixes FROM orange WHERE EXISTS (SELECT 1 FROM unnest(prefixes) AS p WHERE $1 << p)";
const IP_CHECK: &str =
    "SELECT * FROM Orange WHERE EXISTS ( SELECT 1 FROM unnest(prefix) AS p WHERE (p.prefixes >> $1)
);";

pub(crate) async fn open_db() -> Result<sqlx::PgPool, Box<dyn Error>> {
    // TODO: Add user/password args and pull from .env
    info!("Spinning up db conn...");
    let pool = sqlx::postgres::PgPool::connect(PG_URL)
        .await
        .context("While connecting to pg db")?;
    warn!("Migrating db...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("While migrating")?;

    Ok(pool)
}

async fn insert_new_ann(
    ann: Announcement,
    ASN: i64,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn Error>> {
    
    sqlx::query!(
        "UPDATE Orange SET announcements = array_append(announcements, $1) WHERE asn = $2",
        ann as _,
        ASN
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn insert(
    ann: (RawAnnouncement, R_ASN),
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn Error>> {
    let ASN = ann.1 as i64;
    let ann = ann.0;

    match ann.announcing {
        true => {
            info!("Inserting new announcement, prefix: {}, peer ASN: {}", ann.prefix, ASN);
            insert_new_ann(ann.to_announcement_as_new()?, ASN, pool).await?;
        }
        false => {
            info!("Looking for prev ann to withdraw... prefix: {}, peer ASN: {}", ann.prefix, ASN);

            let latest_start_ann: Option<Announcement> = sqlx::query_as!( 
                Announcement,
                r#"
                SELECT a.start_time, a.stop_time, a.prefix as "prefix!: IpNetwork", a.as_path as "as_path!: Vec<i64>", a.as_path_is_seq as "as_path_is_seq!: bool"
                FROM Orange AS o
                CROSS JOIN LATERAL (
                    SELECT a.start_time, a.stop_time, a.prefix, a.as_path, a.as_path_is_seq
                    FROM unnest(o.announcements) AS a
                    WHERE o.asn = $1 AND a.prefix = $2
                    ORDER BY a.start_time DESC
                    LIMIT 1
                ) AS a
                "#,
                ASN,
                ann.prefix
            )
            .fetch_optional(pool)
            .await?;
            
            if let Some(start_ann) = latest_start_ann {
                info!("\tFOUND, updating stop time for {:?}", start_ann);
                // An existing announcement with a matching prefix was found, update stop_time
                let mut new_ann = start_ann.clone();
                new_ann.stop_time = Some(ann.time); // Set your desired start_time
                sqlx::query!(
                    "UPDATE Orange SET announcements = array_replace(announcements, $1, $2) WHERE asn = $3",
                    start_ann as Announcement,
                    new_ann as Announcement,
                    ASN as i64)
                    .execute(pool)
                    .await?;
            } else {
                let n = ann.to_announcement_as_new()?;
                info!("No ann found, inserting as naked withdraw: {:?}", n);
                insert_new_ann(n, ASN, pool).await?;
            }
        }
    }
    // let res = sqlx::query(INSERT)
    //     .bind(&orange.ASN)
    //     .bind(&orange.prefixes)
    //     .execute(pool)
    //     .await?;
    Ok(())
}

pub(crate) async fn delete(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    warn!("Truncating Orange Table");
    sqlx::query(DELETE_ALL).execute(pool).await?;
    Ok(())
}

/// Given a ip address [`IpAddr`] and a PG database Pool [`sqlx::PgPool`], finds if any AS (Orange) declares that ip
/// # Examples
/// ```
/// search("1.0.0.1".parse()?, &pool).await? // Does any AS declare 1.0.0.1? Probably, Cloudflare owns 1.0.0.0/24
/// ```
pub(crate) async fn search(ip: IpAddr, pool: &sqlx::PgPool) -> Result<Option<Orange>, sqlx::Error> {
    todo!()
    // warn!("Truncating Orange Table");
    // let res = sqlx::query(IP_CHECK)
    //     .bind(ip)
    //     .fetch_optional(pool)
    //     .await?;
    // match res {
    //     None => { Ok(None) }
    //     Some(x) => {
    //         Ok(
    //             Some(
    //                 Orange {
    //                     ASN: x.get("id"),
    //                     prefixes: x.get("prefixes"),
    //                 }
    //             )
    //         )
    //     }
    // }
}
pub(crate) async fn getall(ip: IpAddr, pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    type tbl = Vec<Orange>;
    let latest_start_ann = sqlx::query_as!(
        Orange,
        r#"select asn, announcements as "announcements: Vec<Announcement>" from Orange"# // r#"
                                                                                         // SELECT a.start_time, a.stop_time, a.prefix, a.as_path, a.as_path_is_seq
                                                                                         // FROM Orange AS o
                                                                                         // CROSS JOIN LATERAL (
                                                                                         //     SELECT a.start_time, a.stop_time, a.prefix, a.as_path, a.as_path_is_seq
                                                                                         //     FROM unnest(o.announcements) AS a
                                                                                         //     WHERE o.asn = $1 AND a.prefix = $2
                                                                                         //     ORDER BY a.start_time DESC
                                                                                         //     LIMIT 1
                                                                                         // ) AS a
                                                                                         // "#,
                                                                                         // ASN,
                                                                                         // ann.prefix
    )
    .fetch_all(pool)
    .await?;
    Ok(())
}
pub(crate) async fn getasll(ip: IpAddr, pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    type tbl = Vec<Orange>;
    let latest_start_ann = sqlx::query_as!(
        Orange,
        r#"select asn, announcements as "announcements: Vec<Announcement>" from Orange"# // r#"
                                                                                         // SELECT a.start_time, a.stop_time, a.prefix, a.as_path, a.as_path_is_seq
                                                                                         // FROM Orange AS o
                                                                                         // CROSS JOIN LATERAL (
                                                                                         //     SELECT a.start_time, a.stop_time, a.prefix, a.as_path, a.as_path_is_seq
                                                                                         //     FROM unnest(o.announcements) AS a
                                                                                         //     WHERE o.asn = $1 AND a.prefix = $2
                                                                                         //     ORDER BY a.start_time DESC
                                                                                         //     LIMIT 1
                                                                                         // ) AS a
                                                                                         // "#,
                                                                                         // ASN,
                                                                                         // ann.prefix
    )
    .fetch_all(pool)
    .await?;
    Ok(())
}
