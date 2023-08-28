pub(crate) mod types {
    // Error Handling
    use anyhow::{anyhow, Result};
    // Import Special Types
    use ipnetwork::IpNetwork;
    // Serde Derives
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::{PgHasArrayType, PgTypeInfo};

    #[allow(non_camel_case_types)]
    pub type R_ASN = u32;

    pub struct RawAnnouncement {
        pub(crate) time: f64,
        pub(crate) announcing: bool,
        pub(crate) prefix: IpNetwork,
        pub(crate) as_path_segments: Option<Vec<ASPathSeg>>,
    }

    impl RawAnnouncement {
        pub fn to_announcement_as_new(self) -> Result<Announcement> {
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
                as_path_segments: match self.announcing {
                    true => self
                        .as_path_segments
                        .ok_or(anyhow!("NO AS-PATH WHEN ANNOUNCING"))?,
                    false => match self.as_path_segments {
                        None => {
                            vec![]
                        }
                        Some(n) => n,
                    },
                },
            })
        }
    }

    #[derive(sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
    pub struct Orange {
        // check all places orange is used
        pub(crate) asn: i64,
        pub(crate) announcements: Vec<Announcement>,
    }

    #[derive(sqlx::Type, sqlx::FromRow, Debug, Serialize, Deserialize, Clone)]
    #[sqlx(type_name = "announcement")]
    pub struct Announcement {
        pub(crate) start_time: Option<f64>,
        pub(crate) stop_time: Option<f64>,
        pub(crate) prefix: IpNetwork,
        pub(crate) as_path_segments: Vec<ASPathSeg>,
    }

    impl PgHasArrayType for Announcement {
        fn array_type_info() -> PgTypeInfo {
            PgTypeInfo::with_name("_announcement")
        }
    }

    #[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone)]
    #[sqlx(type_name = "as_path_segment")]
    pub struct ASPathSeg {
        pub(crate) seq: bool,
        pub(crate) confed: bool,
        pub(crate) as_path: Vec<i64>,
    }

    impl PgHasArrayType for ASPathSeg {
        fn array_type_info() -> PgTypeInfo {
            PgTypeInfo::with_name("_as_path_segment")
        }
    }

    // #[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone)]
    // #[sqlx(type_name = "as_path_w")]
    // pub struct segar {
    //     a_p: Vec<ASPathSeg>,
    // }
    //
    // impl PgHasArrayType for segar {
    //     fn array_type_info() -> PgTypeInfo {
    //         PgTypeInfo::with_name("_as_path_seg_w")
    //     }
    // }
    //
    // impl Deref for segar {
    //     type Target = Vec<ASPathSeg>;
    //
    //     fn deref(&self) -> &Self::Target {
    //         &self.a_p
    //     }
    // }
    //
    // impl DerefMut for segar {
    //     fn deref_mut(self: &mut segar) -> &mut Vec<ASPathSeg> {
    //         &mut self.a_p
    //     }
    // }
    //
    // impl From<Vec<ASPathSeg>> for segar {
    //     fn from(value: Vec<ASPathSeg>) -> Self {
    //         Self { a_p: value }
    //     }
    // }
}

//C reate --> Make new
//R ead   --> Collect
//U date  --> Change
//D elete --> Remove
// -------------------
//Insert  --> Autopick between C and U

// types
use ipnetwork::IpNetwork;
use std::net::IpAddr;
use types::{ASPathSeg, Announcement, Orange, RawAnnouncement, R_ASN};
// errors && logs
use anyhow::{anyhow, Context};
use log::{info, warn};
const PG_URL: &str = "postgres://postgres:postgrespw@localhost:55000/BGP2";
const DELETE_ALL: &str = "TRUNCATE Orange";

pub(crate) async fn open_db() -> Result<sqlx::PgPool, anyhow::Error> {
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

async fn create_announcement(
    ann: Announcement,
    ASN: i64,
    pool: &sqlx::PgPool,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        "UPDATE Orange SET announcements = array_append(announcements, $2) WHERE asn = $1",
        ASN,
        ann as Announcement
    )
    .execute(pool)
    .await
    .context("here")?;
    Ok(())
}

async fn update_announcement(
    ann_to_update: Announcement,
    new_ann: Announcement,
    ASN: i64,
    pool: &sqlx::PgPool,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        "UPDATE Orange SET announcements = array_replace(announcements, $1, $2) WHERE asn = $3",
        ann_to_update as Announcement,
        new_ann as Announcement,
        ASN as i64
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn create_orange(orange: Orange, pool: &sqlx::PgPool) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"INSERT INTO Orange (asn, announcements) VALUES ($1, $2)"#,
        orange.asn,
        orange.announcements as Vec<Announcement>
    )
    .execute(pool)
    .await
    .context("here")?;
    Ok(())
}

