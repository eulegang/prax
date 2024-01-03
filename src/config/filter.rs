use hyper::header::{HeaderName, HeaderValue};

use crate::srv::{Filter, Result};

use super::{Config, Rule};

use std::io::Write;

impl Filter for Config {
    async fn modify_request(
        &self,
        hostname: &str,
        req: &mut crate::srv::Req<Vec<u8>>,
    ) -> Result<()> {
        let proxy = self.proxy.lock().await;

        log::debug!("applying request rules to {hostname}");
        let Some(target) = proxy.targets.iter().find(|t| t.hostname == hostname) else {
            return Ok(());
        };

        for rule in &target.req {
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

                Rule::Intercept => match self.nvim.as_ref() {
                    Some(nvim) => {
                        let lines = {
                            let nvim = nvim.lock().await;
                            let mut view = nvim.view.lock().await;
                            view.intercept_request(req).await?
                        };

                        let lines = lines.await?;
                        {
                            let nvim = nvim.lock().await;
                            let mut view = nvim.view.lock().await;
                            if view.retrieve_intercept_request(lines, req).await? {}
                        }
                    }
                    None => {
                        log::warn!("can not send to intercepter");
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

        Ok(())
    }

    async fn modify_response(
        &self,
        hostname: &str,
        res: &mut crate::srv::Res<Vec<u8>>,
    ) -> Result<()> {
        let proxy = self.proxy.lock().await;

        log::debug!("applying request rules to {hostname}");
        let Some(target) = proxy.targets.iter().find(|t| t.hostname == hostname) else {
            return Ok(());
        };

        for rule in &target.resp {
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

                Rule::Intercept => match self.nvim.as_ref() {
                    Some(nvim) => {
                        let lines = {
                            let nvim = nvim.lock().await;
                            let mut view = nvim.view.lock().await;
                            view.intercept_response(res).await?
                        };

                        let lines = lines.await?;
                        {
                            let nvim = nvim.lock().await;
                            let mut view = nvim.view.lock().await;
                            if view.retrieve_intercept_response(lines, res).await? {}
                        }
                    }
                    None => {
                        log::warn!("can not send to intercepter");
                    }
                },

                Rule::SetHeader(k, v) => {
                    if let Ok(header) = HeaderValue::from_str(v) {
                        res.headers_mut()
                            .insert(HeaderName::from_bytes(k.as_bytes()).unwrap(), header);
                    }
                }
            }
        }

        Ok(())
    }
}
