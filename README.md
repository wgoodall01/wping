# wping

A bare-bones reimplementation of `ping` in Rust.

# Features

- Ping an Ipv4 address
- Ping a hostname
- Ping with a timeout
- Ping using packets with a specified TTL
- Ping with a specific packet count

Yeah, it's basically just ping.

# Examples

> Note: you need to run `wping` as root, or with `cap_net_raw`, because it opens a raw socket to send ICMP packets directly.

```bash
# # Ping an IPv4 address
# wping 1.1.1.1
PING one.one.one.one (1.1.1.1) 56(84) bytes of data.
64 bytes from one.one.one.one (1.1.1.1): icmp_seq=0 ttl=128 time=41
64 bytes from one.one.one.one (1.1.1.1): icmp_seq=1 ttl=128 time=45
64 bytes from one.one.one.one (1.1.1.1): icmp_seq=2 ttl=128 time=49
64 bytes from one.one.one.one (1.1.1.1): icmp_seq=3 ttl=128 time=46
64 bytes from one.one.one.one (1.1.1.1): icmp_seq=4 ttl=128 time=43
--- one.one.one.one ping statistics ---
5 packets transmitted, 5 received, 0 errors, 0.00% packet loss
rtt min/avg/max/stdev = 41/44.8/49/2.71
```

```bash
# # Ping a domain with an IPv4 time-to-live of 9 hops
# wping -t 9 google.com
PING google.com (172.217.12.174) 56(84) bytes of data.
64 bytes from lga25s62-in-f14.1e100.net (172.217.12.174): icmp_seq=0 ttl=9 time=16
64 bytes from lga25s62-in-f14.1e100.net (172.217.12.174): icmp_seq=1 ttl=9 time=20
64 bytes from lga25s62-in-f14.1e100.net (172.217.12.174): icmp_seq=2 ttl=9 time=22
64 bytes from lga25s62-in-f14.1e100.net (172.217.12.174): icmp_seq=3 ttl=9 time=21
64 bytes from lga25s62-in-f14.1e100.net (172.217.12.174): icmp_seq=4 ttl=9 time=20
--- google.com ping statistics ---
5 packets transmitted, 5 received, 0 errors, 0.00% packet loss
rtt min/avg/max/stdev = 16/19.8/22/2.04
```

# Usage

```
wping 1.0
William Goodall <wgoodall01@gmail.com>

USAGE:
    wping [OPTIONS] <host>

ARGS:
    <host>    The host name or IPv4 address to ping

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --count <count>        Count of packets to send [default: 5]
    -W, --timeout <timeout>    Timeout while waiting for ping responses (in ms) [default: 2000]
    -t, --ttl <ttl>            IP time-to-live for each packet sent [default: 128]
```

# Implementation

Under the hood, `wping` uses the `pnet` package to do low-level network IO, and the `resolve` package to resolve DNS names.
