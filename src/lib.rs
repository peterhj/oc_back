extern crate constant_time_eq;
extern crate flate;
extern crate oc_engine;
extern crate once_cell;
extern crate rustc_serialize;
extern crate service_base;

use crate::secret_asset::*;

use constant_time_eq::*;
use flate::deflate::{Encoder as DeflateEncoder};
use oc_engine::*;
use rustc_serialize::base64;
use rustc_serialize::json;
use service_base::prelude::*;
use service_base::chan::*;
use service_base::daemon::{protect};
use service_base::route::*;

use std::cell::{RefCell};
use std::collections::{BTreeMap};
use std::convert::{TryInto};
use std::io::{Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc};
use std::sync::mpsc::{SyncSender, Receiver, sync_channel};
use std::thread::{sleep, spawn};
use std::time::{Duration};

pub mod build;
pub mod gen_asset;
pub mod secret_asset;
pub mod static_asset;

pub fn service_main() -> () {
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
  let (back_tx, engine_rx) = sync_channel(8);
  /*let (engine_tx, back_rx) = sync_channel(8);
  let router = Arc::new(routes(back_tx, back_rx));*/
  let router = Arc::new(routes(back_tx));
  let _ = spawn(move || {
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
      loop {
        match engine_rx.recv() {
          //Ok(req) => {}
          Ok((req, engine_tx)) => {
            let rep = match chan.query(&Msg::Ext(req)) {
              Ok(Msg::Ext(rep)) => rep,
              _ => break
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
  static TL_CACHE: RefCell<BTreeMap<String, Result<Vec<u8>, ()>>> = RefCell::new(BTreeMap::new());
}

pub fn routes(back_tx: SyncSender<(Engine, MsgReceiver<EngineMsg>)>, /*back_rx: Receiver<EngineMsg>*/) -> Router {
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
    ok().with_payload_str_mime(rendered, Mime::TextHtml).into()
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
    let (data, mime) = match asset {
      "tachyons.min.css" => {
        (crate::static_asset::TACHYONS_MIN_CSS, Mime::TextCss)
      }
      "katex.min.css" => {
        (crate::static_asset::KATEX_MIN_CSS, Mime::TextCss)
      }
      "style.css" => {
        (crate::static_asset::STYLE_CSS, Mime::TextCss)
      }
      "katex.min.js" => {
        (crate::static_asset::KATEX_MIN_JS, Mime::ApplicationJavascript)
      }
      "auto-render.min.js" => {
        (crate::static_asset::AUTO_RENDER_MIN_JS, Mime::ApplicationJavascript)
      }
      "chat.js" => {
        let template = crate::static_asset::CHAT_JS;
        let rendered = template.replace("{{host}}", &format!("https://zanodu.xyz/olympiadchat/{}", base64::URL_SAFE.encode(&token)));
        let (data, mime) = (rendered, Mime::ApplicationJavascript);
        return ok().with_payload_str_mime(data, mime).into();
      }
      _ => return None
    };
    TL_CACHE.with(move|cache| {
      let mut retry = false;
      let mut cache = cache.borrow_mut();
      loop {
        match cache.get(asset) {
          None => {
            assert!(!retry);
            let mut buf = Vec::new();
            let mut enc = DeflateEncoder::new(&mut buf);
            match enc.write_all(data.as_bytes()) {
              Err(e) => {
                drop(enc);
                println!("DEBUG:  oc_back: route:   deflate? write err: {:?}", e);
                cache.insert(asset.to_owned(), Err(()));
              }
              Ok(_) => {
                match enc.finish().into_result() {
                  Err(e) => {
                    println!("DEBUG:  oc_back: route:   deflate? finish err: {:?}", e);
                    cache.insert(asset.to_owned(), Err(()));
                  }
                  Ok(_) => {
                    println!("DEBUG:  oc_back: route:   deflate? ok: olen={} len={}", data.len(), buf.len());
                    cache.insert(asset.to_owned(), Ok(buf));
                  }
                }
              }
            }
            retry = true;
            continue;
          }
          Some(Err(_)) => {
            println!("DEBUG:  oc_back: route:   no cache: olen={}", data.len());
            break ok().with_payload_str_mime(data, mime).into();
          }
          Some(Ok(compressed)) => {
            println!("DEBUG:  oc_back: route:   cache ok: len={}", compressed.len());
            // FIXME: preserve utf-8 charset.
            break ok().with_payload_bin_mime_encoding(compressed.to_owned(), mime, HttpEncoding::Deflate).into();
          }
        }
        unreachable!();
      }
    })
  }));
  let back_tx = back_tx.clone();
  //let back_rx = back_rx.clone();
  let tokens0 = &STATIC_ACCESS_TOKENS;
  router.insert_post(("olympiadchat", "{token:base64}", "wapi", "{endpoint}"), Box::new(move |_, args, _| {
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
        struct Reply {
          seq_nr: i64,
        };
        let reply = Reply{seq_nr: 1};
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
      "post" => {
        // FIXME
        let val = "Let $ABC$ be a triangle.".to_owned();
        let (engine_tx, back_rx) = sync_channel(1);
        match back_tx.send((EngineMsg::EMQ(EngineMatReq{
          val,
        }), engine_tx)) {
          Err(_) => {
            return None;
          }
          Ok(_) => {}
        }
        #[derive(RustcEncodable)]
        struct Reply {
          err: bool,
        };
        //let reply = Reply{err: false};
        let reply = match back_rx.recv() {
          Err(_) => {
            return None;
          }
          Ok(EngineMsg::EMP(EngineMatRep{
            res,
          })) => {
            Reply{err: res.is_err()}
          }
          Ok(_) => {
            return None;
          }
        };
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
