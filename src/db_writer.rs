pub(crate) mod types {
    use std::{fmt, ops};
    use std::fmt::Formatter;
    // Error Handling
    
    // Import Special Types
    use ipnetwork::IpNetwork;
    // Serde Derives
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
    use tokio::fs::write;
    pub static DELIMITER: &str = ",";
    #[derive(sqlx::FromRow, Debug, Clone)]
    pub struct Announcement {
        pub(crate) id: uuid::Uuid,
        pub(crate) asn: i64,
        pub(crate) withdrawal: bool,
        pub(crate) timestamp: f64,
        pub(crate) prefix: IpNetwork,
        pub(crate) as_path_segments: Vec<ASPathSeg>,
    }
    //r#"1cc8b0cf-fa07-40e3-ba47-6c71522c3fdd,65000,f,0,1.0.0.0/8,"{   (f\,t\,\""\{1\,2324\,42\}\""),  (f\,t\,\""\{1\,24\,42\}\"")}""#.to_string();
    impl ops::Deref for AP_Segments {
        type Target = Vec<ASPathSeg>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    pub struct AP_Segments(pub Vec<ASPathSeg>);
    impl fmt::Display for AP_Segments {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "\"{{{}}}\"", self.iter().map(|x| format!("{}", x)).collect::<Vec<String>>().join(DELIMITER))
            // let initial = write!(f, "{{");
            // self.0.iter().fold(initial, |result, apseg| {
            //     result.and_then(|_| write!(f, "({}),", apseg))
            // }).and_then(|_| write!(f, "}}"))
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
    impl fmt::Display for ASPathSeg {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            let tmp = self.as_path.iter().map(|x|{x.to_string()}).collect::<Vec<String>>().join(&*("\\".to_string() + DELIMITER));
            write!(f, "({}\\{DELIMITER}{}\\{DELIMITER}\\\"\"\\{{{}\\}}\\\"\")", self.seq, self.confed, tmp)
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


// errors && logs
use anyhow::{Context};
#[cfg(debug_assertions)]
use log::{info, warn};

const PG_URL: &str = "postgres://postgres:postgrespw@localhost:55000/BGP";

pub(crate) async fn open_db() -> Result<sqlx::PgPool, anyhow::Error> {
    // TODO: Add user/password args and pull from .env
    #[cfg(debug_assertions)]
    { info!("Spinning up db conn..."); }
    let pool = sqlx::postgres::PgPool::connect(PG_URL)
        .await
        .context("While connecting to pg db")?;
    #[cfg(debug_assertions)]
    { warn!("Migrating db..."); }
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("While migrating")?;
    Ok(pool)
}

// async fn create_announcement(ann: Announcement, pool: &sqlx::PgPool) -> Result<(), anyhow::Error> {
//     sqlx::query!(
//         r#"INSERT INTO Announcement VALUES ($1)"#,
//         ann as Announcement
//     )
//     .execute(pool)
//     .await
//     .context("here")?;
//     Ok(())
// }
// 
/// Given a PG database Pool [`sqlx::PgPool`], drops all AS (Oranges)
/// WARNING: GIVES NO WARNING BEFORE DELETING ALL DATA
///
/// # Examples
/// ```
/// delete_all(&pool).await? // Bye bye data
/// `
pub(crate) async fn delete_all(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    #[cfg(debug_assertions)]
    {
        warn!("Truncating Announcement Table");
    }    sqlx::query("TRUNCATE Announcement").execute(pool).await?;
    Ok(())
}

// Given a ip address [`IpAddr`] and a PG database Pool [`sqlx::PgPool`], finds if any AS (Orange) declares that ip
// # Examples
// ```
// search("1.0.0.1".parse()?, &pool).await? // Does any AS declare 1.0.0.1? Probably, Cloudflare owns 1.0.0.0/24
// ```
// pub(crate) async fn ip_search(ip: IpAddr,pool: &sqlx::PgPool) -> Result<Option<Orange>, sqlx::Error> {
//     let res = sqlx::query_as!(
//         Orange,
//         r#"SELECT asn, announcements as "announcements: Vec<Announcement>" FROM Orange WHERE EXISTS ( SELECT 1 FROM unnest(announcements) AS a WHERE (a.prefix >> $1));"#,
//         IpNetwork::from(ip)
//     )
//         .fetch_optional(pool)
//         .await?;
//     Ok(res)
// }

// Given a PG database Pool [`sqlx::PgPool`], finds if all AS (Orange)
//
// WARNING: Loads huge amounts of data into memory
// # Examples
// ```
// let allOranges: Vec<Orange> = getall(&pool).await? // Collect all Oranges
// ```
// pub(crate) async fn read_all(pool: &sqlx::PgPool) -> Result<Vec<Orange>, sqlx::Error> {
//     warn!("Loading all Oranges into memory...");
//     let latest_start_ann = sqlx::query_as!(
//         Orange,
//         r#"select asn, announcements as "announcements: Vec<Announcement>" from Orange"#
//     )
//     .fetch_all(pool)
//     .await?;
//     Ok(latest_start_ann)
// }
