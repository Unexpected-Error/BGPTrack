pub(crate) mod types {
    use ipnetwork::IpNetwork;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Orange {
        //<const n: usize>
        pub(crate) ASN: i32,
        pub(crate) prefixes: Vec<IpNetwork>,
    }
}

use types::Orange;
use std::error::Error;
use anyhow::Context;
use log::{info, warn};
use std::net::IpAddr;
use sqlx::Row;

const PG_URL: &str = "postgres://postgres:postgrespw@localhost:55000/BGP";
const INSERT: &str = "INSERT INTO Orange (id, prefixes) VALUES ($1, $2)";
const DELETE_ALL: &str = "TRUNCATE Orange";
const IP_CHECK: &str = "SELECT id, prefixes FROM orange WHERE EXISTS (SELECT 1 FROM unnest(prefixes) AS p WHERE $1 << p)";


pub(crate) async fn open_db() -> Result<sqlx::PgPool, Box<dyn Error>> {
    // TODO: Add user/password args and pull from .env 
    info!("Spinning up db conn...");
    let pool = sqlx::postgres::PgPool::connect(PG_URL).await.context("While connecting to pg db")?;
    warn!("Migrating db...");
    sqlx::migrate!("./migrations").run(&pool).await.context("While migrating")?;

    Ok(pool)
}


pub(crate) async fn create(orange: &Orange, pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    info!("Inserting\n{:#?}", orange);
    let res = sqlx::query(INSERT)
        .bind(&orange.ASN)
        .bind(&orange.prefixes)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn delete(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    warn!("Truncating Orange Table");
    let res = sqlx::query(DELETE_ALL)
        .execute(pool)
        .await?;
    Ok(())
}

/// Given a ip address [`IpAddr`] and a PG database Pool [`sqlx::PgPool`], finds if any AS (Orange) declares that ip
/// # Examples
/// ```
/// search("1.0.0.1".parse()?, &pool).await? // Does any AS declare 1.0.0.1? Probably, Cloudflare owns 1.0.0.0/24
/// ```
pub(crate) async fn search(ip: IpAddr, pool: &sqlx::PgPool) -> Result<Option<Orange>, sqlx::Error> {
    warn!("Truncating Orange Table");
    let res = sqlx::query(IP_CHECK)
        .bind(ip)
        .fetch_optional(pool)
        .await?;
    match res {
        None => { Ok(None) }
        Some(x) => {
            Ok(
                Some(
                    Orange {
                        ASN: x.get("id"),
                        prefixes: x.get("prefixes"),
                    }
                )
            )
        }
    }
}