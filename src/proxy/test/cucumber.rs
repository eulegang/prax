use std::str::FromStr;

use cucumber::{given, then, when, Parameter, World};
use http::{uri::PathAndQuery, HeaderName, HeaderValue, Method, StatusCode};
use hyper::{Request, Response};

use crate::{
    proxy::{query::Query, Config},
    Filter,
};

#[derive(Debug, Default, World)]
pub struct HttpWorld {
    subject: Subject,
}

#[derive(Default, Debug)]
enum Subject {
    #[default]
    Init,

    Request(crate::Req<Vec<u8>>),
    Response(crate::Res<Vec<u8>>),
}

#[derive(Debug, Default, Eq, Parameter, PartialEq)]
#[param(regex = ".*")]
struct Meth(Method);

impl std::str::FromStr for Meth {
    type Err = <Method as std::str::FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Meth(Method::from_str(s)?))
    }
}

#[given(expr = "a {meth} request")]
fn given_method(world: &mut HttpWorld, method: Meth) {
    let mut req = Request::new(vec![]);
    *req.method_mut() = method.0;
    world.subject = Subject::Request(req);
}

#[given(expr = "a {} response")]
fn given_status(world: &mut HttpWorld, status: u16) {
    let mut res = Response::new(vec![]);
    *res.status_mut() = StatusCode::from_u16(status).unwrap();

    world.subject = Subject::Response(res);
}

#[given(expr = "a header {} is {}")]
fn given_header(world: &mut HttpWorld, name: String, value: String) {
    let name = HeaderName::try_from(name).unwrap();
    let value = HeaderValue::try_from(value).unwrap();

    match &mut world.subject {
        Subject::Init => todo!(),
        Subject::Request(request) => {
            request.headers_mut().insert(name, value);
        }
        Subject::Response(response) => {
            response.headers_mut().insert(name, value);
        }
    }
}

#[given(expr = "a query {} is {}")]
fn given_query(world: &mut HttpWorld, name: String, value: String) {
    match &mut world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(request) => {
            let uri = request.uri_mut().clone();
            let mut parts = uri.into_parts();
            if let Some(pq) = &mut parts.path_and_query {
                let mut q = pq.query().map(Query::from).unwrap_or_default();
                q.push(&name, Some(&value));

                *pq = q.to_path_and_query(pq.path()).unwrap();
            } else {
                let mut q = Query::default();
                q.push(&name, Some(&value));
                parts.path_and_query = Some(q.to_path_and_query("").unwrap())
            }

            *request.uri_mut() = hyper::Uri::from_parts(parts).unwrap();
        }
        Subject::Response(_) => {
            panic!("can't modify the query of a response")
        }
    }
}

#[given(expr = "a path {}")]
fn given_path(world: &mut HttpWorld, path: String) {
    match &mut world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(request) => {
            let uri = request.uri_mut().clone();
            let mut parts = uri.into_parts();
            if let Some(pq) = &mut parts.path_and_query {
                if let Some(q) = pq.query() {
                    *pq = PathAndQuery::from_str(&format!("{}?{}", path, q)).unwrap();
                } else {
                    *pq = PathAndQuery::from_str(&path).unwrap();
                }
            } else {
                parts.path_and_query = Some(PathAndQuery::from_str(&path).unwrap());
            }

            *request.uri_mut() = hyper::Uri::from_parts(parts).unwrap();
        }
        Subject::Response(_) => {
            panic!("can't modify the path of a response")
        }
    }
}

#[given(expr = "a body {}")]
fn given_body(world: &mut HttpWorld, body: String) {
    match &mut world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(request) => *request.body_mut() = body.as_bytes().to_vec(),
        Subject::Response(response) => *response.body_mut() = body.as_bytes().to_vec(),
    }
}

#[when(expr = "filtered {}")]
async fn filter(world: &mut HttpWorld, config: String) {
    let mut pre = r#"target("example.com:3000"):"#.to_string();
    pre.push_str(&config);
    let config: &'static str = String::leak(pre); // we don't care about leaks in tests
    let config = Config::test(&config, ()).await.unwrap();

    match &mut world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(req) => {
            let _ = config
                .modify_request("example.com:3000", req)
                .await
                .unwrap();
        }
        Subject::Response(res) => {
            let _ = config
                .modify_response("example.com:3000", res)
                .await
                .unwrap();
        }
    }
}

#[then(expr = "method is {meth}")]
fn then_method(world: &mut HttpWorld, method: Meth) {
    match &world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(req) => {
            assert_eq!(req.method(), method.0)
        }
        Subject::Response(_) => todo!(),
    }
}

#[then(expr = "status is {}")]
fn then_status(world: &mut HttpWorld, status: u16) {
    match &mut world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(_) => panic!("can't assert status of request"),
        Subject::Response(response) => {
            assert_eq!(response.status().as_u16(), status);
        }
    }
}

#[then(expr = "query {} is {}")]
fn then_query(world: &mut HttpWorld, name: String, value: String) {
    match &mut world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Response(_) => panic!("can not assert the query of a response"),
        Subject::Request(request) => {
            let query = request
                .uri()
                .path_and_query()
                .and_then(|pq| pq.query())
                .map(Query::from)
                .unwrap_or_default();

            for (k, v) in query.iter() {
                let Some(v) = v else {
                    continue;
                };

                if k == name && v == value {
                    return;
                }
            }

            panic!("could not find {name}={value} in {query}")
        }
    }
}

#[then(expr = "path is {}")]
fn then_path(world: &mut HttpWorld, path: String) {
    match &world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Response(_) => panic!("can not assert the path of a response"),
        Subject::Request(request) => {
            assert_eq!(request.uri().path(), path)
        }
    }
}

#[then(expr = "body is {}")]
fn then_body(world: &mut HttpWorld, body: String) {
    match &world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(request) => {
            assert_eq!(request.body(), body.as_bytes())
        }
        Subject::Response(response) => {
            assert_eq!(response.body(), body.as_bytes())
        }
    }
}

#[then(expr = "header {} is {}")]
fn then_header(world: &mut HttpWorld, name: String, value: String) {
    let name = HeaderName::try_from(name).unwrap();
    let value = HeaderValue::try_from(value).unwrap();

    let map = match &world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(req) => req.headers(),
        Subject::Response(res) => res.headers(),
    };

    for (n, v) in map {
        if n == name && v == value {
            return;
        }
    }

    panic!("failed to find header {name:?}: {value:?} in {map:?}")
}

#[tokio::test]
async fn cucumber() {
    HttpWorld::run("tests/features").await
}
