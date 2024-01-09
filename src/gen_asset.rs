pub static CHAT_MIN_JS: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/chat.min.js"));
pub static AUTO_RENDER_MIN_JS_GZ: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/auto-render.min.js.gz"));
pub static KATEX_MIN_JS_GZ: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/katex.min.js.gz"));
pub static CHAT_CSS: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/chat.css"));
pub static KATEX_MIN_CSS_GZ: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/katex.min.js.gz"));
pub static TACHYONS_MIN_CSS_GZ: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/tachyons.min.js.gz"));
