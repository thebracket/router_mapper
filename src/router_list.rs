use crate::router::{router_builder, Router};
use anyhow::{bail, Result};
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tokio::task::JoinSet;
use tracing::{error, info};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouterTarget {
    pub ip_address: String,
    pub community: String,
}

#[derive(Debug)]
pub struct RouterList {
    pub targets: Vec<RouterTarget>,
}

impl RouterList {
    pub fn from_csv(filename: &str) -> Result<Self> {
        // Check that the file exists
        let path = Path::new(filename);
        if !path.exists() {
            bail!("File {} does not exist", filename);
        }

        let mut targets = Vec::new();

        // Create a CSV reader
        let reader = ReaderBuilder::new()
            .comment(Some(b'#'))
            .trim(csv::Trim::All)
            .from_path(path)?;

        for line in reader.into_records() {
            let line = line?;
            let target: RouterTarget = line.deserialize(None)?;
            targets.push(target);
        }

        Ok(Self { targets })
    }

    async fn router_builder_with_retries(
        ip: String,
        community: String,
        retries: u32,
    ) -> Result<Router> {
        for attempt in 0..retries {
            if attempt > 0 {
                info!("Retrying {ip} (attempt {} of {retries})...", attempt + 1);
            }
            match router_builder(ip.clone(), community.clone()).await {
                Ok(router) => return Ok(router),
                Err(e) => error!("Error fetching SNMP data from {ip}: {e}"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        bail!("Unable to fetch SNMP data from {ip}. I tried {retries} times");
    }

    pub async fn fetch_all(&self) -> Result<Vec<Router>> {
        let mut set = JoinSet::new();
        for target in self.targets.iter() {
            let ip = target.ip_address.clone();
            let community = target.community.clone();
            let router = Self::router_builder_with_retries(ip, community, 3);
            set.spawn(router);
        }

        let mut results = Vec::new();
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(router)) => results.push(router),
                Ok(Err(e)) => error!("Router Parse Error: {:?}", e),
                Err(e) => error!("JoinSet Error: {:?}", e),
            }
        }

        Ok(results)
    }
}
