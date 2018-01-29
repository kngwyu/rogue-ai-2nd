use std::io::{BufRead, BufReader};
use std::str::from_utf8;
use dangeon::Dangeon;
use consts::*;

#[cfg(test)]
pub fn buf_to_str(buf: &Vec<Vec<u8>>) -> String {
    let mut res = String::new();
    let len = buf.len();
    for (i, v) in buf.iter().enumerate() {
        res.push_str(from_utf8(&v).unwrap());
        if i < len - 1 {
            res.push('\n');
        }
    }
    res
}

#[cfg(test)]
pub fn str_to_buf(s: &str) -> Vec<Vec<u8>> {
    let mut res = Vec::new();
    let mut buf = String::new();
    let mut reader = BufReader::new(s.as_bytes());
    while let Ok(n) = reader.read_line(&mut buf) {
        if n == 0 || buf.pop() != Some('\n') {
            break;
        }
        if buf.is_empty() {
            continue;
        }
        let mut v = buf.as_bytes().to_owned();
        while v.len() < COLUMNS {
            v.push(b' ');
        }
        res.push(v);
        buf.clear();
    }
    while res.len() < LINES {
        res.push(vec![b' '; COLUMNS]);
    }
    res
}

#[cfg(test)]
pub fn make_dangeon(s: &str) -> Dangeon {
    let mut res = Dangeon::default();
    let buf = str_to_buf(s);
    res.merge(&buf);
    res
}
