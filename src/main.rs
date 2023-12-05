#![recursion_limit = "256"]
#![allow(dead_code)]

mod config;
mod csnmp;
mod router;
use anyhow::Result;
use std::{net::IpAddr, time::Instant};
use tracing::info;
mod query_engine;
mod router_list;

#[derive(Debug)]
struct RouteMap {
    name: String,
    parent: Option<usize>,
}

fn print_tree(tree: &Vec<RouteMap>, idx: usize, indent: usize, printed: &mut Vec<bool>) {
    for (index, map) in tree.iter().enumerate() {
        if map.parent == Some(idx) {
            printed[index] = true;
            println!("{}-> {}", "-".repeat(indent), map.name);
            print_tree(tree, index, indent + 3, printed);
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
    let default_ipv4: IpAddr = "0.0.0.0".parse().unwrap();
    route_map.iter_mut().enumerate().for_each(|(idx, map)| {
        let me = &routers[idx];
        if !me.ip_routes.routes.is_empty() {
            let my_default_route = &me
                .ip_routes
                .routes
                .iter()
                .find(|r| r.destination == default_ipv4)
                .unwrap()
                .next_hop;
            let likely_parent = routers.iter().position(|r| {
                r.ip_table
                    .ips
                    .iter()
                    .any(|ip| ip.address == *my_default_route)
            });
            map.parent = likely_parent;

            if !me
                .ip_table
                .ips
                .iter()
                .any(|ip| ip.ip_network().contains(*my_default_route))
            {
                tracing::info!(
                    "{} is a NOT parent of {}",
                    map.name,
                    me.system_info.hostname
                );
                tracing::info!(
                    "{} has a default route of {}",
                    me.system_info.hostname,
                    my_default_route
                );
                println!("{me:?}");
                tracing::info!("Let's go looking for a parent...");
                let likely_parent_route_idx = me
                    .ip_routes
                    .routes
                    .iter()
                    .position(|r| r.destination == *my_default_route);
                if let Some(likely_parent_route_idx) = likely_parent_route_idx {
                    if let Some(likely_parent) = routers
                        .iter()
                        .enumerate()
                        .filter(|(new_idx, _)| *new_idx != idx)
                        .position(|(_, r)| {
                            r.ip_table.ips.iter().any(|ip| {
                                ip.address == me.ip_routes.routes[likely_parent_route_idx].next_hop
                            })
                        })
                    {
                        tracing::info!(
                            "Found a likely parent: {}",
                            routers[likely_parent].system_info.hostname
                        );
                        map.parent = Some(likely_parent);
                    }
                }
            }
        } else {
            tracing::info!("{} has no routes", map.name);
        }
    });

    // Display as a nice tree
    let mut printed = vec![false; route_map.len()];

    for (index, map) in route_map.iter().enumerate() {
        if map.parent.is_none() && !printed[index] {
            printed[index] = true;
            println!("{}", map.name);
            print_tree(&route_map, index, 3, &mut printed);
        }
    }

    while printed.iter().any(|p| !p) {
        for (index, map) in route_map.iter().enumerate() {
            if !printed[index] {
                printed[index] = true;
                println!("{}", map.name);
                print_tree(&route_map, index, 3, &mut printed);
            }
        }
    }

    Ok(())
}
