use crate::torrent::Torrent;
use std::sync::Arc;
use thiserror::Error;
use tokio::time::{timeout, Duration};
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};

#[derive(Error, Debug)]
pub enum PeerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid handshake response")]
    InvalidHandshake,
    #[error("Handshake timed out")]
    Timeout,
    #[error("Connection closed by peer")]
    ConnectionClosed,
    // ... other variants ...
}

pub struct PeerConnection {
    stream: TcpStream,
    peer_id: [u8; 20],
    info_hash: [u8; 20],
}

impl PeerConnection {
    pub async fn new(
        ip: String,
        port: u16,
        torrent: &Arc<Torrent>,
        peer_id: [u8; 20],
    ) -> Result<Self, PeerError> {
        let stream = TcpStream::connect(format!("{}:{}", ip, port)).await?;

        Ok(Self {
            stream,
            peer_id,
            info_hash: torrent.info_hash,
        })
    }

    pub async fn handshake(&mut self) -> Result<(), PeerError> {
        println!("Starting handshake");

        // Timeout for the entire handshake process
        let handshake_timeout = Duration::from_secs(30); // Adjust as needed

        let handshake_process = async {
            // Construct handshake message
            let mut handshake = vec![];
            handshake.extend(b"\x13BitTorrent protocol");
            handshake.extend(&[0u8; 8]);
            handshake.extend(&self.info_hash);
            handshake.extend(&self.peer_id);

            // Send handshake
            self.stream.write_all(&handshake).await?;
            println!("Handshake sent");

            // Read response
            let mut response = vec![0u8; 68];
            let mut total_read = 0;

            while total_read < 68 {
                match self.stream.read(&mut response[total_read..]).await {
                    Ok(0) => {
                        println!("Peer closed the connection");
                        return Err(PeerError::ConnectionClosed);
                    }
                    Ok(n) => {
                        total_read += n;
                        println!("Read {} bytes, total: {}", n, total_read);
                    }
                    Err(e) => {
                        println!("Error reading from stream: {:?}", e);
                        return Err(e.into());
                    }
                }
            }

            // as string
            println!(
                "Received complete handshake response: {:?}",
                String::from_utf8_lossy(&response)
            );

            // Verify response
            if response[0] != 19 || &response[1..20] != b"BitTorrent protocol" {
                return Err(PeerError::InvalidHandshake);
            }

            Ok(())
        };

        // Execute the handshake process with a timeout
        match timeout(handshake_timeout, handshake_process).await {
            Ok(result) => result,
            Err(_) => {
                println!("Handshake timed out");
                Err(PeerError::Timeout)
            }
        }
    }

    // Add more methods for peer wire protocol communication here
}
