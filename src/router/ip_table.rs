use crate::query_engine::{as_cidr, as_int, as_ip, snmp_query};
use anyhow::Result;
use std::{collections::HashMap, net::IpAddr};

#[derive(Debug)]
pub struct IpTable {
    pub ips: Vec<IpAddress>,
}

#[derive(Debug)]
pub struct IpAddress {
    pub address: IpAddr,
    pub interface_index: i32,
    pub cidr_mask: u8,
}

impl IpTable {
    pub(crate) async fn from_snmp(ip_address: &str, community: &str) -> Result<IpTable> {
        let ip_table = snmp_query(ip_address, community, "1.3.6.1.2.1.4.20").await?;

        let mut ips = HashMap::new();

        for (oid, val) in ip_table {
            if oid.starts_with("1.3.6.1.2.1.4.20.1.1.") {
                let ip = as_ip(&val)?;
                ips.insert(
                    ip,
                    IpAddress {
                        address: ip,
                        interface_index: -1,
                        cidr_mask: 255,
                    },
                );
            } else if oid.starts_with("1.3.6.1.2.1.4.20.1.2.") {
                // Interface index
                let ip: IpAddr = oid[21..].parse()?;
                if let Some(iface) = ips.get_mut(&ip) {
                    iface.interface_index = as_int(&val)?;
                }
            } else if oid.starts_with("1.3.6.1.2.1.4.20.1.3.") {
                // Netmask
                let ip: IpAddr = oid[21..].parse()?;
                if let Some(iface) = ips.get_mut(&ip) {
                    iface.cidr_mask = as_cidr(&val).unwrap_or_default();
                }
            }
        }

        Ok(IpTable {
            ips: ips.into_iter().map(|(_, v)| v).collect(),
        })
    }
}
