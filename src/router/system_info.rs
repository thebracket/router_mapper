use crate::query_engine::{as_string, snmp_query};
use anyhow::Result;

#[derive(Default, Debug)]
pub struct SystemInfo {
    pub platform: String,
    pub contact: String,
    pub hostname: String,
    pub location: String,
}

impl SystemInfo {
    pub(crate) async fn from_snmp(ip: &str, community: &str) -> Result<Self> {
        let info = snmp_query(ip, community, "1.3.6.1.2.1.1").await?;
        let mut result = SystemInfo::default();
        for (oid, val) in info {
            match oid.as_str() {
                "1.3.6.1.2.1.1.1.0" => result.platform = as_string(&val)?,
                "1.3.6.1.2.1.1.4.0" => result.contact = as_string(&val)?,
                "1.3.6.1.2.1.1.5.0" => result.hostname = as_string(&val)?,
                "1.3.6.1.2.1.1.6.0" => result.location = as_string(&val)?,
                _ => {}
            }
        }

        Ok(result)
    }
}
