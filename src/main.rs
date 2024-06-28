mod bencode;
mod torrent;
mod tracker;

use hex;
use rand::Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent = torrent::read_torrent_file("../torrents/grok-1.torrent")?;

    println!("Torrent info_hash: {:?}", hex::encode(torrent.info_hash));

    // Generate a random peer_id
    let peer_id: [u8; 20] = rand::thread_rng().gen();

    match tracker::get_peers(&torrent, &peer_id).await {
        Ok(tracker_response) => println!("Tracker response: {:?}", tracker_response),
        Err(e) => println!("Error getting peers: {}", e),
    }

    Ok(())
}
