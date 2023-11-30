use anyhow::{bail, Result};
use crate::csnmp::{ObjectIdentifier, ObjectValue, Snmp2cClient};
use ipnetwork::ip_mask_to_prefix;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_secs(5);
// Setting this to large numbers breaks on Mikrotik
const MAX_REPEAT: u32 = 20;

pub async fn snmp_query(
    target_ip: &str,
    community: &str,
    oid: &str,
) -> Result<Vec<(String, ObjectValue)>> {
    let community = Vec::from(community);
    let top_oid: ObjectIdentifier = oid.parse()?;
    let sock_addr = SocketAddr::from((target_ip.parse::<IpAddr>()?, 161));

    let client = Snmp2cClient::new(
        sock_addr,
        community,
        Some("0.0.0.0:0".parse()?),
        Some(TIMEOUT),
    )
    .await?;

    let results = client.walk_bulk(top_oid, MAX_REPEAT).await?;

    let result = results
        .into_iter()
        .map(|(oid, value)| (oid.to_string(), value))
        .collect();

    Ok(result)
}

pub fn as_string(value: &ObjectValue) -> Result<String> {
    match value {
        ObjectValue::String(s) => Ok(String::from_utf8(s.clone())?),
        ObjectValue::Integer(i) => Ok(i.to_string()),
        ObjectValue::IpAddress(ip) => Ok(ip.to_string()),
        ObjectValue::Counter32(i) => Ok(i.to_string()),
        ObjectValue::Counter64(i) => Ok(i.to_string()),
        ObjectValue::TimeTicks(i) => Ok(i.to_string()),
        ObjectValue::Opaque(s) => Ok(String::from_utf8(s.clone())?),
        _ => bail!("Unknown value type"),
    }
}

pub fn as_ip(value: &ObjectValue) -> Result<IpAddr> {
    match value {
        ObjectValue::IpAddress(ip) => Ok(IpAddr::from(*ip)),
        _ => bail!("Unknown value type"),
    }
}

pub fn as_int(value: &ObjectValue) -> Result<i32> {
    match value {
        ObjectValue::Integer(i) => Ok(*i),
        _ => bail!("Unknown value type"),
    }
}

pub fn as_cidr(value: &ObjectValue) -> Result<u8> {
    let ip = as_ip(value)?;
    Ok(ip_mask_to_prefix(ip)?)
}
