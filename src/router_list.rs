use crate::router::{router_builder, Router};
use anyhow::{bail, Result};
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::task::JoinSet;
use tracing::error;

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

    pub async fn fetch_all(&self) -> Result<Vec<Router>> {
        let mut set = JoinSet::new();
        for target in self.targets.iter() {
            let ip = target.ip_address.clone();
            let community = target.community.clone();
            let router = router_builder(ip, community);
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
