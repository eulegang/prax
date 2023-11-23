use std::io::Write;

use hyper::body::Incoming;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{Request, Response};

use super::{Elem, Rule};

pub fn apply_request(req: &mut Request<Incoming>, rules: &[Rule]) {
    for rule in rules {
        match rule {
            Rule::Dump => {
                let mut buf = Vec::<u8>::new();

                let _ = writeln!(buf, "{}", req.uri().path());
                for (key, value) in req.headers() {
                    if let Ok(v) = value.to_str() {
                        let _ = writeln!(buf, "{}: {}", key, v);
                    }
                }

                if let Ok(s) = std::str::from_utf8(&buf) {
                    log::info!("dump resp \n{s}")
                } else {
                    log::error!("response is not text");
                }
            }

            Rule::SetHeader(k, v) => {
                if let Ok(header) = HeaderValue::from_str(v) {
                    req.headers_mut()
                        .insert(HeaderName::from_bytes(k.as_bytes()).unwrap(), header);
                }
            }
            Rule::Log(Elem::Path) => log::info!("Path: {}", req.uri().path()),
            Rule::Log(Elem::Method) => log::info!("Method: {}", req.method()),
            Rule::Log(Elem::Header(h)) => {
                if let Some(value) = req.headers().get(h) {
                    log::info!("Header ({}): {}", h, value.to_str().unwrap_or("not string"));
                } else {
                    log::info!("Header ({}): (none)", h);
                }
            }
            Rule::Log(Elem::Query(q)) => {
                if let Some(qa) = req.uri().query() {
                    log::info!("todo query! {qa}");
                } else {
                    log::info!("Query ({}): (none)", q)
                }
            }

            Rule::Log(Elem::Body) => (),
            Rule::Log(Elem::Status) => (),
        }
    }
}

pub fn apply_response(resp: &mut Response<Vec<u8>>, rules: &[Rule]) {
    for rule in rules {
        match rule {
            Rule::Dump => {
                let mut buf = Vec::<u8>::new();

                let _ = writeln!(buf, "{}", resp.status());
                for (key, value) in resp.headers() {
                    if let Ok(v) = value.to_str() {
                        let _ = writeln!(buf, "{}: {}", key, v);
                    }
                }

                let _ = writeln!(buf);

                buf.extend_from_slice(resp.body());

                if let Ok(s) = std::str::from_utf8(&buf) {
                    log::info!("dump resp\n{s}")
                } else {
                    log::error!("response is not text");
                }
            }

            Rule::SetHeader(k, v) => {
                if let Ok(header) = HeaderValue::from_str(v) {
                    resp.headers_mut()
                        .insert(HeaderName::from_bytes(k.as_bytes()).unwrap(), header);
                }
            }
            Rule::Log(Elem::Path) => (),
            Rule::Log(Elem::Method) => (),
            Rule::Log(Elem::Query(_)) => (),

            Rule::Log(Elem::Header(h)) => {
                if let Some(value) = resp.headers().get(h) {
                    log::info!("Header ({}): {}", h, value.to_str().unwrap_or("not string"));
                } else {
                    log::info!("Header ({}): (none)", h);
                }
            }
            Rule::Log(Elem::Status) => {
                log::info!("Status: {}", resp.status())
            }
            Rule::Log(Elem::Body) => {
                if let Ok(content) = std::str::from_utf8(resp.body()) {
                    log::info!("Body: {}", content);
                } else {
                    log::info!("Body: (binary)");
                }
            }
        }
    }
}
