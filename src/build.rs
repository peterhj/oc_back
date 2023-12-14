pub static GIT_COMMIT_HASH: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/git_commit_hash"));
pub static TIMESTAMP: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/timestamp"));

pub fn date() -> &'static str {
  TIMESTAMP.get( .. 10).unwrap()
}

pub fn digest() -> &'static str {
  GIT_COMMIT_HASH.get( .. 9).unwrap()
}
