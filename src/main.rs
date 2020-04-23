use pnet::packet::icmp::echo_request::*;
use pnet::packet::icmp::*;
use pnet::packet::ip::IpNextHeaderProtocols::Icmp;
use pnet::packet::Packet;
use pnet::transport::TransportChannelType::Layer4;
use pnet::transport::TransportProtocol::Ipv4;
use pnet::transport::{icmp_packet_iter, transport_channel};
use simple_error::{try_with, SimpleError};
use std::env;
use std::net::IpAddr;

fn main() -> Result<(), SimpleError> {
    // Get the arguments
    let args: Vec<String> = env::args().collect();

    let dest_addr_str = match args.as_slice() {
        [_bin, ip, ..] => ip,
        _ => {
            println!("nope, you need args");
            return Err("What".into());
        }
    };
    let dest_addr = IpAddr::V4(dest_addr_str.parse().expect("could not parse IP address"));

    println!("pinging addr={:?}", dest_addr);

    // Set up the (tx,rx) ICMP transport
    let protocol = Layer4(Ipv4(Icmp));
    let (mut tx, mut rx) = try_with!(
        transport_channel(4096 /* buffer size */, protocol),
        "can't open transport channel"
    );
    println!("opened channel");

    let mut rx_iter = icmp_packet_iter(&mut rx);

    loop {
        let req = make_echo_request();
        println!("sending request: {:?}", req);
        let sent_size = tx.send_to(req, dest_addr).unwrap();
        println!("sent the ping. size={}", sent_size);

        match rx_iter.next() {
            Ok((packet, addr)) => println!(
                "Response from {:?}: {:?}\n\tpayload:{:?}",
                addr,
                packet,
                packet.payload()
            ),
            Err(error) => println!("Got error: {:?}", error),
        }
    }
}

fn make_echo_request() -> EchoRequestPacket<'static> {
    let buf: Vec<u8> = vec![0; MutableEchoRequestPacket::minimum_packet_size()];
    let mut packet = MutableEchoRequestPacket::owned(buf).unwrap();

    packet.set_icmp_type(IcmpTypes::EchoRequest);
    packet.set_icmp_code(IcmpCodes::NoCode);
    packet.set_identifier(0x0101);
    packet.set_sequence_number(0x0101);

    let echo_checksum = checksum(&IcmpPacket::new(packet.packet()).unwrap());
    packet.set_checksum(echo_checksum);
    packet.consume_to_immutable()
}
