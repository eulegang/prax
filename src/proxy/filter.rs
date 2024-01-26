use hyper::{
    header::{HeaderName, HeaderValue},
    http::uri::PathAndQuery,
    Method, StatusCode, Uri,
};

use crate::{Filter, Result};

use super::{Attr, Config, Rule};

use std::{io::Write, str::FromStr};

impl<F> Filter for Config<F>
where
    F: Filter + Sync,
{
    async fn modify_request(&self, hostname: &str, req: &mut crate::Req<Vec<u8>>) -> Result<()> {
        log::debug!("applying config request rules to {hostname}");
        let Some(target) = dbg!(&self.proxy)
            .targets
            .iter()
            .find(|t| t.hostname == hostname)
        else {
            return Ok(());
        };

        for rule in &target.req {
            log::trace!("applying request rule {rule:?}");

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

                Rule::Intercept => self.intercept.modify_request(hostname, req).await?,

                Rule::Set(attr, value) => match attr {
                    super::Attr::Method => {
                        let update = Method::from_str(value.as_str())?;

                        *req.method_mut() = update;
                    }
                    Attr::Status => {}
                    Attr::Path => {
                        let mut parts = req.uri().clone().into_parts();
                        let pq = if let Some(pq) = parts.path_and_query {
                            if let Some(query) = pq.query() {
                                PathAndQuery::from_str(&format!("{}?{}", value, query))?
                            } else {
                                PathAndQuery::from_str(&value)?
                            }
                        } else {
                            PathAndQuery::from_str(&value)?
                        };

                        parts.path_and_query = Some(pq);

                        *req.uri_mut() = Uri::from_parts(parts).unwrap();
                    }
                    Attr::Query(key) => {
                        let val = if value.is_empty() {
                            "".to_string()
                        } else {
                            format!("={value}")
                        };

                        let mut parts = req.uri().clone().into_parts();
                        let pq = if let Some(pq) = parts.path_and_query {
                            if let Some(query) = pq.query() {
                                if query.is_empty() {
                                    PathAndQuery::from_str(&format!("{}?{}{}", value, key, value))?
                                } else {
                                    PathAndQuery::from_str(&format!(
                                        "{}?{}&{}{}",
                                        value, query, key, val
                                    ))?
                                }
                            } else {
                                PathAndQuery::from_str(&format!("{}?{}{}", pq.path(), key, val))?
                            }
                        } else {
                            PathAndQuery::from_str(&format!("/?{key}{val}"))?
                        };

                        parts.path_and_query = Some(pq);

                        *req.uri_mut() = Uri::from_parts(parts).unwrap();
                    }
                    Attr::Header(key) => {
                        if let Ok(header) = HeaderValue::from_str(&value) {
                            req.headers_mut()
                                .insert(HeaderName::from_bytes(key.as_bytes()).unwrap(), header);
                        }
                    }
                    Attr::Body => {
                        *req.body_mut() = value.as_bytes().to_owned();
                    }
                },
                Rule::Subst(_, _) => todo!(),
            }
        }

        log::trace!("finished applying config request rules to {hostname}");
        Ok(())
    }

    async fn modify_response(&self, hostname: &str, res: &mut crate::Res<Vec<u8>>) -> Result<()> {
        log::debug!("applying response rules to {hostname}");
        let Some(target) = self.proxy.targets.iter().find(|t| t.hostname == hostname) else {
            return Ok(());
        };

        for rule in &target.resp {
            log::trace!("applying response rule {rule:?}");

            match rule {
                Rule::Dump => {
                    let mut buf = Vec::<u8>::new();

                    let _ = writeln!(buf, "{}", res.status());
                    for (key, value) in res.headers() {
                        if let Ok(v) = value.to_str() {
                            let _ = writeln!(buf, "{}: {}", key, v);
                        }
                    }

                    let _ = writeln!(buf);

                    buf.extend_from_slice(res.body());

                    if let Ok(s) = std::str::from_utf8(&buf) {
                        log::info!("dump res\n{s}")
                    } else {
                        log::error!("response is not text");
                    }
                }

                Rule::Intercept => self.intercept.modify_response(hostname, res).await?,

                Rule::Set(attr, value) => match attr {
                    Attr::Method => {}
                    Attr::Path => {}
                    Attr::Query(_) => {}
                    Attr::Status => {
                        let status = StatusCode::from_str(value).unwrap();
                        *res.status_mut() = status;
                    }
                    Attr::Header(key) => {
                        if let Ok(header) = HeaderValue::from_str(&value) {
                            res.headers_mut()
                                .insert(HeaderName::from_bytes(key.as_bytes()).unwrap(), header);
                        }
                    }
                    Attr::Body => {
                        *res.body_mut() = value.as_bytes().to_owned();
                    }
                },
                Rule::Subst(_, _) => todo!(),
            }
        }

        Ok(())
    }
}
