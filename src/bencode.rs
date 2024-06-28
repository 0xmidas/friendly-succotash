use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum BencodeValue {
    Integer(i64),
    ByteString(Vec<u8>),
    List(Vec<BencodeValue>),
    Dictionary(HashMap<Vec<u8>, BencodeValue>),
}

/* Parses the dictionary of the torrent file, not a general prupose bencode parser! */

pub fn parse_bencode(input: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    match input.first() {
        Some(b'i') => parse_integer(&input[1..]),
        Some(b'0'..=b'9') => parse_byte_string(input),
        Some(b'l') => parse_list(&input[1..]),
        Some(b'd') => parse_dictionary(&input[1..]),
        _ => Err("Invalid bencode format".to_string()),
    }
}

fn parse_integer(input: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    let end = input
        .iter()
        .position(|&x| x == b'e')
        .ok_or("Integer not terminated")?;
    let num_str = std::str::from_utf8(&input[..end]).map_err(|_| "Invalid UTF-8 in integer")?;
    let num = num_str
        .parse::<i64>()
        .map_err(|_| "Invalid integer format")?;
    Ok((BencodeValue::Integer(num), &input[end + 1..]))
}

fn parse_byte_string(input: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    let colon_pos = input
        .iter()
        .position(|&x| x == b':')
        .ok_or("Byte string length not terminated")?;
    let length_str = std::str::from_utf8(&input[..colon_pos])
        .map_err(|_| "Invalid UTF-8 in byte string length")?;
    let length = length_str
        .parse::<usize>()
        .map_err(|_| "Invalid byte string length")?;
    let content_start = colon_pos + 1;
    let content_end = content_start + length;
    if content_end > input.len() {
        return Err("Byte string content too short".to_string());
    }
    let content = input[content_start..content_end].to_vec();
    Ok((BencodeValue::ByteString(content), &input[content_end..]))
}

fn parse_list(input: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    let mut result = Vec::new();
    let mut remaining = input;
    while !remaining.is_empty() && remaining[0] != b'e' {
        let (value, rest) = parse_bencode(remaining)?;
        result.push(value);
        remaining = rest;
    }
    if remaining.is_empty() {
        return Err("List not terminated".to_string());
    }
    Ok((BencodeValue::List(result), &remaining[1..]))
}

fn parse_dictionary(input: &[u8]) -> Result<(BencodeValue, &[u8]), String> {
    let mut result = HashMap::new();
    let mut remaining = input;
    while !remaining.is_empty() && remaining[0] != b'e' {
        let (key, rest) = parse_byte_string(remaining)?;
        let (value, rest) = parse_bencode(rest)?;
        if let BencodeValue::ByteString(key_bytes) = key {
            result.insert(key_bytes, value);
        } else {
            return Err("Dictionary key must be a byte string".to_string());
        }
        remaining = rest;
    }
    if remaining.is_empty() {
        return Err("Dictionary not terminated".to_string());
    }
    Ok((BencodeValue::Dictionary(result), &remaining[1..]))
}

pub fn encode(value: &BencodeValue) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match value {
        BencodeValue::Integer(i) => Ok(format!("i{}e", i).into_bytes()),
        BencodeValue::ByteString(s) => {
            Ok([format!("{}:", s.len()).into_bytes(), s.clone()].concat())
        }
        BencodeValue::List(l) => {
            let mut result = vec![b'l'];
            for item in l {
                result.extend(encode(item)?);
            }
            result.push(b'e');
            Ok(result)
        }
        BencodeValue::Dictionary(d) => {
            let mut result = vec![b'd'];
            let mut keys: Vec<_> = d.keys().collect();
            keys.sort();
            for key in keys {
                result.extend(encode(&BencodeValue::ByteString(key.to_vec()))?);
                result.extend(encode(d.get(key).unwrap())?);
            }
            result.push(b'e');
            Ok(result)
        }
    }
}
