use crate::{
    lines::LinesImprint,
    proxy::{test::filter_check::Validate, Config},
    Filter,
};
use trace::Trace;

mod filter_check;
mod trace;

mod intercept {
    mod negative {
        const CONFIG: &str = r#"target("example.com:3000")"#;

        use super::super::*;

        #[tokio::test]
        async fn request() {
            const IN: &str = "GET /?subject=hello\nhost: example.com:3000\n";
            const OUT: &str = "GET /?subject=hello\nhost: example.com:3000\n";

            let trace = Trace::default();
            let config = Config::test(CONFIG, trace).await.unwrap();

            filter_check::check_req(&config, IN, OUT).await;

            assert!(config.intercept.requests.get(0).is_none());
            assert!(config.intercept.responses.get(0).is_none());
        }

        #[tokio::test]
        async fn response() {
            const IN: &str = "200\nhost: example.com:3000\nserver: nginx\n";
            const OUT: &str = "200\nhost: example.com:3000\nserver: nginx\n";

            let trace = Trace::default();
            let config = Config::test(CONFIG, trace).await.unwrap();

            filter_check::check_res(&config, IN, OUT).await;

            assert!(config.intercept.requests.get(0).is_none());
            assert!(config.intercept.responses.get(0).is_none());
        }
    }

    mod positive {
        use super::super::*;

        const CONFIG: &str = r#"target("example.com:3000"):req(intercept):resp(intercept)"#;

        #[tokio::test]
        async fn request() {
            const IN: &str = "GET /?subject=hello\nhost: example.com:3000\n";
            const OUT: &str = "GET /?subject=hello\nhost: example.com:3000\n";

            let trace = Trace::default();
            let config = Config::test(CONFIG, trace).await.unwrap();

            filter_check::check_req(&config, IN, OUT).await;

            assert!(config.intercept.requests.get(0).is_some());
            assert!(config.intercept.requests.get(1).is_none());
            assert!(config.intercept.responses.get(0).is_none());
        }

        #[tokio::test]
        async fn response() {
            const IN: &str = "200\nhost: example.com:3000\nserver: nginx\n";
            const OUT: &str = "200\nhost: example.com:3000\nserver: nginx\n";

            let trace = Trace::default();
            let config = Config::test(CONFIG, trace).await.unwrap();

            filter_check::check_res(&config, IN, OUT).await;

            assert!(config.intercept.requests.get(0).is_none());
            assert!(config.intercept.responses.get(0).is_some());
            assert!(config.intercept.responses.get(1).is_none());
        }
    }
}

#[tokio::test]
async fn no_hostname() {
    const IN: &str = "GET /\n";
    const OUT: &str = "GET /\n";
    const CONFIG: &str = r#"
target("example.com:3000")
    :req(set(header("Authentication"), "Bearer foobarxyz"))
    :resp(set(header("server"), "foobar"))"#;

    let config = Config::test(CONFIG, ()).await.unwrap();
    let input: Vec<String> = IN.split('\n').map(ToString::to_string).collect();
    let output: Vec<String> = OUT.split('\n').map(ToString::to_string).collect();

    let mut input_req = hyper::Request::new(Vec::new());
    input_req.imprint(input).unwrap();

    if let Err(e) = config
        .modify_request("google.com:3000", &mut input_req)
        .await
    {
        panic!("failed to process request {:?} {}", input_req, e);
    }

    let mut output_req = hyper::Request::new(Vec::new());
    output_req.imprint(output).unwrap();

    let validations = input_req.validate_with(&output_req);

    if !validations.is_empty() {
        let mut buf = String::new();
        for val in validations {
            buf.push_str(&format!("- {}\n", val));
        }
        panic!("{}\n{:#?}\n\n != \n\n{:#?}", buf, input_req, output_req);
    }
}
