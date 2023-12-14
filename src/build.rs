pub static GIT_COMMIT_HASH: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/git_commit_hash"));
pub static TIMESTAMP: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/timestamp"));
