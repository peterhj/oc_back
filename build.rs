extern crate deflate;
extern crate jest;
extern crate time;

use time::{get_time};

use std::fs::{OpenOptions};
use std::io::{BufRead, Write, Cursor};
use std::path::{PathBuf};
use std::process::{Command};

fn main() {
  let t = get_time();
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=.git/logs/HEAD");
  println!("cargo:rerun-if-changed=../oc_front/.git/logs/HEAD");
  println!("cargo:rerun-if-changed=../oc_front/chat.js");
  println!("cargo:rerun-if-changed=../oc_front/auto-render.min.js");
  println!("cargo:rerun-if-changed=../oc_front/katex.min.js");
  println!("cargo:rerun-if-changed=../oc_front/style.css");
  println!("cargo:rerun-if-changed=../oc_front/katex.min.css");
  println!("cargo:rerun-if-changed=../oc_front/tachyons.min.css");
  let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
  let mut tsp_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("timestamp")).unwrap();
  write!(&mut tsp_f, "{}", t.utc().rfc3339()).unwrap();
  let res = Command::new("git")
    .current_dir(std::env::var("CARGO_MANIFEST_DIR").unwrap())
    .arg("log").arg("-n").arg("1").arg("--format=%H")
    .output().unwrap();
  assert!(res.status.success());
  let line = Cursor::new(res.stdout).lines().next().unwrap();
  let line = line.unwrap();
  let mut gch_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("git_commit_hash")).unwrap();
  write!(&mut gch_f, "{}", line).unwrap();
  let res = Command::new("git")
    .current_dir(PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../oc_front"))
    .arg("log").arg("-n").arg("1").arg("--format=%H")
    .output().unwrap();
  assert!(res.status.success());
  let line = Cursor::new(res.stdout).lines().next().unwrap();
  let line = line.unwrap();
  let mut gch_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("front_git_commit_hash")).unwrap();
  write!(&mut gch_f, "{}", line).unwrap();
  let src_min = jest::mangle_js_file("../oc_front/chat.js", &["renderMathInElement"]).unwrap();
  let mut src_min_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("chat.min.js")).unwrap();
  write!(&mut src_min_f, "{}", src_min).unwrap();
  let src_min = deflate::deflate_file_gzip("../oc_front/auto-render.min.js").unwrap();
  let mut src_min_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("auto-render.min.js.gz")).unwrap();
  write!(&mut src_min_f, "{}", src_min).unwrap();
  let src_min = deflate::deflate_file_gzip("../oc_front/katex.min.js").unwrap();
  let mut src_min_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("katex.min.js.gz")).unwrap();
  write!(&mut src_min_f, "{}", src_min).unwrap();
  let src_min = deflate::deflate_file_gzip("../oc_front/style.css").unwrap();
  let mut src_min_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("style.css.gz")).unwrap();
  write!(&mut src_min_f, "{}", src_min).unwrap();
  let src_min = deflate::deflate_file_gzip("../oc_front/katex.min.css").unwrap();
  let mut src_min_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("katex.min.css.gz")).unwrap();
  write!(&mut src_min_f, "{}", src_min).unwrap();
  let src_min = deflate::deflate_file_gzip("../oc_front/tachyons.min.css").unwrap();
  let mut src_min_f = OpenOptions::new().write(true).create(true).truncate(true)
    .open(out_dir.join("tachyons.min.css.gz")).unwrap();
  write!(&mut src_min_f, "{}", src_min).unwrap();
}
