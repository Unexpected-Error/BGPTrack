#![feature(future_join)]
// use std::future::join;
// use std::io::Read;
// 
// // Logs and Errors
// use anyhow::{Context, Result};
// use async_ringbuf::{AsyncHeapRb};
// #[allow(unused_imports)]
// use log::{info, warn};
// 
// // DB
// #[allow(unused_imports)]
// use sqlx::{Connection, Row};
// use tokio::io::{AsyncRead, AsyncReadExt, BufReader, ReadBuf};
// use uuid::Uuid;
// mod db_writer;
// 
// use db_writer::{open_db};
// // use crate::bgp::{collect_bgp, parse_bgp};
// use crate::db_writer::delete_all;
// 
// // BGP
// mod bgp;
// 
// // use crate::db_writer::{insert_announcement};
// // use bgp::{collect_bgp, parse_bgp};
// 
// 
// // let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
// 
// #[tokio::main]
// async fn main() -> Result<()> {
//     
//     env_logger::init();
//     let pool = open_db().await?;
//     delete_all(&pool).await?;
//     
//     // let data = tokio::task::spawn_blocking(move || {
//     //     parse_bgp(collect_bgp(1692223200, 1692223953), prod)
//     //     // 20230816.2200
//     //     //1692811618
//     //     //1692243953
//     //     //1692206400
//     // })
//     // .await
//     // .with_context(|| "Collecting BGP data panicked")??;    
//     // let mut a: Vec<u8> = vec![];
//     //     // while let Some(n) = rx.recv().await {
//     //     //     println!("GOT ANOTHER {}", n);
//     //     //     // cpin.read_from(f).await?
//     //     //     cpin.send(n.into_bytes()).await?;
//     //     //     
//     //     // }
//     //     // let mut s: String = "".to_string();
//     //     // for (i, da) in data.iter().enumerate() {
//     //     //     if i > 5 {break};
//     //     //     if let Some(dat) = rx.recv().await {
//     //     //         info!("another")
//     //     //         s = s + &*format!("{}\t{}\t{}\t{:?}\t1.1.1.1/24\t{{}}\n", dat.id, dat.asn, dat.withdrawal as u8, dat.timestamp, );
//     //     //     }
//     //     // }
//     //     // let x: tokio_stream::wrappers::UnboundedReceiverStream = rx;
//     //     // cpin.send(s.into_bytes()).await?;
//     
//     
//     let rb = AsyncHeapRb::<u8>::new(200);
//     let (prod, cons) = rb.split();
//     tokio::join!(
//         async move {
//             let mut n = prod;
//             let s = "teststr";
//             for a in s.bytes() {
//                 println!("pushing {}", a);
//                 n.push(a);
//             }
//         },
//         async move {
//             let mut con = cons;
//             println!("Got {:?}", con.pop().await);
//         }
//     );
//     // let mut conn = pool.acquire().await?;
//     // let mut cpin = conn.copy_in_raw("COPY Announcement FROM STDIN").await?;
//     // 
//     // 
//     // tokio::spawn(async move {
//     //     let mut n = prod;
//     //     let s = format!("{}\t65000\t0\t{:?}\t1.1.1.1/24\t{{}}", Uuid::new_v4(), 1f64);
//     //     for a in s.into_bytes() {
//     //         info!("pushing {}", a);
//     //         n.push(a);
//     //     }
//     // }).await?;
//     // 
//     // let mut con =  cons;
//     // cpin.read_from(con).await?;
//     // info!("sent");
//     // cpin.finish().await?;
//     Ok(())
// }
// 
// 
// 
// 
// // let mut map = StringPatriciaMap::new();
// // map.insert("1100", "ASN6500");
// // map.insert("1110", "ASN6501");
//　22 days to download 3 days of data and 3 days to reasearch methods
// // TODO: Add parralelled insert using pool
// // 20MB network   | 190 MB disk || 15 minutes data | 290 minutes processing time 
// // 175 GB network | 1.7 TB      || 3 months data   | 4.8 years processing time 
// // TODO: Collect timing and size stats
// // TODO: Use docker to automate
// async fn fut () {}

use std::future::{join};
use async_ringbuf::AsyncHeapRb;
fn main() {
    block_on(async_main());
}

async fn async_main() {
    let rb = AsyncHeapRb::<i32>::new(2);
    let (prod, cons) = rb.split();

    join!(
        async move {
            let mut prod = prod;
            for i in 0..2 {
                prod.push(i).await.unwrap();
            }
        },
        async move {
            let mut cons = cons;
            for i in 0..2 {
                assert_eq!(cons.pop().await, Some(i));
            }
            assert_eq!(cons.pop().await, None);
        },
    );
}