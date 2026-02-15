use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::mpsc;
use tokio::time::{interval, timeout, Duration, Instant};

use crate::packet::tcp::TcpFrameReader;
use crate::packet::{incoming, outgoing, tcp};
use crate::types::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Resolving,
    Connected,
    CodeRunning,
}

pub struct ConnectionManager {
    team: u32,
    use_usb: bool,
    state: ConnectionState,
    target_addr: Option<SocketAddr>,
    sequence: u16,
    last_received: Option<Instant>,
    trip_times: Vec<f64>, // rolling window for avg trip time
    lost_packets: u32,
    sent_count: u32,
    received_count: u32,
}

impl ConnectionManager {
    pub fn new(team: u32) -> Self {
        Self {
            team,
            use_usb: false,
            state: ConnectionState::Disconnected,
            target_addr: None,
            sequence: 0,
            last_received: None,
            trip_times: Vec::new(),
            lost_packets: 0,
            sent_count: 0,
            received_count: 0,
        }
    }

    pub fn set_team(&mut self, team: u32) {
        if self.team != team {
            self.team = team;
            self.state = ConnectionState::Disconnected;
            self.target_addr = None;
        }
    }

    pub fn set_usb_mode(&mut self, usb: bool) {
        self.use_usb = usb;
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn trip_time_ms(&self) -> f64 {
        if self.trip_times.is_empty() {
            0.0
        } else {
            self.trip_times.iter().sum::<f64>() / self.trip_times.len() as f64
        }
    }

    pub fn lost_packets(&self) -> u32 {
        self.lost_packets
    }

    /// Convert team number to static IP: 10.TE.AM.2
    pub fn team_to_ip(team: u32) -> IpAddr {
        let te = (team / 100) as u8;
        let am = (team % 100) as u8;
        IpAddr::V4(Ipv4Addr::new(10, te, am, 2))
    }

    /// Resolve the roboRIO address. Returns the socket address to connect to.
    pub async fn resolve_address(&mut self) -> SocketAddr {
        self.state = ConnectionState::Resolving;

        // Try USB mode first if enabled
        if self.use_usb {
            let usb_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(172, 22, 11, 2)), 1110);
            log::info!("Using USB address: {}", usb_addr);
            self.target_addr = Some(usb_addr);
            return usb_addr;
        }

        // Try mDNS resolution
        let mdns_hostname = format!("roboRIO-{}-FRC.local", self.team);
        log::info!("Attempting mDNS lookup for {}", mdns_hostname);

        if let Some(addr) = self.try_mdns_lookup(&mdns_hostname).await {
            log::info!("Resolved via mDNS: {}", addr);
            self.target_addr = Some(addr);
            return addr;
        }

