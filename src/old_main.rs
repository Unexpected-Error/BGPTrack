// #[macro_use]
// extern crate log;
// 
// use anyhow::{Result, Context};
// use bgpkit_parser::{BgpkitParser, BgpElem};
// use bgpkit_broker;
// use bgpkit_broker::BgpkitBroker;
// use serde::{Deserialize, Serialize};
// use surrealdb::engine::remote::ws::Ws;
// use surrealdb::opt::auth::Root;
// use surrealdb::sql::Thing;
// use surrealdb::{Connection, Surreal};
// use epoch_converter;
// use patricia_tree;
// use patricia_tree::StringPatriciaMap;
// use ipnet::IpNet;
// use surrealdb::sql::Kind::Record;
// 
// 
// fn init() {
//     env_logger::init();
//     info!("starting up");
// } // Init Logger, etc
// 
// fn collect_bgp(start: u64, end: u64) -> BgpkitBroker {
//     let broker = bgpkit_broker::BgpkitBroker::new()
//         .ts_start(start.to_string().as_str())
//         .ts_end(end.to_string().as_str());
//     info!("Total BGP files: {}", (&broker).into_iter().count());
//     return broker;
// }
// 
// #[tokio::main]
// async fn main() -> Result<()> {
//     init();
// 
//     let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;
// 
//     db.signin(Root {
//         username: "root",
//         password: "root",
//     }).await.context("Signing into surreal DB")?;
//     
//     // let mut map = StringPatriciaMap::new();
//     // map.insert("1100", "ASN6500");
//     // map.insert("1110", "ASN6501");
//     // let data = tokio::task::spawn_blocking(
//     //     || {
//     //         parse_bgp(&collect_bgp(1692223200, 1692243953))
//     //     }
//     // ).await.with_context(|| {"Collecting BGP data panicked"})?;
// 
//     
//     surreal_db(db).await?;
//     
//     Ok(())
// }
// 
// 
// fn parse_bgp(broker: &BgpkitBroker) -> Result<()>{
//     for item in broker {
//         log::info!("downloading updates file: {}", &item.url);
//         let parser = BgpkitParser::new(item.url.as_str()).unwrap();
//         log::info!("parsing updates file");
// 
//         // let elems = parser
//         //     .into_elem_iter()
//         //     .filter_map(|elem| {
//         //         if let Some(origins) = &elem.origin_asns {
//         //             if origins.contains(&13335.into()) {
//         //                 Some(elem)
//         //             } else {
//         //                 None
//         //             }
//         //         } else {
//         //             None
//         //         }
//         //     })
//         //     .collect::<Vec<BgpElem>>();
//         for elem in parser {
//             println!("\tELEMENT:\n{:?}", elem);
//             return Ok(())
//         }
//     }
//     Ok(())
// }
// 
// //     
// //     let broker = bgpkit_broker::BgpkitBroker::new() // timestamp
// //         .ts_start("1634693400")
// //         .ts_end("1692243953");
// //     
// //     /// ANNOUNCE <Range> -> search prefix tree - > if ASN, 
// //     ///  collect ASN1 ASN2 time1, time2, prefix
// //     /// ANNOUNCE <Range> - > prefix tree - > ASN,timestamp
// //     /// 
// //     /// for remove <Range> - > seek prefix in tree - > if ASN & & time < 2m:
// //     ///  collect ASN, start, end, prefix
// //     /// 
// //     /// prefix tree - > ASN , ASN {declared, removed
// //     /// ANNOUNCE <Range>
// //     /// REMOVE <Range>
// //     /// 
// //     /// 
// //     /// 
// //     /// 
// //     /// 
// //     /// 
// //     /// 
// //     /// 
// //     /// 
// //     /// 
// // 
// //     for item in &broker {
// //         log::info!("downloading updates file: {}", &item.url);
// //         let parser = BgpkitParser::new(item.url.as_str()).unwrap();
// //         log::info!("parsing updates file");
// //         
// // // iterating through the parser. the iterator returns `BgpElem` one at a time.
// // //         let elems = parser
// // //             .into_elem_iter()
// // //             .filter_map(|elem| {
// // //                 if let Some(origins) = &elem.origin_asns {
// // //                     if origins.contains(&13335.into()) {
// // //                         Some(elem)
// // //                     } else {
// // //                         None
// // //                     }
// // //                 } else {
// // //                     None
// // //                 }
// // //             })
// // //             .collect::<Vec<BgpElem>>();
// //         for elem in parser {
// //             println!("\tELEMENT:\n{:?}", elem);
// //             if map.get(elem.prefix.prefix.to_string()).is_none() {
// //                 let n = elem.prefix.prefix
// //             } 
// //             return Ok(())
// //         }
// //         // log::info!("{} elems matches", elems.len());
// //     }
// //     
// //      Ok(())
// // } 
// // 
// // 
// // 
// #[derive(Debug, Serialize, Deserialize)]
// struct AS {
//     ASN: u32,
//     AS_PATH: Vec<u32>
// }
// #[derive(Debug, Deserialize)]
// struct Entry {
//     id: Thing
// }
// #[derive(Debug, Serialize)]
// struct Name<'a> {
//     first: &'a str,
//     last: &'a str,
// }
// 
// #[derive(Debug, Serialize)]
// struct Person<'a> {
//     title: &'a str,
//     name: Name<'a>,
//     marketing: bool,
// }
// 
// #[derive(Debug, Serialize)]
// struct Responsibility {
//     marketing: bool,
// }
// 
// #[derive(Debug, Deserialize)]
// struct oRecord {
//     #[allow(dead_code)]
//     id: Thing,
// }
// async fn surreal_db<C: Connection>(db: Surreal<C>) -> surrealdb::Result<()> {
// 
// 
//     // Select a specific namespace / database
//     db.use_ns("default").use_db("test").await?;
//     let cmt: Entry = db.create("Orange").content(
//         AS {
//             ASN: 65000u32,//"ASN6500".to_string()
//             AS_PATH: vec![65001, 65000]
//         }
//     ).await?;
//     
//     info!("New AS:\t{:?}", cmt);
//     
//     
//     let oranges: Vec<Entry> = db.select(("Orange")).await?;
//     
//     dbg!(oranges);
//     // 
//     // let n: Vec<AS> = db.delete("Orange").await?;
//     // 
//     // info!("Deleted AS count:{}", n.into_iter().count());
//     let people: Vec<Person> = 
//         db.delete("person").await?;
//     // Create a new person with a random id
//     // let created: ORecord = db
//     //     .create("person")
//     //     .content(Person {
//     //         title: "Founder & CEO",
//     //         name: Name {
//     //             first: "Tobie",
//     //             last: "Morgan Hitchcock",
//     //         },
//     //         marketing: true,
//     //     })
//     //     .await?;
//     // dbg!(created);
//     // 
//     // // Update a person record with a specific id
//     // let updated: ORecord = ORecord::from(db
//     //     .update(("person", "jaime"))
//     //     .merge(Responsibility { marketing: true })
//     //     .await?);
//     // dbg!(updated);
//     // 
//     // // Select all people records
//     // let people: Vec<ORecord> = db.select("person").await?;
//     // dbg!(people);
//     // 
//     // // Perform a custom advanced query
//     // let groups = db
//     //     .query("SELECT marketing, count() FROM type::table($table) GROUP BY marketing")
//     //     .bind(("table", "person"))
//     //     .await?;
//     // dbg!(groups);
//     
//     Ok(())
// }

