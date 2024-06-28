use crate::bencode::{encode, parse_bencode, BencodeValue};
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub struct Torrent {
    pub announce: String,
    pub info_hash: [u8; 20],
    pub name: String,
    pub piece_length: u64,
    pub pieces: Vec<[u8; 20]>,
    pub length: u64, // single file length
    pub url_list: Vec<String>,
}

pub fn read_file(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

pub fn read_torrent_file(path: &str) -> Result<Torrent, Box<dyn std::error::Error>> {
    let contents = read_file(path)?;

    // Parse the Bencode data
    let (bencode_value, _) = parse_bencode(&contents)?;

    let encoded = encode(&bencode_value)?;
    assert_eq!(contents, encoded);

    // Extract the torrent information
    if let BencodeValue::Dictionary(root_dict) = bencode_value {
        // either announce or url-list is required
        let announce = match root_dict.get(&b"announce"[..]) {
            Some(BencodeValue::ByteString(s)) => String::from_utf8(s.clone())?,
            _ => String::new(),
        };

        let urls = match root_dict.get(&b"url-list"[..]) {
            Some(BencodeValue::List(l)) => l.iter().cloned().collect(),
            _ => Vec::new(),
        };

        let mut url_list = Vec::new();
        if !urls.is_empty() {
            for url in urls {
                if let BencodeValue::ByteString(s) = url {
                    url_list.push(String::from_utf8(s.clone())?);
                }
            }
        }

        if announce.is_empty() && url_list.is_empty() {
            return Err("Missing 'announce' or 'url-list' field".into());
        }

        let info = match root_dict.get(&b"info"[..]) {
            Some(BencodeValue::Dictionary(d)) => d,
            _ => return Err("Missing or invalid 'info' dictionary".into()),
        };
        let name = match info.get(&b"name"[..]) {
            Some(BencodeValue::ByteString(s)) => String::from_utf8(s.clone())?,
            _ => return Err("Missing or invalid 'name' field".into()),
        };

        let piece_length = match info.get(&b"piece length"[..]) {
            Some(BencodeValue::Integer(i)) => *i as u64,
            _ => return Err("Missing or invalid 'piece length' field".into()),
        };

        let pieces: Vec<_> = match info.get(&b"pieces"[..]) {
            Some(BencodeValue::ByteString(s)) => s
                .chunks(20)
                .map(|chunk| {
                    let mut array = [0u8; 20];
                    array.copy_from_slice(chunk);
                    array
                })
                .collect(),
            _ => return Err("Missing or invalid 'pieces' field".into()),
        };

        // default to 0 if length is not present
        let length = match info.get(&b"length"[..]) {
            Some(BencodeValue::Integer(i)) => *i as u64,
            _ => 0,
        };

        // Calculate the info hash
        let info_bytes = encode(root_dict.get(&b"info"[..]).unwrap())?;
        let mut hasher = Sha1::new();
        hasher.update(&info_bytes);
        let info_hash: [u8; 20] = hasher.finalize().into();

        Ok(Torrent {
            announce,
            info_hash,
            name,
            piece_length,
            pieces,
            length,
            url_list,
        })
    } else {
        Err("Invalid torrent file structure".into())
    }
}