        // Fallback to static IP
        let static_ip = Self::team_to_ip(self.team);
        let static_addr = SocketAddr::new(static_ip, 1110);
        log::info!("Using static IP fallback: {}", static_addr);
        self.target_addr = Some(static_addr);
        static_addr
    }

    async fn try_mdns_lookup(&self, _hostname: &str) -> Option<SocketAddr> {
        // Try mDNS resolution with a 2-second timeout
        let mdns_result = timeout(Duration::from_secs(2), async {
            // Create mDNS service discovery
            let mdns = mdns_sd::ServiceDaemon::new().ok()?;

            // Browse for the roboRIO service
            let service_type = "_ni._tcp.local.";
            let receiver = mdns.browse(service_type).ok()?;

            // Wait for service events with timeout
            let browse_timeout = Duration::from_secs(2);
            let start = Instant::now();

            while start.elapsed() < browse_timeout {
                if let Ok(event) = timeout(Duration::from_millis(100), receiver.recv_async()).await {
                    if let Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) = event {
                        // Check if this is the roboRIO we're looking for
                        if info.get_fullname().contains(&self.team.to_string()) {
                            if let Some(addr) = info.get_addresses().iter().next() {
                                return Some(SocketAddr::new(*addr, 1110));
                            }
                        }
                    }
                }
            }

            None::<SocketAddr>
        }).await;

        mdns_result.ok().flatten()
    }

    /// The main connection loop. Call this to start communication.
    ///
    /// - `control_rx`: receives (ControlFlags, RequestFlags, Vec<JoystickData>, Alliance) from the DriverStation
    /// - `packet_tx`: sends parsed RioPackets to the DriverStation
    /// - `tcp_message_tx`: sends parsed TCP messages
    /// - `tcp_outbound_rx`: receives outbound TCP frames to send
    pub async fn run(
        &mut self,
        mut control_rx: mpsc::UnboundedReceiver<(ControlFlags, RequestFlags, Vec<JoystickData>, Alliance)>,
        packet_tx: mpsc::UnboundedSender<incoming::RioPacket>,
        tcp_message_tx: mpsc::UnboundedSender<TcpMessage>,
        mut tcp_outbound_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    ) {
        let mut reconnect_attempts = 0u32;

        // Store latest control state
        let mut latest_control = (
            ControlFlags::default(),
            RequestFlags::default(),
            Vec::new(),
            Alliance::new(AllianceColor::Red, 1),
        );

        loop {
            // Resolve address
            let target = self.resolve_address().await;

            // Bind UDP socket for receiving
            let udp_socket = match UdpSocket::bind("0.0.0.0:1150").await {
                Ok(sock) => sock,
                Err(e) => {
                    log::error!("Failed to bind UDP socket: {}", e);
                    self.backoff_delay(reconnect_attempts).await;
                    reconnect_attempts += 1;
                    continue;
                }
            };

            log::info!("UDP socket bound to 0.0.0.0:1150");

            // Spawn UDP send task
            let send_socket = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(sock) => sock,
                Err(e) => {
                    log::error!("Failed to bind send socket: {}", e);
                    self.backoff_delay(reconnect_attempts).await;
                    reconnect_attempts += 1;
                    continue;
                }
            };

            let mut ticker = interval(Duration::from_millis(20));
            let mut sequence = 0u16;
            let receive_timeout = Duration::from_secs(1);
            let mut buf = vec![0u8; 2048];

            // TCP connection state
            let target_ip = match target {
                SocketAddr::V4(addr) => IpAddr::V4(*addr.ip()),
                SocketAddr::V6(addr) => IpAddr::V6(*addr.ip()),
            };
            let tcp_target = SocketAddr::new(target_ip, 1740);

            // Try to establish TCP connection (non-blocking, optional)
            let mut tcp_stream: Option<TcpStream> = None;
            let mut tcp_reader = TcpFrameReader::new();
            let mut tcp_read_buf = vec![0u8; 4096];
            let mut tcp_connect_attempt = Box::pin(timeout(Duration::from_secs(3), TcpStream::connect(tcp_target)));

            // Main UDP send/receive loop
            let mut connection_active = true;

            while connection_active {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Send control packet
                        let (control, request, joysticks, alliance) = &latest_control;
                        let packet = outgoing::build_ds_packet(
                            sequence,
                            control,
                            request,
                            alliance,
                            joysticks,
                        );

                        if let Err(e) = send_socket.send_to(&packet, target).await {
                            log::warn!("UDP send error: {}", e);
                        }

                        sequence = sequence.wrapping_add(1);
                    }

                    Some(new_state) = control_rx.recv() => {
                        latest_control = new_state;
                    }

                    result = timeout(receive_timeout, udp_socket.recv_from(&mut buf)) => {
                        match result {
                            Ok(Ok((len, _addr))) => {
                                match incoming::parse_rio_packet(&buf[..len]) {
                                    Ok(rio_packet) => {
                                        if packet_tx.send(rio_packet).is_err() {
                                            log::warn!("Failed to send parsed packet");
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to parse roboRIO packet: {}", e);
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                log::warn!("UDP receive error: {}", e);
                            }
                            Err(_) => {
                                // Timeout - no packet received
                                log::warn!("No UDP packet received for 1 second, disconnecting");
                                connection_active = false;
                            }
                        }
                    }

                    // TCP connection attempt
                    result = &mut tcp_connect_attempt, if tcp_stream.is_none() => {
                        match result {
                            Ok(Ok(stream)) => {
                                log::info!("TCP connected to {}", tcp_target);
                                tcp_stream = Some(stream);
                            }
                            Ok(Err(e)) => {
                                log::warn!("TCP connection failed: {}", e);
                                // Retry connection after a delay
                                tokio::time::sleep(Duration::from_secs(2)).await;
                                tcp_connect_attempt = Box::pin(timeout(Duration::from_secs(3), TcpStream::connect(tcp_target)));
                            }
                            Err(_) => {
                                log::warn!("TCP connection timed out");
                                // Retry connection
                                tcp_connect_attempt = Box::pin(timeout(Duration::from_secs(3), TcpStream::connect(tcp_target)));
                            }
                        }
                    }

                    // TCP read
                    result = async {
                        if let Some(stream) = tcp_stream.as_mut() {
                            stream.read(&mut tcp_read_buf).await
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        match result {
                            Ok(0) => {
                                log::info!("TCP connection closed by remote");
                                tcp_stream = None;
                                tcp_connect_attempt = Box::pin(timeout(Duration::from_secs(3), TcpStream::connect(tcp_target)));
                            }
                            Ok(n) => {
                                tcp_reader.feed(&tcp_read_buf[..n]);
                                while let Some((tag, payload)) = tcp_reader.next_frame() {
                                    if let Some(msg) = tcp::parse_tcp_message(tag, &payload) {
                                        if tcp_message_tx.send(msg).is_err() {
                                            log::warn!("Failed to send TCP message");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("TCP read error: {}", e);
                                tcp_stream = None;
                                tcp_connect_attempt = Box::pin(timeout(Duration::from_secs(3), TcpStream::connect(tcp_target)));
                            }
                        }
                    }

                    // TCP write
                    Some(frame) = tcp_outbound_rx.recv() => {
                        if let Some(stream) = tcp_stream.as_mut() {
                            if let Err(e) = stream.write_all(&frame).await {
                                log::warn!("TCP write error: {}", e);
                                tcp_stream = None;
                                tcp_connect_attempt = Box::pin(timeout(Duration::from_secs(3), TcpStream::connect(tcp_target)));
                            }
                        }
                    }
                }
            }

            // Connection lost, update state and retry
            self.state = ConnectionState::Disconnected;
            log::info!("Connection lost, will retry after backoff");

            self.backoff_delay(reconnect_attempts).await;
            reconnect_attempts += 1;
        }
    }

    async fn backoff_delay(&self, attempt: u32) {
        let delay_ms = std::cmp::min(100 * 2u64.pow(attempt), 2000);
        log::debug!("Backing off for {}ms (attempt {})", delay_ms, attempt);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_to_ip() {
        assert_eq!(ConnectionManager::team_to_ip(1234).to_string(), "10.12.34.2");
        assert_eq!(ConnectionManager::team_to_ip(254).to_string(), "10.2.54.2");
        assert_eq!(ConnectionManager::team_to_ip(1).to_string(), "10.0.1.2");
        assert_eq!(ConnectionManager::team_to_ip(9999).to_string(), "10.99.99.2");
    }

    #[test]
    fn test_initial_state() {
        let cm = ConnectionManager::new(1234);
        assert_eq!(cm.state(), ConnectionState::Disconnected);
        assert_eq!(cm.trip_time_ms(), 0.0);
        assert_eq!(cm.lost_packets(), 0);
    }

    #[test]
    fn test_set_team() {
        let mut cm = ConnectionManager::new(1234);
        cm.set_team(5678);
        // Should reset state since team changed
        assert_eq!(cm.state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_backoff_capping() {
        // Test that backoff calculation caps at 2 seconds
        let backoff = |attempt: u32| -> u64 {
            std::cmp::min(100 * 2u64.pow(attempt), 2000)
        };
        assert_eq!(backoff(0), 100);
        assert_eq!(backoff(1), 200);
        assert_eq!(backoff(2), 400);
        assert_eq!(backoff(3), 800);
        assert_eq!(backoff(4), 1600);
        assert_eq!(backoff(5), 2000); // capped
        assert_eq!(backoff(10), 2000); // still capped
    }
}
