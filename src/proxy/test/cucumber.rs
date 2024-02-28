use cucumber::{given, then, when, Parameter, World};
use http::{HeaderName, HeaderValue, Method, StatusCode};
use hyper::{Request, Response};

use crate::{proxy::Config, Filter};

#[derive(Debug, Default, World)]
pub struct GehrkWorld {
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
fn setup_method(world: &mut GehrkWorld, method: Meth) {
    let mut req = Request::new(vec![]);
    *req.method_mut() = method.0;
    world.subject = Subject::Request(req);
}

#[given(expr = "a {} response")]
fn setup_status(world: &mut GehrkWorld, status: u16) {
    let mut res = Response::new(vec![]);
    *res.status_mut() = StatusCode::from_u16(status).unwrap();

    world.subject = Subject::Response(res);
}

#[given(expr = "a header {}: {}")]
fn setup_header(world: &mut GehrkWorld, name: String, value: String) {
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

#[when(expr = "filtered {}")]
async fn filter(world: &mut GehrkWorld, config: String) {
    let config: &'static str = String::leak(config); // we don't care about leaks in tests
    let config = Config::test(&config, ()).await.unwrap();

    match &mut world.subject {
        Subject::Init => panic!("uninitied"),
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
fn method_then(world: &mut GehrkWorld, method: Meth) {
    match &world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(req) => {
            assert_eq!(req.method(), method.0)
        }
        Subject::Response(_) => todo!(),
    }
}

#[then(expr = "a header {}: {}")]
fn header_then(world: &mut GehrkWorld, name: String, value: String) {
    let name = HeaderName::try_from(name).unwrap();
    let value = HeaderValue::try_from(value).unwrap();

    let map = match &world.subject {
        Subject::Init => panic!("uninited"),
        Subject::Request(req) => req.headers(),
        Subject::Response(res) => res.headers(),
    };

    for (n, v) in map {
        if dbg!(n == name) && dbg!(v == value) {
            return;
        }
    }

    panic!("failed to find header {name:?}: {value:?} in {map:?}")
}

#[tokio::test]
async fn cucumber() {
    GehrkWorld::run("tests/features").await
}
