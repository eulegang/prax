use std::str::FromStr;

use cucumber::{cli, given, then, when, Parameter, World};
use http::{uri::PathAndQuery, HeaderName, HeaderValue, Method};

use prax::{
    proxy::{Config, Query},
    Filter,
};

#[derive(Debug, Default, World)]
pub struct ReqWorld {
    subject: prax::Req<Vec<u8>>,
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
fn given_method(world: &mut ReqWorld, method: Meth) {
    *world.subject.method_mut() = method.0;
}

#[given(expr = "a header {} is {}")]
fn given_header(world: &mut ReqWorld, name: String, value: String) {
    let name = HeaderName::try_from(name).unwrap();
    let value = HeaderValue::try_from(value).unwrap();

    world.subject.headers_mut().insert(name, value);
}

#[given(expr = "a query {} is {}")]
fn given_query(world: &mut ReqWorld, name: String, value: String) {
    let uri = world.subject.uri().clone();
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

    *world.subject.uri_mut() = hyper::Uri::from_parts(parts).unwrap();
}

#[given(expr = "a path {}")]
fn given_path(world: &mut ReqWorld, path: String) {
    let uri = world.subject.uri().clone();
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

    *world.subject.uri_mut() = hyper::Uri::from_parts(parts).unwrap();
}

#[given(expr = "a body {}")]
fn given_body(world: &mut ReqWorld, body: String) {
    *world.subject.body_mut() = body.as_bytes().to_vec();
}

#[when(expr = "filtered {}")]
async fn filter(world: &mut ReqWorld, config: String) {
    let mut pre = r#"target("example.com:3000"):"#.to_string();
    pre.push_str(&config);
    let config: &'static str = String::leak(pre); // we don't care about leaks in tests
    let config = Config::test(&config, ()).await.unwrap();

    let _ = config
        .modify_request("example.com:3000", &mut world.subject)
        .await
        .unwrap();
}

#[then(expr = "method is {meth}")]
fn then_method(world: &mut ReqWorld, method: Meth) {
    assert_eq!(world.subject.method(), method.0)
}

#[then(expr = "query {} is {}")]
fn then_query(world: &mut ReqWorld, name: String, value: String) {
    let query = world
        .subject
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

#[then(expr = "path is {}")]
fn then_path(world: &mut ReqWorld, path: String) {
    assert_eq!(world.subject.uri().path(), path)
}

#[then(expr = "body is {}")]
fn then_body(world: &mut ReqWorld, body: String) {
    assert_eq!(world.subject.body(), body.as_bytes())
}

#[then(expr = "header {} is {}")]
fn then_header(world: &mut ReqWorld, name: String, value: String) {
    let name = HeaderName::try_from(name).unwrap();
    let value = HeaderValue::try_from(value).unwrap();

    let map = world.subject.headers();

    for (n, v) in map {
        if n == name && v == value {
            return;
        }
    }

    panic!("failed to find header {name:?}: {value:?} in {map:?}")
}

#[derive(cli::Args)] // re-export of `clap::Args`
struct CustomOpts {
    #[arg(long)]
    test_threads: Option<usize>,
}

fn main() {
    println!("args found {:?}", std::env::args());

    let opts = cli::Opts::<_, _, _, CustomOpts>::parsed();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(opts.custom.test_threads.unwrap_or(1))
        .build()
        .unwrap();

    rt.block_on(async {
        ReqWorld::cucumber()
            .with_cli(opts)
            .run_and_exit("cucumber/requests")
            .await;
    })
}
