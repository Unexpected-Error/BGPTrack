// Logs and Errors
use std::error::Error;
use std::io::Read;
#[allow(unused_imports)]
use std::net::{IpAddr, Ipv4Addr};
#[allow(unused_imports)]
use anyhow::Context;
#[allow(unused_imports)]
use log::{info, warn};

// BGP data
use bgpkit_broker;
#[allow(unused_imports)]
use bgpkit_parser::{BgpkitParser, BgpElem};
use bgpkit_broker::BgpkitBroker;
use bgpkit_parser::models::{AsPath, AsPathSegment, ElemType, NetworkPrefix};
use ipnet::IpNet;
use crate::db_writer::types::{Orange, R_ASN, RawAnnouncement};

// db type
use ipnetwork::IpNetwork;
use time::{OffsetDateTime, PrimitiveDateTime};

// Data Processing
use rayon::prelude::*;

pub fn collect_bgp(start: u64, end: u64) -> BgpkitBroker {
    let broker = bgpkit_broker::BgpkitBroker::new()
        .project("riperis")
        .collector_id("rrc25")
        //.data_type("update")
        .ts_start(start.to_string().as_str())
        .ts_end(end.to_string().as_str());
    return broker;
}

pub fn parse_bgp(broker: BgpkitBroker) -> Result<Vec<(RawAnnouncement, R_ASN)>, Box<dyn Error + Send + Sync>> {
    
    Ok(broker
        .into_iter()
        //.par_iter()
        .map(
            |item| {
                info!("Collector: {}\nType: {}\nUrl: {}", &item.collector_id, &item.data_type, &item.url);
                BgpkitParser::new(item.url.as_str()).context("Init BGP parse").unwrap()
            })
        .collect::<Vec<BgpkitParser<Box<dyn Read + Send>>>>()
        .into_iter()
        .flat_map(|parser| { //: BgpkitParser<c> ::<BgpkitParser<Box<dyn Read + Send>>>
            parser
                .into_elem_iter()
                //.par_iter()
                .map(|elem| {
                    (RawAnnouncement {
                        time: elem.timestamp,//.trunc() as i64,//PrimitiveDateTime::new(odt.date(), odt.time()),
                        announcing: match elem.elem_type {
                            ElemType::ANNOUNCE => { true }
                            ElemType::WITHDRAW => { false }
                        },
                        prefix: match elem.prefix.prefix {
                            IpNet::V4(x) => {
                                IpNetwork::new(IpAddr::from(x.addr()), x.prefix_len())
                            }
                            IpNet::V6(x) => {
                                IpNetwork::new(IpAddr::from(x.addr()), x.prefix_len())
                            }
                        }.context("Matching IPs").unwrap(),
                        as_path: match elem.as_path {
                            None => { None }
                            Some(as_p) => {
                                if as_p.segments.len() != 1 {
                                    panic!("WHAT THE FUCK IS THIS AS_PATH HOLY SHIT")
                                } else {
                                    match as_p.segments.get(0) {

                                        // AsPathSegment::AsSequence(v) | AsPathSegment::ConfedSequence(v) =>
                                        //     v.,
                                        // AsPathSegment::AsSet(v) | AsPathSegment::ConfedSet(v) => {
                                        //     format!("{{{}}}", v.iter().join(","))
                                        // }

                                        Some(AsPathSegment::AsSequence(seq)) => { seq.into_iter().map(|b| { b.asn }) }
                                        n => panic!("WHAT THE FUCK IS THIS AS_PATH HOLY SHIT! {:?}", n)
                                    }
                                }
                            }
                        },
                        as_path_is_seq: true,
                    }, elem.peer_asn.asn)
                })
        }).collect::<Vec<(RawAnnouncement,R_ASN)>>())


    // for elem in parser {
    //     // Deal with non [`AsSequence`] AsPaths
    //     // Deal with 32 vs 64 ASNs
    //     // Deal with multiple origin ASNs ( Maybe get from AS_PATH?)
    //     
    //     
    //     // if elem.prefix.prefix.addr() == mp 
    //     // {print!("a"); continue;}
    //     //println!("\tELEMENT:\n{:?}", elem);
    //     // dbg!(elem.prefix.prefix.addr());
    //     // dbg!(elem.prefix.prefix.netmask());
    //     
    //     
    //     
    //     match elem.elem_type {
    //         ElemType::ANNOUNCE => {
    //             
    //         }
    //         ElemType::WITHDRAW => {
    //             
    //         }
    //     }
    //     // let network: IpNetwork = IpNetwork::new( elem.prefix.prefix.addr(), elem.prefix.prefix.prefix_len())?;
    //     // return Ok(Orange {
    //     //     ASN: elem.peer_asn.asn as i32,
    //     //     prefixes: vec![network],
    //     // })
    // 
}