pub(crate) async fn insert_announcement(ann: (RawAnnouncement, R_ASN),pool: &sqlx::PgPool) -> Result<(), anyhow::Error> {
    let ASN = ann.1 as i64;
    let ann = ann.0;

    match ann.announcing {
        true => {
            info!(
                "Inserting new announcement, prefix: {}, peer ASN: {}",
                ann.prefix, ASN
            );
            match sqlx::query!(
                r#"select exists(select 1 from orange where asn=$1) AS "exists""#,
                ASN
            )
            .fetch_one(pool)
            .await?
            .exists
            .ok_or(anyhow!("Didn't find exists bool"))?
            {
                true => create_announcement(ann.to_announcement_as_new()?, ASN, pool).await?,
                false => {
                    create_orange(
                        Orange {
                            asn: ASN,
                            announcements: vec![ann.to_announcement_as_new()?],
                        },
                        pool,
                    )
                    .await?
                }
            }
        }
        false => {
            info!(
                "Looking for prev ann to withdraw... prefix: {}, peer ASN: {}",
                ann.prefix, ASN
            );

            match sqlx::query_as!(
                Announcement,
                r#"
                SELECT a.start_time, a.stop_time, a.prefix as "prefix!: IpNetwork", a.as_path_segments as "as_path_segments!: Vec<ASPathSeg>"
                FROM Orange AS o
                CROSS JOIN LATERAL (
                    SELECT a.start_time, a.stop_time, a.prefix, a.as_path_segments
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
                .await? {
                None => {
                    let n = ann.to_announcement_as_new()?;
                    info!("No ann found, inserting as naked withdraw: {:?}", n);
                    match sqlx::query!(
                        r#"select exists(select 1 from orange where asn=$1) AS "exists""#,
                        ASN
                    )
                        .fetch_one(pool)
                        .await?
                        .exists
                        .ok_or(anyhow!("Didn't find exists bool"))?
                    {
                        true => {create_announcement(n, ASN, pool).await?}
                        false => {create_orange(Orange { asn: ASN, announcements: vec![n] }, pool).await?}
                    }
                }
                Some(start_ann) => {
                    info!("\tFOUND, updating stop time for {:?}", start_ann);
                    // An existing announcement with a matching prefix was found, update stop_time
                    let mut new_ann = start_ann.clone();
                    new_ann.stop_time = Some(ann.time); // Set your desired start_time
                    update_announcement(start_ann, new_ann, ASN, pool).await?;
                }
            }
        }
    }
    
    Ok(())
}

/// Given a PG database Pool [`sqlx::PgPool`], drops all AS (Oranges)
/// WARNING: GIVES NO WARNING BEFORE DELETING ALL DATA
///
/// # Examples
/// ```
/// delete_all(&pool).await? // Bye bye data
/// `
pub(crate) async fn delete_all(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    warn!("Truncating Orange Table");
    sqlx::query(DELETE_ALL).execute(pool).await?;
    Ok(())
}

/// Given a ip address [`IpAddr`] and a PG database Pool [`sqlx::PgPool`], finds if any AS (Orange) declares that ip
/// # Examples
/// ```
/// search("1.0.0.1".parse()?, &pool).await? // Does any AS declare 1.0.0.1? Probably, Cloudflare owns 1.0.0.0/24
/// ```
pub(crate) async fn ip_search(ip: IpAddr,pool: &sqlx::PgPool) -> Result<Option<Orange>, sqlx::Error> {
    let res = sqlx::query_as!(
        Orange,
        r#"SELECT asn, announcements as "announcements: Vec<Announcement>" FROM Orange WHERE EXISTS ( SELECT 1 FROM unnest(announcements) AS a WHERE (a.prefix >> $1));"#,
        IpNetwork::from(ip)
    )
        .fetch_optional(pool)
        .await?;
    Ok(res)
}

/// Given a PG database Pool [`sqlx::PgPool`], finds if all AS (Orange)
///
/// WARNING: Loads huge amounts of data into memory
/// # Examples
/// ```
/// let allOranges: Vec<Orange> = getall(&pool).await? // Collect all Oranges
/// ```
pub(crate) async fn read_all(pool: &sqlx::PgPool) -> Result<Vec<Orange>, sqlx::Error> {
    warn!("Loading all Oranges into memory...");
    let latest_start_ann = sqlx::query_as!(
        Orange,
        r#"select asn, announcements as "announcements: Vec<Announcement>" from Orange"#
    )
    .fetch_all(pool)
    .await?;
    Ok(latest_start_ann)
}
