use std::net::{Ipv4Addr, UdpSocket};
use std::time::{Duration, Instant};
// Struct to represent an SSDP message
#[derive(Debug, Clone)]
pub struct SsdpMessage {
    pub source_address: String,
    pub source_port: u16,
    pub server: String,
    pub location: String,
    pub nt: String,
    pub usn: String,
    pub cache_control: String,
    pub custom_fields: Vec<(String, String)>,
}

impl std::fmt::Display for SsdpMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "SsdpMessage {{ source_address: {}, source_port: {}, server: {}, location: {}, nt: {}, usn: {}, cache_control: {}, custom_fields: {:?} }}",
            self.source_address, self.source_port, self.server, self.location, self.nt, self.usn, self.cache_control, self.custom_fields
        )
    }
}

impl SsdpMessage {
    fn new() -> Self {
        Self {
            source_address: String::new(),
            source_port: 0,
            server: String::new(),
            location: String::new(),
            nt: String::new(),
            usn: String::new(),
            cache_control: String::new(),
            custom_fields: Vec::new(),
        }
    }

    pub fn from_message(message: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ssdp_message = SsdpMessage::new();

        for line in message.lines() {
            if line.is_empty() {
                continue; // Skip empty lines
            }

            // If the NOTIFY header is found, we need to skip the first line (the NOTIFY line itself)
            if line.to_lowercase().starts_with("notify") {
                continue;
            }

            let mut parts = line.splitn(2, ':');
            let header = parts.next().ok_or("Invalid SSDP message")?.trim();
            let value = parts.next().ok_or("Invalid SSDP message")?.trim();

            match header.to_lowercase().as_str() {
                "host" => {
                    if let Some((host, port)) = value.split_once(':') {
                        ssdp_message.source_address = host.to_string();
                        ssdp_message.source_port = port.parse::<u16>()?;
                    }
                }
                "server" => ssdp_message.server = value.to_string(),
                "location" => ssdp_message.location = value.to_string(),
                "nt" => ssdp_message.nt = value.to_string(),
                "usn" => ssdp_message.usn = value.to_string(),
                "cache-control" => ssdp_message.cache_control = value.to_string(),
                _ => {
                    ssdp_message
                        .custom_fields
                        .push((header.to_string(), value.to_string()));
                }
            }
        }

        Ok(ssdp_message)
    }
}

pub struct SsdpListener {
    port: u16,
}

impl SsdpListener {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn listen(
        &self,
        duration: Duration,
    ) -> Result<Vec<SsdpMessage>, Box<dyn std::error::Error>> {
        // Create a UDP socket bound to the specified port
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", self.port))?;

        // Join the SSDP multicast group
        socket.join_multicast_v4(
            &Ipv4Addr::new(239, 255, 255, 250),
            &Ipv4Addr::new(0, 0, 0, 0),
        )?;

        // Buffer to store the received message
        let mut buf = [0u8; 2048];

        let start_time = Instant::now();
        let mut messages = Vec::new();

        println!(
            "Listening for SSDP NOTIFY messages on port {} for {:?}...",
            self.port, duration
        );

        // Receive messages until the specified duration elapses
        while Instant::now() - start_time < duration {
            match socket.recv_from(&mut buf[..]) {
                Ok((size, _)) => {
                    // Parse and handle the SSDP NOTIFY message
                    let message = std::str::from_utf8(&buf[..size])?.to_string();
                    messages.push(SsdpMessage::from_message(&message)?);
                }
                Err(e) => {
                    eprintln!("Error receiving SSDP message: {}", e);
                    break; // Exit loop on error
                }
            }
        }

        Ok(messages)
    }
}
