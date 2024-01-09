pub static CHAT_MIN_JS: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/chat.min.js"));
pub static AUTO_RENDER_MIN_JS_GZ: &'static [u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/auto-render.min.js.gz"));
pub static KATEX_MIN_JS_GZ: &'static [u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/katex.min.js.gz"));
pub static STYLE_CSS_GZ: &'static [u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/style.css.gz"));
pub static KATEX_MIN_CSS_GZ: &'static [u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/katex.min.css.gz"));
pub static TACHYONS_MIN_CSS_GZ: &'static [u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/tachyons.min.css.gz"));
