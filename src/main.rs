use rand::random;
use std::env;
use std::net::Ipv4Addr;
use std::thread::sleep;
use std::time::{Duration, Instant};

mod ping;
use ping::*;

fn main() {
    // Get the arguments
    let args: Vec<String> = env::args().collect();

    let dest_addr_str = match args.as_slice() {
        [_bin, ip, ..] => ip,
        _ => "1.1.1.1",
    };
    let addr: Ipv4Addr = dest_addr_str
        .parse()
        .expect("wping: could not parse IP address");

    println!("PING {addr} ({addr}) 56(84) bytes of data.", addr = addr);

    // Open the pinger
    let mut pinger = Pinger::open().unwrap();

    let mut rtt_list: Vec<u128> = Vec::new();
    let mut count_sent: u32 = 0;

    for seq in 0..5 {
        let sent_time = Instant::now();
        count_sent += 1;
        pinger.send(addr, seq, &[42; 56]).unwrap();

        let response = pinger.recv(Duration::from_secs(10)).unwrap();

        match response {
            Some((addr, seq, _)) => {
                // Bookkeeping for stats.
                let rtt = sent_time.elapsed().as_millis();
                rtt_list.push(rtt);

                println!("64 bytes from {}: icmp_seq={} time={}", addr, seq, rtt)
            }
            None => println!("Request timed out."),
        }

        sleep(Duration::from_secs(1));
    }

    // Calculate some statistics
    let rtt_min = rtt_list.iter().copied().min().unwrap();
    let rtt_max = rtt_list.iter().copied().max().unwrap();
    let rtt_total: u128 = rtt_list.iter().sum();
    let rtt_avg: f64 = rtt_total as f64 / rtt_list.len() as f64;
    let packet_loss: f64 = (count_sent - rtt_list.len() as u32) as f64 / count_sent as f64;

    let rtt_rss: f64 = rtt_list
        .iter()
        .copied()
        .map(|x| (x as f64 - rtt_avg).powi(2))
        .sum();
    let rtt_stdev = (rtt_rss as f64 / rtt_list.len() as f64).sqrt();

    println!("--- {} ping statistics ---", addr);
    println!(
        "{} packets transmitted, {} received, {:.2}% packet loss",
        count_sent,
        rtt_list.len(),
        packet_loss
    );
    println!(
        "rtt min/avg/max/stdev = {}/{}/{}/{:.2}",
        rtt_min, rtt_avg, rtt_max, rtt_stdev
    );
}
