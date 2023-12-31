extern crate constant_time_eq;
extern crate deflate;
//extern crate flate;
//extern crate minify_js;
extern crate oc_engine;
extern crate once_cell;
extern crate rustc_serialize;
extern crate service_base;
extern crate time;

use crate::secret_asset::*;

use constant_time_eq::*;
//use flate::deflate::{Encoder as DeflateEncoder};
use oc_engine::*;
use once_cell::sync::{Lazy};
use rustc_serialize::base64;
use rustc_serialize::json;
use service_base::prelude::*;
use service_base::chan::*;
use service_base::daemon::{protect};
use service_base::route::*;
use time::{get_time_usec};

use std::cell::{RefCell};
use std::collections::{BTreeMap};
use std::convert::{TryInto};
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{Write};
use std::net::{TcpListener, TcpStream};
use std::str::{from_utf8};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{SyncSender, Receiver, sync_channel};
use std::thread::{sleep, spawn};
use std::time::{Duration};

pub mod build;
pub mod gen_asset;
pub mod secret_asset;
pub mod static_asset;

static DATA_LOCK: Lazy<Mutex<Option<File>>> = Lazy::new(|| Mutex::new(None));

pub fn service_main() -> () {
  println!("INFO:   build: {}.{}", crate::build::timestamp(), crate::build::digest2());
  let host = "127.0.0.1";
  let port_start = 9000;
  let port_fin = 9009;
  let mut port = port_start;
  let bind = loop {
    match TcpListener::bind((host, port)) {
      Err(_) => {}
      Ok(bind) => break bind
    }
    if port >= port_fin {
      panic!("ERROR:  oc_back::service_main: failed to bind port");
    }
    port += 1;
  };
  println!("INFO:   listening on {}:{}", host, port);
  let chroot_dir = "/var/lib/oc_back/new_root";
  protect(chroot_dir, 297, 297).unwrap();
  // NB: mkdir /var out-of-band (need to be root).
  let _ = create_dir_all("/var/db").ok();
  {
    let mut f = OpenOptions::new()
      .create(true).write(true).append(true).truncate(false)
      .open("/var/db/data.jsonl").unwrap();
    *DATA_LOCK.lock().unwrap() = Some(f);
  }
  let (back_tx, engine_rx) = sync_channel::<(EngineMsg, SyncSender<EngineMsg>)>(64);
  /*let (engine_tx, back_rx) = sync_channel(8);
  let router = Arc::new(routes(back_tx, back_rx));*/
  let router = Arc::new(routes(back_tx));
  let _ = spawn(move || {
    let mut retry: Option<(EngineMsg, SyncSender<EngineMsg>)> = None;
    'outer: loop {
      let port_start = 10000;
      let port_fin = 10099;
      let mut port = port_start;
      let stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
          Err(_) => {}
          Ok(stream) => break stream
        }
        if port >= port_fin {
          let _ = retry.take();
          match engine_rx.try_recv() {
            Ok(_) => {
              //let _ = engine_tx.send(None);
              loop {
                match engine_rx.try_recv() {
                  Ok(_) => {
                    //let _ = engine_tx.send(None);
                  }
                  _ => {
                    break;
                  }
                }
              }
            }
            //Err(TryRecvError::Empty) => {}
            _ => {}
          }
          sleep(Duration::from_secs(1));
          continue 'outer;
        }
        port += 1;
      };
      println!("INFO:   engine: connected on port={}", port);
      let mut chan = Chan::<EngineMsg>::new(stream);
      if let Some((req, engine_tx)) = retry.take() {
        let req = Msg::Ext(req);
        let rep = match chan.query(&req) {
          Ok(Msg::Ext(rep)) => rep,
          _ => {
            println!("DEBUG:  engine:   query: immediately failed");
            let req = match req {
              Msg::Ext(req) => req,
              _ => unreachable!()
            };
            retry = Some((req, engine_tx));
            break;
          }
        };
        match engine_tx.send(rep) {
          Ok(_) => {}
          _ => {}
        }
      }
      loop {
        match engine_rx.recv() {
          Ok((req, engine_tx)) => {
            let req = Msg::Ext(req);
            let rep = match chan.query(&req) {
              Ok(Msg::Ext(rep)) => rep,
              _ => {
                println!("DEBUG:  engine:   query: failed");
                let req = match req {
                  Msg::Ext(req) => req,
                  _ => unreachable!()
                };
                retry = Some((req, engine_tx));
                break;
              }
            };
            match engine_tx.send(rep) {
              Ok(_) => {}
              _ => {}
            }
          }
          _ => {
          }
        }
      }
      println!("INFO:   engine: disconnected");
    }
  });
  let pool: SpawnPool = SpawnPool::new(bind);
  pool.replying({
    // FIXME FIXME: pre and post functions.
    //println!("INFO:   gateway: connected on {}:{}", host, port);
    // TODO TODO
    let router = router.clone();
    Arc::new(move |query| {
      match query {
        Msg::OKQ => Msg::OKR,
        Msg::H1Q(req) => {
          let port = 443;
          match router.match_(port, &req) {
            Err(_) | Ok(None) => Msg::Top,
            Ok(Some(rep)) => Msg::H1P(rep)
          }
        }
        _ => {
          unimplemented!();
          //Msg::Bot
        }
      }
    })
  });
}

