mod bencode;
mod peer_connection;
mod torrent;
mod tracker;

use hex;
use peer_connection::PeerConnection;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent = Arc::new(torrent::read_torrent_file("../torrents/grok-1.torrent")?);
    println!("Torrent info_hash: {:?}", hex::encode(torrent.info_hash));

    // Generate a random peer_id
    let peer_id: [u8; 20] = rand::thread_rng().gen();

    // get peers from tracker
    let tracker_response = tracker::get_peers(&torrent, &peer_id).await?;

    // Connect to a peer
    let peer = &tracker_response.peers[0];

    let mut peer_connection =
        PeerConnection::new(peer.ip.clone(), peer.port, &torrent, peer_id).await?;

    println!("Peer connection established");

    // use tokio spawn to run the handshake concurrently
    let handshake =
        tokio::spawn(
            async move { timeout(Duration::from_secs(10), peer_connection.handshake()).await },
        );

    // wait for the handshake to complete and check if it was successful
    let _ = handshake.await??;

    println!("Handshake successful");

    Ok(())
}
