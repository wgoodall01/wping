use clap::Clap;
use resolve::resolver::{resolve_addr, resolve_host};
use snafu::{OptionExt, ResultExt, Snafu};
use std::net::{IpAddr, Ipv4Addr};
use std::thread::sleep;
use std::time::{Duration, Instant};

mod ping;
use ping::*;

#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "William Goodall <wgoodall01@gmail.com>")]
struct Args {
    /// IP time-to-live for each packet sent.
    #[clap(long = "ttl", short = "t", default_value = "128")]
    ttl: u8,

    /// Timeout while waiting for ping responses (in ms)
    #[clap(long = "timeout", short = "W", default_value = "2000")]
    timeout: u64,

    /// Count of packets to send
    #[clap(long = "count", short = "c", default_value = "5")]
    count: u16,

    /// The host name or IPv4 address to ping.
    host: String,
}

#[derive(Snafu, Debug)]
enum CliError {
    #[snafu(display("could not resolve: {}", source))]
    Resolution { source: std::io::Error },

    #[snafu(display("could not resolve Ipv4 address for host"))]
    NoIpv4Addr,

    #[snafu(display("could not open ICMP transport: {}", source))]
    IcmpOpen { source: PingError },

    #[snafu(display("could not send echo request: {}", source))]
    IcmpSend { source: PingError },

    #[snafu(display("could not recieve icmp response: {}", source))]
    IcmpRecv { source: PingError },
}

fn main() {
    let args = Args::parse();
    match run_ping(args) {
        Ok(_) => std::process::exit(0),
        Err(err) => {
            println!("{}", err);
            std::process::exit(1);
        }
    }
}

fn run_ping(args: Args) -> Result<(), CliError> {
    // Try to parse host as ipv4
    let parsed_ip: Result<Ipv4Addr, _> = args.host.parse();
    let (host_ip, host_name): (Ipv4Addr, Option<String>) = match parsed_ip {
        // The host is an ip address, so reverse-resolve its domain
        Ok(addr) => (addr, resolve_addr(&IpAddr::V4(addr)).ok()),

        // The host is a domain, so resolve its IP
        Err(_) => {
            // TODO: gracefully handle failures
            let addresses = resolve_host(&args.host).context(Resolution)?;

            // Take the first address.
            let addr: Ipv4Addr = addresses
                .filter_map(|ad| match ad {
                    IpAddr::V4(x) => Some(x), // only take ipv4 addresses
                    _ => None,
                })
                .next()
                .context(NoIpv4Addr)?;

            (addr, Some(args.host))
        }
    };

    let host_description = host_name.unwrap_or(format!("{}", host_ip));

    println!(
        "PING {desc} ({ip}) 56(84) bytes of data.",
        desc = host_description,
        ip = host_ip
    );

    // Open the pinger
    let mut pinger = Pinger::open(args.ttl).context(IcmpOpen)?;

    let mut rtt_list: Vec<u128> = Vec::new();
    let mut count_sent: u32 = 0;
    let mut count_err: u32 = 0;

    for seq in 0..args.count {
        let sent_time = Instant::now();
        count_sent += 1;
        pinger.send(host_ip, seq, &[42; 56]).context(IcmpSend)?;

        let response = pinger.recv(Duration::from_millis(args.timeout));

        match response {
            Ok(Reply::EchoReply {
                from,
                sequence_number,
                ..
            }) => {
                // Bookkeeping for stats.
                let rtt = sent_time.elapsed().as_millis();
                rtt_list.push(rtt);

                // Reverse DNS lookup for the IP, to describe it in the log line
                let domain = resolve_addr(&IpAddr::V4(from)).ok();
                let desc = match domain {
                    Some(name) => format!("{} ({})", name, from),
                    None => format!("{}", from),
                };

                println!(
                    "64 bytes from {}: icmp_seq={} ttl={} time={}",
                    desc, sequence_number, args.ttl, rtt
                )
            }

            Ok(Reply::TimeToLiveExceeded { from }) => {
                count_err += 1;
                println!("From {} icmp_seq={} Time to live exceeded", from, seq)
            }

            Ok(Reply::Timeout) => println!("Request timed out."),

            Err(err) => return Err(CliError::IcmpRecv { source: err }),
        }

        sleep(Duration::from_secs(1));
    }

    // Calculate some statistics
    let packet_loss: f64 = (count_sent - rtt_list.len() as u32) as f64 / count_sent as f64;

    println!("--- {} ping statistics ---", host_description);
    println!(
        "{} packets transmitted, {} received, {} errors, {:.2}% packet loss",
        count_sent,
        rtt_list.len(),
        count_err,
        packet_loss
    );

    if !rtt_list.is_empty() {
        let rtt_min = rtt_list.iter().copied().min().unwrap();
        let rtt_max = rtt_list.iter().copied().max().unwrap();
        let rtt_total: u128 = rtt_list.iter().sum();
        let rtt_avg: f64 = rtt_total as f64 / rtt_list.len() as f64;
        let rtt_rss: f64 = rtt_list
            .iter()
            .copied()
            .map(|x| (x as f64 - rtt_avg).powi(2))
            .sum();
        let rtt_stdev = (rtt_rss as f64 / rtt_list.len() as f64).sqrt();

        println!(
            "rtt min/avg/max/stdev = {}/{}/{}/{:.2}",
            rtt_min, rtt_avg, rtt_max, rtt_stdev
        );
    }

    Ok(())
}
