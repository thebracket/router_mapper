use crate::{
    config::CONFIG,
    csnmp::ObjectValue,
    query_engine::{as_int, as_ip, snmp_query},
};
use anyhow::{bail, Result};
use std::net::IpAddr;
use tracing::error;

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

        if dgw_dest.is_err() || dgw_iface.is_err() || dgw_next_hop.is_err() {
            return Self::from_snmp_modern(ip_address, community).await;
        }

        let (dgw_dest, dgw_iface, dgw_next_hop) = (dgw_dest?, dgw_iface?, dgw_next_hop?);
        if dgw_dest.is_empty() || dgw_iface.is_empty() || dgw_next_hop.is_empty() {
            return Self::from_snmp_modern(ip_address, community).await;
        }

        Ok(Self {
            routes: vec![IpRoute {
                destination: as_ip(&dgw_dest[0].1)?,
                interface_index: as_int(&dgw_iface[0].1)?,
                next_hop: as_ip(&dgw_next_hop[0].1)?,
            }],
        })
    }

    async fn from_snmp_modern(ip_address: &str, community: &str) -> Result<Self> {
        let (dest, next_hop) = tokio::join!(
            snmp_query(ip_address, community, "1.3.6.1.2.1.4.24.4.1.1.0.0.0.0"),
            snmp_query(ip_address, community, "1.3.6.1.2.1.4.24.4.1.4.0.0.0.0")
        );

        if dest.is_err() || next_hop.is_err() {
            error!(
                "Error obtaining gateway information: {:?}, {ip_address}",
                dest.err()
            );
            return Ok(Self { routes: vec![] });
        }

        let (dest, next_hop) = (dest?, next_hop?);

        if dest.is_empty() || next_hop.is_empty() {
            error!("Error obtaining gateway information - no routes found, {ip_address}");
            return Ok(Self { routes: vec![] });
        }

        // Secondary hop support - for iBGP and route-reflection setups
        if CONFIG.enable_next_hop_lookup {
            if let Ok(secondary_hop) =
                Self::lookup_secondary(ip_address, community, &next_hop[0].1).await
            {
                tracing::info!("Secondary hop found: {secondary_hop:?}");
                return Ok(Self {
                    routes: vec![IpRoute {
                        destination: as_ip(&dest[0].1)?,
                        interface_index: -1,
                        next_hop: secondary_hop,
                    }],
                });
            }
        }

        tracing::info!("Returning routes via inetCidrRouteTable");
        Ok(Self {
            routes: vec![IpRoute {
                destination: as_ip(&dest[0].1)?,
                interface_index: -1,
                next_hop: as_ip(&next_hop[0].1)?,
            }],
        })
    }

    async fn lookup_secondary(
        ip_address: &str,
        community: &str,
        next_hop: &ObjectValue,
    ) -> Result<IpAddr> {
        let secondary_hop = format!(
            "1.3.6.1.2.1.4.24.4.1.4.0.0.0.0.0.0.0.0.0.{}",
            as_ip(&next_hop)?
        );
        let result = snmp_query(ip_address, community, &secondary_hop).await?;
        if result.is_empty() {
            bail!("No secondary hop found");
        }
        as_ip(&result[0].1)
    }
}
