extern crate constant_time_eq;
extern crate rustc_serialize;
extern crate service_base;

use constant_time_eq::*;
use rustc_serialize::base64;
use service_base::prelude::*;
use service_base::chan::*;
use service_base::daemon::{protect};
use service_base::route::*;

use std::convert::{TryInto};
use std::io::{BufRead, BufReader, Cursor};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc};
use std::sync::mpsc::{SyncSender, Receiver, sync_channel};
use std::thread::{sleep, spawn};
use std::time::{Duration};

pub mod gen_asset;
pub mod secret_asset;
pub mod static_asset;

pub fn service_main() -> () {
  let (front_tx, back_rx) = sync_channel(8);
  let (back_tx, front_rx) = sync_channel(8);
  let _ = spawn(move || {
    'outer: loop {
      let port_start = 10000;
      let port_fin = 10009;
      let mut port = port_start;
      let stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
          Err(_) => {}
          Ok(stream) => break stream
        }
        if port >= port_fin {
          match back_rx.try_recv() {
            Ok(_) => {
              let _ = back_tx.send(None);
              loop {
                match back_rx.try_recv() {
                  Ok(_) => {
                    let _ = back_tx.send(None);
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
      let mut chan = Chan::new(stream);
      loop {
        match back_rx.recv() {
          Ok(req) => {
            let rep = match chan.query(&Msg::JSO(req)) {
              Ok(Msg::JSO(rep)) => Some(rep),
              _ => break
            };
            match back_tx.send(rep) {
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
  let router = Arc::new(routes());
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
  let pool = SpawnPool::new(bind);
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

pub fn static_access_tokens() -> Vec<((), String, Box<[u8]>)> {
  let mut tokens = Vec::new();
  for line in BufReader::new(Cursor::new(crate::secret_asset::ACCESS_TOKENS)).lines() {
    let line = line.unwrap();
    let parts: Vec<_> = line.split(",").collect();
    tokens.push((
    (),
    parts[1].into(),
    base64::decode_from_str(parts[2]).unwrap().into()));
  }
  tokens
}

pub fn routes() -> Router {
  let mut router = Router::new();
  router.insert_get((), Box::new(move |_, _, _| {
    println!("DEBUG:  oc_back: route: /");
    let mut rep = HttpResponse::ok();
    rep.set_payload_str_with_mime("Hello world!\n", Mime::TextHtml);
    rep.into()
  }));
  router.insert_get("about", Box::new(move |_, _, _| {
    println!("DEBUG:  oc_back: route: /about");
    let mut rep = HttpResponse::ok();
    rep.set_payload_str_with_mime("It&rsquo;s about time.\n", Mime::TextHtml);
    rep.into()
  }));
  let tokens0 = static_access_tokens();
  router.insert_get(("olympiadchat", "{token:base64}"), Box::new(move |_, args, _| {
    println!("DEBUG:  oc_back: route: /olympiadchat/{{token}}");
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
    let html = template.replace("{{host}}", &format!("https://zanodu.xyz/olympiadchat/{}", base64::URL_SAFE.encode(&token)));
    let mut rep = HttpResponse::ok();
    rep.set_payload_str_with_mime(html, Mime::TextHtml);
    rep.into()
    /*
    let template: _ = crate::static_asset::CHAT_HTML.into();
    let rendered = template.render(_);
    let mut rep = HttpResponse::ok();
    rep.set_payload_str_with_mime(rendered, Mime::TextHtml);
    rep.into()
    */
  }));
  let tokens0 = static_access_tokens();
  router.insert_get(("olympiadchat", "{token:base64}", "{asset}"), Box::new(move |_, args, _| {
    println!("DEBUG:  oc_back: route: /olympiadchat/{{token}}/{{asset}}");
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
        (crate::static_asset::CHAT_JS, Mime::ApplicationJavascript)
      }
      _ => return None
    };
    let mut rep = HttpResponse::ok();
    rep.set_payload_str_with_mime(data, mime);
    rep.into()
  }));
  /*router.insert_get(("olympiadchat", "{token:base64}", "api", "{endpoint}"), Box::new(move |_, args, _| {
    let token = args.get("token")?.as_base64()?;
    /*let ident = {
      let mut mat_ident = None;
      for &(_, ref ident, ref token0) in tokens0.iter() {
        if constant_time_eq(token0, token) {
          mat_ident = Some(ident.clone());
          break;
        }
      }
      mat_ident?
    };*/
    let endpoint = args.get("endpoint")?.as_str()?;
    unimplemented!();
  }));*/
  // TODO TODO
  router
}
