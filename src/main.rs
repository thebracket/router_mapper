#![recursion_limit = "256"]
#![allow(dead_code)]

mod config;
mod csnmp;
mod router;
use anyhow::Result;
use std::time::Instant;
use tracing::info;
mod query_engine;
mod router_list;

#[derive(Debug)]
struct RouteMap {
    name: String,
    parent: Option<usize>,
}

fn print_tree(tree: &Vec<RouteMap>, idx: usize, indent: usize) {
    for (index, map) in tree.iter().enumerate() {
        if map.parent == Some(idx) {
            println!("{}-> {}", "-".repeat(indent), map.name);
            print_tree(tree, index, indent + 3);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup tracing for nicer output
    tracing_subscriber::fmt::init();

    info!("Router Mapper 0.0.1 is Starting");

    let now = Instant::now();
    let targets = router_list::RouterList::from_csv("router_list.csv")?;
    let routers = targets.fetch_all().await?;
    let elapsed = now.elapsed();
    info!(
        "Queried {} routers in {:.2} seconds. Retrieved {} routers.",
        targets.targets.len(),
        elapsed.as_secs_f64(),
        routers.len()
    );

    // Build the initial route map
    let mut route_map: Vec<RouteMap> = routers
        .iter()
        .map(|router| RouteMap {
            name: router.system_info.hostname.clone(),
            parent: None,
        })
        .collect();

    // Find by default gateway search
    route_map.iter_mut().enumerate().for_each(|(idx, map)| {
        let me = &routers[idx];
        if !me.ip_routes.routes.is_empty() {
            let my_default_route = &me.ip_routes.routes[0].next_hop;
            let likely_parent = routers.iter().position(|r| {
                r.ip_table
                    .ips
                    .iter()
                    .any(|ip| ip.address == *my_default_route)
            });
            map.parent = likely_parent;
        }
    });

    // Display as a nice tree
    for (index, map) in route_map.iter().enumerate() {
        if map.parent.is_none() {
            println!("{}", map.name);
            print_tree(&route_map, index, 3);
        }
    }

    Ok(())
}
