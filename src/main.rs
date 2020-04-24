use clap::Clap;
use std::env;
use std::net::Ipv4Addr;
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

fn main() {
    let args = Args::parse();

    let addr: Ipv4Addr = args
        .host
        .parse()
        .expect("wping: could not parse IP address");

    println!("PING {addr} ({addr}) 56(84) bytes of data.", addr = addr);

    // Open the pinger
    let mut pinger = Pinger::open(args.ttl).unwrap();

    let mut rtt_list: Vec<u128> = Vec::new();
    let mut count_sent: u32 = 0;
    let mut count_err: u32 = 0;

    for seq in 0..args.count {
        let sent_time = Instant::now();
        count_sent += 1;
        pinger.send(addr, seq, &[42; 56]).unwrap();

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

                println!(
                    "64 bytes from {}: icmp_seq={} ttl={} time={}",
                    from, sequence_number, args.ttl, rtt
                )
            }

            Ok(Reply::TimeToLiveExceeded { from }) => {
                count_err += 1;
                println!("From {} icmp_seq={} Time to live exceeded", from, seq)
            }

            Ok(Reply::Timeout) => println!("Request timed out."),

            Err(err) => println!("error: {}", err),
        }

        sleep(Duration::from_secs(1));
    }

    // Calculate some statistics
    let packet_loss: f64 = (count_sent - rtt_list.len() as u32) as f64 / count_sent as f64;

    println!("--- {} ping statistics ---", addr);
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
}
