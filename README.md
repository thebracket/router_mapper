# Router Mapper

> This is experimental. Please don't expect much at this point.

This project queries SNMP data from routers. It obtains:

* The router's system description data
* All IP addresses associated with the router
* The router's default gateway

This information is then combined into a tree (which can be multi-headed), and a hierarchy of routers is determined based on gateway-IP mappings.

## Usage

In the base directory, copy `router_list.csv.example` to `router_list.csv` and edit the file to include routers you want to query. For example:

```csv
# List of IP addresses and SNMP communities to query.
# IPv6 is supported for reading - but really not recommended at this point.
# Rename this file from .example to .csv, and put actual data into it.
IP, Community
192.168.1.1, public
```

> It's a great idea to rename your `public` community to something else.

Once that's in place, you can run the tool with `cargo run` (or `cargo run --release` to go faster). You will see output similar to the following:

```
2023-11-30T17:05:49.443446Z  INFO router_mapper: Router Mapper 0.0.1 is Starting
2023-11-30T17:05:49.761790Z  INFO router_mapper: Queried 9 routers in 0.32 seconds
Paquin-Edge
----> A router
----> A router
-------> A router
-------> A router
----------> A router
-------------> A router
-------------> A router
-------> A router
```

Eventually, this is intended to be a useful addition to the LibreQoS network mapping system. For now, it's a toy. Enjoy.

## Notes

Currently using a fork of `csnmp`, because the original crashes on duplicate entries in a table. Routing tables are *allowed* to have multiple entries.