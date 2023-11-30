use anyhow::Result;
use tracing::{debug, error};
mod system_info;
use system_info::SystemInfo;
mod connection;
use connection::Connection;
mod ip_table;
use crate::router::ip_routes::IpRoutes;
use ip_table::IpTable;
mod ip_routes;

pub async fn router_builder(ip_address: String, community: String) -> Result<Router> {
    debug!("Querying {ip_address} for SNMP information");
    let connection = Connection {
        snmp_address: ip_address.to_string(),
        snmp_community: community.to_string(),
    };
    let (system_info, ip_table, ip_routes) = tokio::join!(
        SystemInfo::from_snmp(&ip_address, &community),
        IpTable::from_snmp(&ip_address, &community),
        IpRoutes::from_snmp(&ip_address, &community)
    );
    debug!("Finished querying {ip_address} for SNMP information");

    if system_info.is_err() {
        error!("Failed to query {ip_address} for system information");
    }
    if ip_table.is_err() {
        error!("Failed to query {ip_address} for IP table information");
    }
    if ip_routes.is_err() {
        error!("Failed to query {ip_address} for IP routes information");
    }

    Ok(Router {
        connection,
        system_info: system_info?,
        ip_table: ip_table?,
        ip_routes: ip_routes?,
    })
}

#[derive(Debug)]
pub struct Router {
    pub connection: Connection,
    pub system_info: SystemInfo,
    pub ip_table: IpTable,
    pub ip_routes: IpRoutes,
}
