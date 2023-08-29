// Logs and Errors
#[allow(unused_imports)]
use anyhow::Context;
#[allow(unused_imports)]
use log::{info, warn};
use std::error::Error;
use std::io::Read;
#[allow(unused_imports)]
use std::net::{IpAddr, Ipv4Addr};
use async_ringbuf::AsyncProducer;

// BGP data
use crate::db_writer::types::{ASPathSeg, Announcement};
use bgpkit_broker;
use bgpkit_broker::BgpkitBroker;
use bgpkit_parser::models::{AsPath, AsPathSegment, ElemType, NetworkPrefix};
#[allow(unused_imports)]
use bgpkit_parser::{BgpElem, BgpkitParser};
use crossbeam_channel::Sender;
use ipnet::IpNet;

// db type
use ipnetwork::IpNetwork;
use time::{OffsetDateTime, PrimitiveDateTime};

// Data Processing
use rayon::prelude::*;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

pub fn collect_bgp(start: u64, end: u64) -> BgpkitBroker {
    let broker = bgpkit_broker::BgpkitBroker::new()
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
    broker
        .into_iter()
        .map(|x| x.url).collect::<Vec<String>>()
        .par_iter()
        .for_each(|url| {
            let parser = BgpkitParser::new(url.as_str()).unwrap();
            info!("parsing {} ...", url.as_str());
            let data = parser.into_elem_iter()
                .flat_map(|elem| { 
                    let ann = Announcement {
                        id: Uuid::new_v4(),
                        asn: elem.peer_asn.asn as i64,
                        withdrawal: match elem.elem_type {
                            ElemType::ANNOUNCE => false,
                            ElemType::WITHDRAW => true,
                        },
                        timestamp: elem.timestamp,
                        prefix: match elem.prefix.prefix {
                            IpNet::V4(x) => IpNetwork::new(IpAddr::from(x.addr()), x.prefix_len()),
                            IpNet::V6(x) => IpNetwork::new(IpAddr::from(x.addr()), x.prefix_len()),
                        }
                            .context("Matching IPs")
                            .unwrap(),
                        as_path_segments: match elem.as_path {
                            None => vec![],
                            Some(as_p) => as_p
                                .segments
                                .par_iter()
                                .map(|as_p_seg| match as_p_seg {
                                    AsPathSegment::AsSequence(x) => ASPathSeg {
                                        seq: true,
                                        confed: false,
                                        as_path: x.par_iter().map(|&y| y.asn as i64).collect(),
                                    },
                                    AsPathSegment::AsSet(x) => ASPathSeg {
                                        seq: false,
                                        confed: false,
                                        as_path: x.par_iter().map(|&y| y.asn as i64).collect(),
                                    },
                                    AsPathSegment::ConfedSequence(x) => ASPathSeg {
                                        seq: true,
                                        confed: true,
                                        as_path: x.par_iter().map(|&y| y.asn as i64).collect(),
                                    },
                                    AsPathSegment::ConfedSet(x) => ASPathSeg {
                                        seq: false,
                                        confed: true,
                                        as_path: x.par_iter().map(|&y| y.asn as i64).collect(),
                                    },
                                })
                                .collect::<Vec<ASPathSeg>>(),
                        },
                    };
                    format!("{}\t{}\t{:?}\t{:?}\t1.1.1.1/24\t{{}}\n", Uuid::new_v4(),ann.asn,ann.withdrawal as u8, ann.timestamp).into_bytes()
                })
                .collect::<Vec<u8>>();
            sender.send(data).expect("TODO: panimc message");
        });
    Ok(())
}
