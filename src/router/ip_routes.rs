use crate::query_engine::{as_int, as_ip, snmp_query};
use anyhow::{bail, Result};
use ipnetwork::ip_mask_to_prefix;
use std::{collections::HashMap, net::IpAddr};

#[derive(Debug)]
pub struct IpRoutes {
    pub routes: Vec<CidrEntry>,
}

const CIDR_TABLE: &str = "1.3.6.1.2.1.4.24.4.";
//const CIDR_TABLE: &str = "1.3.6.1.2.1.4.24.7";

const INET_ROUTE_TABLE: &str = "1.3.6.1.2.1.4.21.1.";

#[derive(Debug)]
pub struct CidrEntry {
    // .1 = destination
    pub destination: IpAddr,

    // .2 = netmask
    pub netmask: u8,

    // .3 = tos

    // .4 = next hop
    pub next_hop: IpAddr,

    // .5 = ifIndex
    pub if_index: i32,
    // .6 = Route type
    // .7 = Route Proto
    // .8 = Route Age
    // .9 = Route Info (object id)
    // .10 = Next Hop AS
    // .11 = Metric 1
    // .12, .13, .14. 15 = Metrics 2,3,4,5
    // .16 = Route Status
}

impl IpRoutes {
    fn ip_from_oid(oid: &str) -> Result<IpAddr> {
        let oid_split = oid.split('.').collect::<Vec<&str>>();
        let idx = 11;
        let route_ip = format!(
            "{}.{}.{}.{}",
            oid_split[idx],
            oid_split[idx + 1],
            oid_split[idx + 2],
            oid_split[idx + 3]
        );
        Ok(route_ip.parse::<IpAddr>()?)
    }

    pub(crate) async fn from_snmp(ip_address: &str, community: &str) -> Result<Self> {
        let unknown_ip: IpAddr = "255.255.255.255".parse().unwrap();
        let full_table = snmp_query(ip_address, community, CIDR_TABLE).await;
        if full_table.is_err() {
            tracing::info!("Falling back to older SNMP table for {ip_address}");
            return Self::from_old_table(ip_address, community).await;
        }
        let full_table = full_table.unwrap();

        let mut routes = HashMap::new();

        for (oid, value) in full_table {
            if oid.starts_with("1.3.6.1.2.1.4.24.4.1.1.") {
                // New record. This is the route destination
                let destination = as_ip(&value)?;
                let new_route = CidrEntry {
                    destination,
                    netmask: 255,
                    next_hop: unknown_ip,
                    if_index: -1,
                };
                routes.insert(destination, new_route);
                //tracing::info!("Inserting new route: {destination}");
            } else if oid.starts_with("1.3.6.1.2.1.4.24.4.1.2.") {
                let ip = Self::ip_from_oid(&oid)?;
                if let Some(route) = routes.get_mut(&ip) {
                    let mask = as_ip(&value)?;
                    let cidr = ip_mask_to_prefix(mask)?;
                    route.netmask = cidr;
                } else {
                    tracing::error!("No route found for {ip}");
                }
            } else if oid.starts_with("1.3.6.1.2.1.4.24.4.1.4.") {
                let ip = Self::ip_from_oid(&oid)?;
                if let Some(route) = routes.get_mut(&ip) {
                    route.next_hop = as_ip(&value)?;
                } else {
                    tracing::error!("No route found for {ip}");
                }
            } else if oid.starts_with("1.3.6.1.2.1.4.24.4.1.5.") {
                let ip = Self::ip_from_oid(&oid)?;
                if let Some(route) = routes.get_mut(&ip) {
                    route.if_index = as_int(&value)?;
                } else {
                    tracing::error!("No route found for {ip}");
                }
            }
        }

        if routes.is_empty() {
            return Self::from_old_table(ip_address, community).await;
        }

        Ok(Self {
            routes: routes.into_iter().map(|(_, v)| v).collect(),
        })
    }

    pub(crate) async fn from_old_table(ip_address: &str, community: &str) -> Result<Self> {
        let full_table = snmp_query(ip_address, community, INET_ROUTE_TABLE).await;
        if full_table.is_err() {
            tracing::info!("Unable to retreieve old-style routing table from {ip_address}");
            bail!("Unable to retreieve old-style routing table from {ip_address}, {full_table:?}");
        }
        let full_table = full_table.unwrap();

        let unknown_ip: IpAddr = "255.255.255.255".parse().unwrap();
        let mut routes = HashMap::new();
        for (oid, val) in full_table {
            if oid.starts_with("1.3.6.1.2.1.4.21.1.1.") {
                // We have a new route
                let route_ip = as_ip(&val)?;
                let new_route = CidrEntry {
                    destination: route_ip,
                    netmask: 255,
                    next_hop: unknown_ip,
                    if_index: -1,
                };
                routes.insert(route_ip, new_route);
            } else if oid.starts_with("1.3.6.1.2.1.4.21.1.2.") {
                let ip_from_oid = oid[21..].parse::<IpAddr>()?;
                if let Some(route) = routes.get_mut(&ip_from_oid) {
                    route.if_index = as_int(&val)?;
                }
            } else if oid.starts_with("1.3.6.1.2.1.4.21.1.7.") {
                let ip_from_oid = oid[21..].parse::<IpAddr>()?;
                if let Some(route) = routes.get_mut(&ip_from_oid) {
                    route.next_hop = as_ip(&val)?;
                }
            } else if oid.starts_with("1.3.6.1.2.1.4.21.1.11.") {
                let ip_from_oid = oid[22..].parse::<IpAddr>()?;
                if let Some(route) = routes.get_mut(&ip_from_oid) {
                    let mask = as_ip(&val)?;
                    let cidr = ip_mask_to_prefix(mask)?;
                    route.netmask = cidr;
                }
            }
        }

        Ok(Self {
            routes: routes.into_iter().map(|(_, v)| v).collect(),
        })
    }
}
