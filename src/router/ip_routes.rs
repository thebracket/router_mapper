use crate::query_engine::{as_int, as_ip, snmp_query};
use anyhow::Result;
use std::net::IpAddr;

#[derive(Debug)]
pub struct IpRoutes {
    pub routes: Vec<IpRoute>,
}

#[derive(Debug)]
pub struct IpRoute {
    pub destination: IpAddr,
    pub interface_index: i32,
    pub next_hop: IpAddr,
}

impl IpRoutes {
    pub(crate) async fn from_snmp(ip_address: &str, community: &str) -> Result<Self> {
        // Note that dgw is short for "default gateway"
        let (dgw_dest, dgw_iface, dgw_next_hop) = tokio::join!(
            snmp_query(ip_address, community, "1.3.6.1.2.1.4.21.1.1.0.0.0.0"),
            snmp_query(ip_address, community, "1.3.6.1.2.1.4.21.1.2.0.0.0.0"),
            snmp_query(ip_address, community, "1.3.6.1.2.1.4.21.1.7.0.0.0.0"),
        );

        Ok(Self {
            routes: vec![IpRoute {
                destination: as_ip(&dgw_dest?[0].1)?,
                interface_index: as_int(&dgw_iface?[0].1)?,
                next_hop: as_ip(&dgw_next_hop?[0].1)?,
            }],
        })
    }
}
