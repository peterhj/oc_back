extern crate http1;
extern crate service_base;

use service_base::prelude::*;
use service_base::route::*;

use std::convert::{TryInto};
use std::net::{TcpListener, TcpStream};

pub mod gen_asset;
pub mod secret_asset;
pub mod static_asset;

pub fn service_main() -> () {
  let router = routes();
  let port_start = 9000;
  let port_fin = 9009;
  let mut port = port_start;
  let bind = loop {
    match TcpListener::bind(("127.0.0.1", port)) {
      Err(_) => {}
      Ok(bind) => break bind
    }
    if port == port_fin {
      panic!("ERROR:  oc_back::service_main: failed to bind port");
    }
    port += 1;
  };
  let pool = SpawnPool::new(bind);
  pool.replying(|query| {
    // TODO TODO
    unimplemented!();
    /*match query.try_into() {
      Ok(DecodedMsg::OK) => {
        Msg::OK
      }
      Ok(DecodedMsg::HttpReq(req)) => {
        // TODO
        unimplemented!();
      }
      _ => {
        unimplemented!();
      }
    }*/
  });
}

pub fn routes() -> Router {
  let mut router = Router::new();
  router.insert_get((), Box::new(move |_, _, _| {
    unimplemented!();
  }));
  router.insert_get(("olympiadchat", "{token:base64}"), Box::new(move |_, args, _| {
    let token = args.get("token")?.as_base64()?;
    unimplemented!();
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
    /*
    let template: _ = crate::static_asset::INDEX_HTML.into();
    let rendered = template.render(_);
    let mut rep = HttpResponse::ok();
    rep.set_payload_with_mime(rendered, http1::Mime::TextHtml);
    rep.into()
    */
  }));
  router.insert_get(("olympiadchat", "{token:base64}", "api", "{endpoint}"), Box::new(move |_, args, _| {
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
  }));
  // TODO TODO
  router
}
