// Logs and Errors
use std::error::Error;
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
use super::db_writer::types::Orange;

// db type
use ipnetwork::IpNetwork;

pub fn collect_bgp(start: u64, end: u64) -> BgpkitBroker {
    let broker = bgpkit_broker::BgpkitBroker::new()
        .ts_start(start.to_string().as_str())
        .ts_end(end.to_string().as_str());
    return broker;
}

pub fn parse_bgp(broker: &BgpkitBroker) -> Result<Orange, Box<dyn Error + Send + Sync>>{
    for item in broker {
        log::info!("downloading updates file: {}", &item.url);
        let parser = BgpkitParser::new(item.url.as_str()).unwrap();
        log::info!("parsing updates file");

        // let elems = parser
        //     .into_elem_iter()
        //     .filter_map(|elem| {
        //         if let Some(origins) = &elem.origin_asns {
        //             if origins.contains(&13335.into()) {
        //                 Some(elem)
        //             } else {
        //                 None
        //             }
        //         } else {
        //             None
        //         }
        //     })
        //     .collect::<Vec<BgpElem>>();
        for elem in parser {
            if elem.prefix.prefix.addr() == Ipv4Addr::new(0,0,0,0)
            {print!("a"); continue;}
            println!("\tELEMENT:\n{:?}", elem);
            // dbg!(elem.prefix.prefix.addr());
            // dbg!(elem.prefix.prefix.netmask());
            let network: IpNetwork = IpNetwork::new( elem.prefix.prefix.addr(), elem.prefix.prefix.prefix_len())?;
            return Ok(Orange {
                ASN: elem.peer_asn.asn as i32,
                prefixes: vec![network],
            })
        }
    }
    Err(Box::try_from("HUHHHHHHH").unwrap())
}