thread_local! {
  static TL_CACHE: RefCell<BTreeMap<String, (CacheTag, Result<Vec<u8>, ()>)>> = RefCell::new(BTreeMap::new());
}

#[derive(Clone, Copy, Debug)]
//#[repr(u8)]
pub enum CacheTag {
  Deflate,
  //MinifyJs,
}

pub fn cached<K: AsRef<str>, V: AsRef<str>>(key: K, tag: CacheTag, data: V, mime: Mime) -> Option<HttpResponse> {
  let key = key.as_ref();
  let data = data.as_ref();
  TL_CACHE.with(move |cache| {
    let mut retry = false;
    let mut cache = cache.borrow_mut();
    loop {
      match cache.get(key) {
        None => {
          assert!(!retry);
          let t0 = get_time_usec();
          match tag {
            CacheTag::Deflate => {
              let buf = deflate::deflate_bytes(data.as_bytes());
              let t1 = get_time_usec();
              let dt = (t1 - t0).to_std().unwrap();
              let dt = dt.as_secs() as f64 + 1.0e-9 * dt.subsec_nanos() as f64;
              println!("DEBUG:  oc_back: route:   deflate? ok: olen={} len={} dt={:.03} s", data.len(), buf.len(), dt);
              cache.insert(key.to_owned(), (tag, Ok(buf)));
            }
            /*CacheTag::Deflate => {
              let mut buf = Vec::new();
              let mut enc = DeflateEncoder::new(&mut buf);
              match enc.write_all(data.as_bytes()) {
                Err(e) => {
                  drop(enc);
                  println!("DEBUG:  oc_back: route:   deflate? write err: {:?}", e);
                  cache.insert(key.to_owned(), (tag, Err(())));
                }
                Ok(_) => {
                  match enc.finish().into_result() {
                    Err(e) => {
                      println!("DEBUG:  oc_back: route:   deflate? finish err: {:?}", e);
                      cache.insert(key.to_owned(), (tag, Err(())));
                    }
                    Ok(_) => {
                      let t1 = get_time_usec();
                      let dt = (t1 - t0).to_std().unwrap();
                      let dt = dt.as_secs() as f64 + 1.0e-9 * dt.subsec_nanos() as f64;
                      println!("DEBUG:  oc_back: route:   deflate? ok: olen={} len={} dt={:.03} s", data.len(), buf.len(), dt);
                      cache.insert(key.to_owned(), (tag, Ok(buf)));
                    }
                  }
                }
              }
            }*/
            /*CacheTag::MinifyJs => {
              match minify_js::minify_oneshot(data.as_bytes()) {
                Err(_) => {
                  println!("DEBUG:  oc_back: route:   minify js? err");
                  cache.insert(key.to_owned(), (tag, Err(())));
                }
                Ok(buf) => {
                  let t1 = get_time_usec();
                  let dt = (t1 - t0).to_std().unwrap();
                  let dt = dt.as_secs() as f64 + 1.0e-9 * dt.subsec_nanos() as f64;
                  println!("DEBUG:  oc_back: route:   minify js? ok: olen={} len={} dt={:.03} s", data.len(), buf.len(), dt);
                  cache.insert(key.to_owned(), (tag, Ok(buf)));
                }
              }
            }*/
          }
          retry = true;
          continue;
        }
        Some((_, Err(_))) => {
          println!("DEBUG:  oc_back: route:   no cache: olen={}", data.len());
          break ok().with_payload_str_mime(data, mime).into();
        }
        Some((CacheTag::Deflate, Ok(compressed))) => {
          println!("DEBUG:  oc_back: route:   cache ok: len={}", compressed.len());
          break ok().with_payload_bin(compressed.to_owned(), mime, HttpCharset::Utf8, HttpEncoding::Deflate).into();
        }
        /*Some((CacheTag::MinifyJs, Ok(minified))) => {
          println!("DEBUG:  oc_back: route:   cache ok: len={}", minified.len());
          break ok().with_payload_str_mime(from_utf8(minified).unwrap().to_owned(), mime).into();
        }*/
      }
      unreachable!();
    }
  })
}