use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::sql::Thing;
use surrealdb::Surreal;

#[derive(Debug, Serialize, Deserialize)]
struct Name<'a> {
    first: &'a str,
    last: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
struct Person<'a> {
    title: &'a str,
    name: Name<'a>,
    marketing: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Responsibility {
    marketing: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    #[allow(dead_code)]
    id: Thing,
}
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: String,
    balance: String
}

#[tokio::main]
async fn main() -> surrealdb::Result<()> {
    let db = Surreal::new::<Ws>("127.0.0.1:8000").await?;
    db.signin(Root {
        username: "root",
        password: "root",
    })
        .await?;
    db.use_ns("test").use_db("test").await?;
    let created: Record = db
        .create("Users")
        .content(
            User {
                id: "alskjdasdaasd".to_string(),
                balance: "0".to_string(),
            }
        )
        .await?;
    // dbg!(created);
    
    let mut res = db.query("Select * From Users;").query("SELECT * FROM Users WHERE balance = '0'").await?;
    let all: Vec<Record> = res.take(0)?;
    for n in all {
        
    }
    dbg!(all);
    
    let idk = db.query("Delete Users Return before").await?;
    // dbg!(idk);
    Ok(())
}

//    let people: Vec<Record> = db.select("Users").await?;
//     dbg!(people);
//[src/main.rs:318] res = Response(
//     {
//         0: Ok(
//             [
//                 Object(
//                     Object(
//                         {
//                             "id": Thing(
//                                 Thing {
//                                     tb: "person",
//                                     id: String(
//                                         "c8cjp7tdyceyf9c6jcec",
//                                     ),
//                                 },
//                             ),
//                             "marketing": True,
//                             "name": Object(
//                                 Object(
//                                     {
//                                         "first": Strand(
//                                             Strand(
//                                                 "Tobie",
//                                             ),
//                                         ),
//                                         "last": Strand(
//                                             Strand(
//                                                 "Morgan Hitchcock",
//                                             ),
//                                         ),
//                                     },
//                                 ),
//                             ),
//                             "title": Strand(
//                                 Strand(
//                                     "Founder & CEO",
//                                 ),
//                             ),
//                         },
//                     ),
//                 ),
//                 Object(
//                     Object(
//                         {
//                             "id": Thing(
//                                 Thing {
//                                     tb: "person",
//                                     id: String(
//                                         "jaime",
//                                     ),
//                                 },
//                             ),
//                             "marketing": True,
//                         },
//                     ),
//                 ),
//             ],
//         ),
//     },
// )