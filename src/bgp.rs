#![feature(int_roundings)]
// Logs and Errors
#[allow(unused_imports)]
use anyhow::Context;
#[allow(unused_imports)]
use log::{info, warn, error, debug};

// BGP data
use uuid::Uuid;

use crate::db_writer::types::{APSegments, ASPathSeg, DELIMITER};
use bgpkit_broker::BgpkitBroker;
use bgpkit_parser::{
    models::{AsPathSegment, ElemType},
    BgpkitParser,
};

// Data Processing
use crossbeam_channel::{SendError, Sender};
use itertools::Itertools;
use rayon::prelude::*;
const CHNKSZ: usize = 12;
pub const EOF: [u8; 5] = [0, 0, 0, 0, 0];
pub const EOW: [u8; 5] = [0, 0, 0, 0, 1];
pub fn collect_bgp(start: u64, end: u64) -> BgpkitBroker {
    let broker = BgpkitBroker::new()
        .project("riperis")
        .collector_id("rrc25")
        //.data_type("update")
        .ts_start(start.to_string().as_str())
        .ts_end(end.to_string().as_str())
        .page(1)
        .page_size(100);
    return broker;
}

pub fn parse_bgp(broker: BgpkitBroker, sender: Sender<Vec<u8>>) -> Result<(), anyhow::Error> {
    // make copy of sender for each par iter?
    use std::time::Instant;
    let urls = broker.into_iter().map(|x| x.url).collect::<Vec<String>>();
    let chunk_count = urls.iter().count().div_ceil(CHNKSZ);
    let mut index = 0usize;

    urls.chunks(CHNKSZ).for_each(|chunketh| {
        index += 1;
        info!("v-- {index}/{chunk_count} chunk processing --v");
        let now = Instant::now();
        chunketh.par_iter()
            .filter_map(|url| {info!("--- parsing {}", url.as_str()); BgpkitParser::new(url.as_str()).ok()})
            .for_each_with(sender.clone(), |tx, parser| {
               let data = parser.into_elem_iter()
                   .flat_map(|elem| {
                       format!("{ID}{DELIMITER}{ASN}{DELIMITER}{WITHDRAW}{DELIMITER}{TIMESTAMP:?}{DELIMITER}{PREFIX}{DELIMITER}{AS_PATH}\n",
                               ID = Uuid::new_v4(),
                               ASN = elem.peer_asn.asn,
                               WITHDRAW = match elem.elem_type {
                                   ElemType::ANNOUNCE => 0,
                                   ElemType::WITHDRAW => 1,
                               },
                               TIMESTAMP = elem.timestamp,
                               PREFIX = elem.prefix.prefix.addr().to_string() + "/" + &*elem.prefix.prefix.prefix_len().to_string(),
                               AS_PATH = APSegments(
                                   match elem.as_path {
                                       None => vec![],
                                       Some(as_p) => as_p
                                           .segments
                                           .iter()
                                           .map(|as_p_seg| match as_p_seg {
                                               AsPathSegment::AsSequence(x) => ASPathSeg {
                                                   seq: true,
                                                   confed: false,
                                                   as_path: x.par_iter().map(|&y| i64::from(y.asn)).collect(),
                                               },
                                               AsPathSegment::AsSet(x) => ASPathSeg {
                                                   seq: false,
                                                   confed: false,
                                                   as_path: x.par_iter().map(|&y| i64::from(y.asn)).collect(),
                                               },
                                               AsPathSegment::ConfedSequence(x) => ASPathSeg {
                                                   seq: true,
                                                   confed: true,
                                                   as_path: x.par_iter().map(|&y| i64::from(y.asn)).collect(),
                                               },
                                               AsPathSegment::ConfedSet(x) => ASPathSeg {
                                                   seq: false,
                                                   confed: true,
                                                   as_path: x.par_iter().map(|&y| i64::from(y.asn)).collect(),
                                               },
                                           })
                                           .collect::<Vec<ASPathSeg>>(),
                                   }
                               )
                       ).into_bytes()
                   })
                   .collect::<Vec<u8>>();
               match tx.send(data) {
                   Ok(_) => {}
                   Err(e) => { error!("Channel Disconnected, data:\n\t{e}"); }
               }
        });
        sender.send(Vec::from(EOW)).expect("Could not send EOW");
        let elapsed = now.elapsed();
        info!("^-- {index}/{chunk_count} Done in: {:.2?} --^", elapsed);
    });
    sender.send(Vec::from(EOF)).expect("Could not send EOF");
    Ok(())
}
