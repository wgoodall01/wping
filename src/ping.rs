use pnet::packet::icmp::echo_reply::*;
use pnet::packet::icmp::echo_request::{IcmpCodes, *};
use pnet::packet::icmp::*;
use pnet::packet::Packet;
use pnet::transport;
use rand::random;
use snafu::{ResultExt, Snafu};
use std::{io, net, net::IpAddr, time::Duration};

#[derive(Debug, Snafu)]
pub enum PingError {
    #[snafu(display("cannot open ICMP transport: {}", source))]
    ChannelOpen { source: std::io::Error },

    #[snafu(display("could not send ICMP packet: {}", source))]
    IcmpSend { source: io::Error },

    #[snafu(display("could not receive ICMP packet: {}", source))]
    IcmpRecv { source: io::Error },

    #[snafu(display("received unexpected packet: {:?}", packet))]
    UnexpectedPacket { packet: IcmpPacket<'static> },

    #[snafu(display("received malformed packet: {:?}", packet))]
    MalformedPacket { packet: IcmpPacket<'static> },
}

type PingResult<T> = Result<T, PingError>;

pub struct Pinger {
    // underlying pnet icmp transport channels
    tx: transport::TransportSender,
    rx: transport::TransportReceiver,

    // number to be sent with echo requests as the 'identifier' field
    identifier: u16,
}

#[derive(Debug)]
pub struct Echo {
    pub identifier: u16,
    pub sequence_number: u16,
    pub payload: Vec<u8>,
}

impl Pinger {
    pub fn open() -> PingResult<Pinger> {
        use pnet::packet::ip::IpNextHeaderProtocols::Icmp;
        use pnet::transport::{TransportChannelType::Layer4, TransportProtocol::Ipv4};
        use pnet::transport::{TransportReceiver, TransportSender};

        let proto = Layer4(Ipv4(Icmp));
        let (tx, rx): (TransportSender, TransportReceiver) =
            transport::transport_channel(4096, proto).context(ChannelOpen)?;

        // Generate a random identifier for this pinger.
        let identifier: u16 = random();

        Ok(Pinger { tx, rx, identifier })
    }

    pub fn send(
        &mut self,
        addr: net::Ipv4Addr,
        sequence_number: u16,
        payload: &[u8],
    ) -> PingResult<usize> {
        // Allocate some memory for the packet
        let buf: Vec<u8> = vec![0; MutableEchoRequestPacket::minimum_packet_size() + payload.len()];
        let mut packet = MutableEchoRequestPacket::owned(buf).unwrap();

        // Construct the echo request packet
        packet.set_icmp_type(IcmpTypes::EchoRequest);
        packet.set_icmp_code(IcmpCodes::NoCode);
        packet.set_identifier(self.identifier);
        packet.set_sequence_number(sequence_number);
        packet.set_payload(payload);

        // Calculate the packet's checksum
        let check = checksum(&IcmpPacket::new(packet.packet()).unwrap());
        packet.set_checksum(check);
        let request = packet.consume_to_immutable();

        // Then, actually send the packet
        self.tx
            .send_to(request, net::IpAddr::V4(addr))
            .context(IcmpSend)
    }

    pub fn recv(&mut self, timeout: Duration) -> PingResult<Option<(net::Ipv4Addr, u16, Vec<u8>)>> {
        use pnet::transport::icmp_packet_iter;

        let mut rx_queue = icmp_packet_iter(&mut self.rx);

        // Retry to ignore packets with the wrong identifier
        loop {
            return match rx_queue.next_with_timeout(timeout) {
                Ok(Some((packet, recv_addr))) => {
                    // Check the type of the address
                    let addr = match recv_addr {
                        IpAddr::V4(ad) => ad,
                        IpAddr::V6(ad) => {
                            panic!("got impossible response from IPv6 address: {}", ad)
                        }
                    };

                    // TODO: support other kinds of ICMP response than echo replies.
                    match packet.get_icmp_type() {
                        IcmpTypes::EchoReply => {
                            let reply = EchoReplyPacket::new(&packet.packet()).context(
                                MalformedPacket {
                                    packet: clone_packet(packet),
                                },
                            )?;

                            // Skip echo replies with the wrong identifier.
                            if reply.get_identifier() != self.identifier {
                                continue;
                            }

                            Ok(Some((
                                addr,
                                reply.get_sequence_number(),
                                Vec::from(reply.payload()),
                            )))
                        }
                        _ => Err(PingError::UnexpectedPacket {
                            packet: clone_packet(packet),
                        }),
                    }
                }

                // We have a timeout.
                Ok(None) => Ok(None),

                // Some error has happened when receiving the packet.
                Err(err) => Err(PingError::IcmpRecv { source: err }),
            };
        }
    }
}

fn clone_packet<'a>(packet: IcmpPacket<'a>) -> IcmpPacket<'static> {
    let backing_buf = packet.packet();
    let cloned: Vec<u8> = backing_buf.into();
    IcmpPacket::owned(cloned).unwrap()
}
