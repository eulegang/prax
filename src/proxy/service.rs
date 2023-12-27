use std::io::Write;

use hyper::header::{HeaderName, HeaderValue};
use hyper::{Request, Response};

use crate::comm::intercept_request;

use super::Rule;

pub async fn apply_request(req: &mut Request<Vec<u8>>, rules: &[Rule]) {
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

            Rule::Intercept => match intercept_request(req).await {
                Ok(true) => {}
                Ok(false) => {
                    log::warn!("can not sent to intercepter");
                }
                Err(e) => {
                    log::error!("failed to intercept: {e}");
                }
            },

            Rule::SetHeader(k, v) => {
                if let Ok(header) = HeaderValue::from_str(v) {
                    req.headers_mut()
                        .insert(HeaderName::from_bytes(k.as_bytes()).unwrap(), header);
                }
            }
        }
    }
}

pub async fn apply_response(resp: &mut Response<Vec<u8>>, rules: &[Rule]) {
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

            Rule::Intercept => {
                todo!()
            }

            Rule::SetHeader(k, v) => {
                if let Ok(header) = HeaderValue::from_str(v) {
                    resp.headers_mut()
                        .insert(HeaderName::from_bytes(k.as_bytes()).unwrap(), header);
                }
            }
        }
    }
}
