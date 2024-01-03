use std::str::FromStr;

use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap, StatusCode, Uri,
};

use crate::srv;

pub fn req_to_lines(req: &hyper::Request<Vec<u8>>) -> srv::Result<Vec<String>> {
    let mut res = Vec::new();

    let mut status = String::new();

    status.push_str(req.method().to_string().as_ref());
    status.push(' ');
    status.push_str(req.uri().path());
    if let Some(s) = req.uri().query() {
        status.push('?');
        status.push_str(s);
    }

    res.push(status);

    for (k, v) in req.headers() {
        res.push(format!("{}: {}", k, v.to_str()?));
    }

    res.push(String::new());

    let body = std::str::from_utf8(req.body())?;

    for line in body.lines() {
        res.push(line.to_string());
    }

    Ok(res)
}

pub fn resp_to_lines(resp: &hyper::Response<Vec<u8>>) -> srv::Result<Vec<String>> {
    let mut res = Vec::new();

    res.push(resp.status().as_u16().to_string());

    for (k, v) in resp.headers() {
        res.push(format!("{}: {}", k, v.to_str()?));
    }

    res.push(String::new());

    let body = std::str::from_utf8(resp.body())?;

    for line in body.lines() {
        res.push(line.to_string());
    }

    Ok(res)
}

pub fn imprint_lines(req: &mut hyper::Request<Vec<u8>>, lines: Vec<String>) -> srv::Result<()> {
    let Some(status) = lines.get(0) else {
        return Err(srv::Error::InterceptMalformed);
    };

    let (method, uri) = extract_status(req.uri(), status)?;

    let mut headermap = HeaderMap::new();
    let mut i = 1;
    for line in lines.iter().skip(1) {
        if line.is_empty() {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            let name = HeaderName::from_str(name)?;
            let value = HeaderValue::from_str(value)?;

            headermap.insert(name, value);
        }

        i += 1;
    }

    let mut body = Vec::new();

    for line in &lines[i..] {
        body.extend_from_slice(line.as_bytes());
        body.push(b'\n');
    }

    *req.method_mut() = method;
    *req.uri_mut() = uri;
    *req.headers_mut() = headermap;
    *req.body_mut() = body;

    Ok(())
}

fn extract_status(uri: &Uri, lines: &str) -> srv::Result<(hyper::Method, hyper::Uri)> {
    let Some((method, path)) = lines.split_once(' ') else {
        return Err(srv::Error::InterceptMalformed);
    };

    let method = hyper::Method::from_str(method)?;
    let mut builder = Uri::builder();

    if let Some(scheme) = uri.scheme() {
        builder = builder.scheme(scheme.clone())
    }

    if let Some(auth) = uri.authority() {
        builder = builder.authority(auth.clone())
    }

    let uri = builder.path_and_query(path).build()?;

    log::debug!("modiified {method:?} {uri:?}");

    Ok((method, uri))
}

pub fn imprint_lines_resp(
    resp: &mut hyper::Response<Vec<u8>>,
    lines: Vec<String>,
) -> srv::Result<()> {
    let Some(status) = lines.get(0) else {
        return Err(srv::Error::InterceptMalformed);
    };

    let code = StatusCode::from_str(status)?;

    let mut headermap = HeaderMap::new();
    let mut i = 1;
    for line in lines.iter().skip(1) {
        if line.is_empty() {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            let name = HeaderName::from_str(name)?;
            let value = HeaderValue::from_str(value)?;

            headermap.insert(name, value);
        }

        i += 1;
    }

    let mut body = Vec::new();

    for line in &lines[i..] {
        body.extend_from_slice(line.as_bytes());
        body.push(b'\n');
    }

    *resp.status_mut() = code;
    *resp.headers_mut() = headermap;
    *resp.body_mut() = body;

    Ok(())
}
