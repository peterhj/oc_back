use once_cell::sync::{Lazy};
use rustc_serialize::base64;

use std::io::{BufRead, BufReader, Cursor};

pub const _ACCESS_TOKENS: &'static [u8] = include_bytes!("../secrets/access_tokens.txt");

pub static STATIC_ACCESS_TOKENS: Lazy<Vec<((), String, Box<[u8]>)>> = Lazy::new(|| static_access_tokens());

fn static_access_tokens() -> Vec<((), String, Box<[u8]>)> {
  let mut tokens = Vec::new();
  for line in BufReader::new(Cursor::new(crate::secret_asset::_ACCESS_TOKENS)).lines() {
    let line = line.unwrap();
    let parts: Vec<_> = line.split(",").collect();
    tokens.push(((), parts[1].into(), base64::decode_from_str(parts[2]).unwrap().into()));
  }
  tokens
}