pub fn routes(back_tx: SyncSender<(EngineMsg, SyncSender<EngineMsg>)>, /*back_rx: Receiver<EngineMsg>*/) -> Router {
  let mut router = Router::new();
  router.insert_get((), Box::new(move |_, _, _| {
    println!("DEBUG:  oc_back: route: GET /");
    ok().with_payload_str_mime("Hello world!\n", Mime::TextHtml).into()
  }));
  router.insert_get("about", Box::new(move |_, _, _| {
    println!("DEBUG:  oc_back: route: GET /about");
    ok().with_payload_str_mime("It&rsquo;s about time.\n", Mime::TextHtml).into()
  }));
  //let tokens0 = static_access_tokens();
  let tokens0 = &STATIC_ACCESS_TOKENS;
  router.insert_get(("olympiadchat", "{token:base64}"), Box::new(move |_, args, _| {
    println!("DEBUG:  oc_back: route: GET /olympiadchat/{{token}}");
    let token = args.get("token")?.as_base64()?;
    let ident = {
      let mut mat_ident = None;
      for &(_, ref ident, ref token0) in tokens0.iter() {
        if constant_time_eq(token0, token) {
          mat_ident = Some(ident.clone());
          break;
        }
      }
      mat_ident?
    };
    println!("DEBUG:  oc_back: route:   ident={:?}", ident);
    let template = crate::static_asset::CHAT_HTML;
    // FIXME: aho corasick.
    let rendered = template
                  .replace("{{build}}", &format!("{}.{}", crate::build::date(), crate::build::digest()))
                  .replace("{{host}}", &format!("https://zanodu.xyz/olympiadchat/{}", base64::URL_SAFE.encode(&token)));
    let (data, mime) = (rendered, Mime::TextHtml);
    println!("DEBUG:  oc_back: route:   ok: len={}", data.len());
    ok().with_payload_str_mime(data, mime).into()
  }));
  //let tokens0 = static_access_tokens();
  let tokens0 = &STATIC_ACCESS_TOKENS;
  router.insert_get(("olympiadchat", "{token:base64}", "{asset}"), Box::new(move |_, args, _| {
    println!("DEBUG:  oc_back: route: GET /olympiadchat/{{token}}/{{asset}}");
    let token = args.get("token")?.as_base64()?;
    let ident = {
      let mut mat_ident = None;
      for &(_, ref ident, ref token0) in tokens0.iter() {
        if constant_time_eq(token0, token) {
          mat_ident = Some(ident.clone());
          break;
        }
      }
      mat_ident?
    };
    println!("DEBUG:  oc_back: route:   ident={:?}", ident);
    let asset = args.get("asset")?.as_str()?;
    println!("DEBUG:  oc_back: route:   asset={:?}", asset);
    // FIXME: cache control.
    let (tag, data, mime) = match asset {
      "tachyons.min.css" => {
        (CacheTag::Deflate, crate::static_asset::TACHYONS_MIN_CSS, Mime::TextCss)
      }
      "katex.min.css" => {
        (CacheTag::Deflate, crate::static_asset::KATEX_MIN_CSS, Mime::TextCss)
      }
      "style.css" |
      "style.min.css" => {
        (CacheTag::Deflate, crate::static_asset::STYLE_CSS, Mime::TextCss)
      }
      "katex.min.js" => {
        (CacheTag::Deflate, crate::static_asset::KATEX_MIN_JS, Mime::ApplicationJavascript)
      }
      "auto-render.min.js" => {
        (CacheTag::Deflate, crate::static_asset::AUTO_RENDER_MIN_JS, Mime::ApplicationJavascript)
      }
      "chat.js" |
      "chat.min.js" => {
        /*let template = crate::static_asset::CHAT_JS;*/
        let template = crate::gen_asset::CHAT_MIN_JS;
        let rendered = template.replace("{{host}}", &format!("https://zanodu.xyz/olympiadchat/{}", base64::URL_SAFE.encode(&token)));
        let (data, mime) = (rendered, Mime::ApplicationJavascript);
        println!("DEBUG:  oc_back: route:   ok: len={}", data.len());
        return ok().with_payload_str_mime(data, mime).into();
        // FIXME: cache key needs to be per-ident.
        /*let (tag, data, mime) = (CacheTag::MinifyJs, rendered, Mime::ApplicationJavascript);
        return cached(asset, tag, data, mime);*/
      }
      _ => return None
    };
    cached(asset, tag, data, mime)
  }));
  let back_tx = back_tx.clone();
  //let back_rx = back_rx.clone();
  let tokens0 = &STATIC_ACCESS_TOKENS;
  router.insert_post(("olympiadchat", "{token:base64}", "wapi", "{endpoint}"), Box::new(move |_, args, hreq| {
    let t0 = get_time_usec();
    println!("DEBUG:  oc_back: route: POST /olympiadchat/{{token}}/wapi/{{endpoint}}");
    let token = args.get("token")?.as_base64()?;
    let ident = {
      let mut mat_ident = None;
      for &(_, ref ident, ref token0) in tokens0.iter() {
        if constant_time_eq(token0, token) {
          mat_ident = Some(ident.clone());
          break;
        }
      }
      mat_ident?
    };
    println!("DEBUG:  oc_back: route:   ident={:?}", ident);
    let endpoint = args.get("endpoint")?.as_str()?;
    println!("DEBUG:  oc_back: route:   endpoint={:?}", endpoint);
    // TODO TODO
    match endpoint {
      "hi" => {
        #[derive(RustcEncodable)]
        struct QRow {
          t0: String,
          req: String,
          ident: String,
          //seq_nr: i64,
        }
        #[derive(RustcEncodable)]
        struct Reply {
          seq_nr: i64,
        };
        let row = QRow{
          t0: format!("{}", t0.utc().rfc3339_nsec()).into(),
          req: "hi".into(),
          ident: ident.clone(),
          //seq_nr,
        };
        if let Some(mut f) = DATA_LOCK.lock().unwrap().as_mut() {
          writeln!(&mut f, "{}", json::encode_to_string(&row).unwrap()).unwrap();
        }
        let reply = Reply{seq_nr: 1};
        match json::encode_to_string(&reply) {
          Err(_) => {
            // FIXME: error payload.
            println!("DEBUG:  oc_back: route:   hi: err");
            return None;
          }
          Ok(data) => {
            println!("DEBUG:  oc_back: route:   hi: ok");
            return created().with_payload_str_mime(data, Mime::ApplicationJson.into()).into();
          }
        }
      }
      "post" => {
        #[derive(RustcEncodable)]
        struct QRow {
          t0: String,
          req: String,
          ident: String,
          seq_nr: i64,
          val: String,
        }
        #[derive(RustcDecodable)]
        struct Payload {
          q: String,
          seq_nr: i64,
        }
        let payload: Payload = match hreq.payload.as_ref() {
          None => {
            println!("DEBUG:  oc_back: route:   post: no query payload");
            return None;
          }
          Some(payload) => {
            let s = match from_utf8(payload.as_raw_bytes()) {
              Err(_) => {
                println!("DEBUG:  oc_back: route:   post: query payload is not utf-8");
                return None;
              }
              Ok(s) => s
            };
            match json::decode_from_str(s) {
              Err(_) => {
                println!("DEBUG:  oc_back: route:   post: invalid json decode");
                return None;
              }
              Ok(payload) => payload
            }
          }
        };
        let seq_nr = payload.seq_nr;
        let val = payload.q;
        let row = QRow{
          t0: format!("{}", t0.utc().rfc3339_nsec()).into(),
          req: "post".into(),
          ident: ident.clone(),
          seq_nr,
          val: val.clone(),
        };
        if let Some(mut f) = DATA_LOCK.lock().unwrap().as_mut() {
          writeln!(&mut f, "{}", json::encode_to_string(&row).unwrap()).unwrap();
        }
        let (engine_tx, back_rx) = sync_channel(1);
        match back_tx.send((EngineMsg::EMQ(EngineMatReq{
          val,
        }), engine_tx)) {
          Err(_) => {
            println!("DEBUG:  oc_back: route:   post: tx failed");
            return None;
          }
          Ok(_) => {}
        }
        #[derive(RustcEncodable)]
        struct PRow {
          t0: String,
          rep: String,
          ident: String,
          seq_nr: i64,
          res: i8,
        }
        #[derive(RustcEncodable)]
        struct Reply {
          //err: bool,
          err: i8,
          mrk_s: Option<i32>,
          mrk_e: Option<i32>,
          val: Vec<String>,
          //wip: bool,
          wip: i8,
        };
        let reply = match back_rx.recv() {
          Err(_) => {
            println!("DEBUG:  oc_back: route:   post: rx failed");
            return None;
          }
          Ok(EngineMsg::EMP(EngineMatRep{
            res,
            mrk_s,
            mrk_e,
            val,
            wip,
          })) => {
            let t0 = get_time_usec();
            println!("DEBUG:  oc_back: route:   post: rx ok: res={:?}", res);
            let err = res as i8;
            let wip = if wip { 1 } else { 0 };
            let row = PRow{
              t0: format!("{}", t0.utc().rfc3339_nsec()).into(),
              rep: "post".into(),
              ident: ident.clone(),
              seq_nr,
              res: err,
            };
            if let Some(mut f) = DATA_LOCK.lock().unwrap().as_mut() {
              writeln!(&mut f, "{}", json::encode_to_string(&row).unwrap()).unwrap();
            }
            Reply{err, mrk_s, mrk_e, val, wip}
          }
          Ok(_) => {
            println!("DEBUG:  oc_back: route:   post: invalid rx");
            return None;
          }
        };
        match json::encode_to_string(&reply) {
          Err(_) => {
            // FIXME: error payload.
            println!("DEBUG:  oc_back: route:   post: err");
            return None;
          }
          Ok(data) => {
            println!("DEBUG:  oc_back: route:   post: ok");
            return created().with_payload_str_mime(data, Mime::ApplicationJson.into()).into();
          }
        }
      }
      "poll" => {
        #[derive(RustcEncodable)]
        struct Reply {
          ready: bool,
        };
        let reply = Reply{ready: false};
        match json::encode_to_string(&reply) {
          Err(_) => {
            // FIXME: error payload.
            return None;
          }
          Ok(data) => {
            return created().with_payload_str_mime(data, Mime::ApplicationJson.into()).into();
          }
        }
      }
      _ => return None
    }
    None
    //unimplemented!();
  }));
  // TODO TODO
  router
}
