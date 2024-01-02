extern crate rng;
extern crate rustc_serialize;
extern crate time;

use rng::os::{getrandom};
use rustc_serialize::base64::{URL_SAFE};
use time::{now_utc};

use std::env::{args};

fn main() {
  let argv: Vec<_> = args().collect();
  if argv.len() <= 1 {
    eprintln!("usage: {} <ident>", argv[0]);
    return;
  }
  let t = now_utc();
  let mut buf = Vec::with_capacity(36);
  buf.resize(36, 0_u8);
  getrandom(&mut buf).unwrap();
  let s = URL_SAFE.encode(&buf);
  println!("{},{},{}", t.rfc3339(), argv[1], s);
}
