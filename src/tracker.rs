use crate::bencode::{parse_bencode, BencodeValue};
use crate::torrent::Torrent;
use reqwest;
use url::Url;

#[derive(Debug)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug)]
pub struct TrackerResponse {
    pub interval: u32,
    pub peers: Vec<Peer>,
}

pub async fn get_peers(
    torrent: &Torrent,
    peer_id: &[u8; 20],
) -> Result<TrackerResponse, Box<dyn std::error::Error>> {
    let url = build_tracker_url(torrent, peer_id)?;
    println!("Requesting tracker URL: {}", url); // Debug print

    // check if scheme is http or https else return error
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Invalid URL scheme".into());
    }

    let response = reqwest::get(&url).await?.bytes().await?;

    parse_tracker_response(&response)
}

fn build_tracker_url(
    torrent: &Torrent,
    peer_id: &[u8; 20],
) -> Result<String, Box<dyn std::error::Error>> {
    // check if announce is non-empty
    let base = if !torrent.announce.is_empty() {
        &torrent.announce
    } else {
        &torrent.url_list[0]
    };

    let mut url = Url::parse(base)?;

    // Encode the info_hash correctly
    let info_hash_encoded: String = torrent
        .info_hash
        .iter()
        .map(|&byte| format!("%{:02X}", byte))
        .collect();

    // Encode the peer_id correctly
    let peer_id_encoded: String = peer_id
        .iter()
        .map(|&byte| format!("%{:02X}", byte))
        .collect();

    // Manually construct the query string
    let query = format!(
        "info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left={}",
        info_hash_encoded, peer_id_encoded, torrent.length
    );

    // Set the entire query string at once
    url.set_query(Some(&query));

    Ok(url.to_string())
}

fn parse_tracker_response(response: &[u8]) -> Result<TrackerResponse, Box<dyn std::error::Error>> {
    let (bencode_value, _) = parse_bencode(response)?;

    if let BencodeValue::Dictionary(root_dict) = bencode_value {
        if let Some(BencodeValue::ByteString(failure_reason)) =
            root_dict.get(&b"failure reason"[..])
        {
            return Err(format!(
                "Tracker returned failure: {}",
                String::from_utf8_lossy(failure_reason)
            )
            .into());
        }

        let interval = match root_dict.get(&b"interval"[..]) {
            Some(BencodeValue::Integer(i)) => *i as u32,
            _ => return Err("Missing or invalid 'interval' field".into()),
        };

        // peers is a list
        let peers = match root_dict.get(&b"peers"[..]) {
            Some(BencodeValue::List(l)) => parse_compact_peers(l),
            _ => return Err("Missing or invalid 'peers' field".into()),
        };

        Ok(TrackerResponse { interval, peers })
    } else {
        Err("Invalid tracker response structure".into())
    }
}

fn parse_compact_peers(peers: &[BencodeValue]) -> Vec<Peer> {
    let mut result = Vec::new();

    for peer in peers {
        if let BencodeValue::Dictionary(peer_dict) = peer {
            let ip = match peer_dict.get(&b"ip"[..]) {
                Some(BencodeValue::ByteString(ip)) => String::from_utf8_lossy(ip).to_string(),
                _ => continue,
            };

            let port = match peer_dict.get(&b"port"[..]) {
                Some(BencodeValue::Integer(port)) => *port as u16,
                _ => continue,
            };

            result.push(Peer { ip, port });
        }
    }

    result
}
