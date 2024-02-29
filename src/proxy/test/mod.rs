use crate::{
    lines::LinesImprint,
    proxy::{test::filter_check::Validate, Config},
    Filter,
};
use trace::Trace;

mod cucumber;
mod filter_check;
mod trace;

mod request {
    mod set {
        mod header {
            use super::super::super::*;
            const CONFIG: &str = r#"target("example.com:3000"):req(set(header("Authentication"), "Bearer foobarxyz"))"#;

            #[tokio::test]
            async fn missing() {
                const IN: &str = "GET /\nhost: example.com:3000\n";
                const OUT: &str =
                    "GET /\nhost: example.com:3000\nauthentication: Bearer foobarxyz\n";

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn set_override() {
                const IN: &str = "GET /\nhost: example.com:3000\nauthentication: Bearer abc\n";
                const OUT: &str =
                    "GET /\nhost: example.com:3000\nauthentication: Bearer foobarxyz\n";

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }
        }

        mod query {
            use super::super::super::*;

            const CONFIG: &str =
                r#"target("example.com:3000"):req(set(query("q"), "hello-google"))"#;

            #[tokio::test]
            async fn set_without_other() {
                const IN: &str = "GET /\nhost: example.com:3000\n";
                const OUT: &str = "GET /?q=hello-google\nhost: example.com:3000\n";

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn set_with_other() {
                const IN: &str = "GET /?subject=hello\nhost: example.com:3000\n";
                const OUT: &str = "GET /?subject=hello&q=hello-google\nhost: example.com:3000\n";

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }
        }
    }

    mod subst_func {
        mod header {
            use super::super::super::*;

            #[tokio::test]
            async fn missing() {
                const IN: &str = "GET /\nhost: example.com:3000\nuser-agent: curl\n";
                const OUT: &str = "GET /\nhost: example.com:3000\nuser-agent: curl/0.0.0-example\n";
                const CONFIG: &str = r#"
local function add_version(name)
    return name .. '/0.0.0-example'
end

target("example.com:3000")
    :req(sub(header("user-agent"), add_version))
    :resp(sub(header("server"), add_version))
"#;

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn sub_method() {
                const IN: &str = "GET /\nhost: example.com:3000\nuser-agent: curl\n";
                const OUT: &str = "POST /\nhost: example.com:3000\nuser-agent: curl\n";
                const CONFIG: &str =
                    "target(\"example.com:3000\"):req(sub(method, function(s) return \"POST\" end))";

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn sub_systemccmd_method() {
                const IN: &str = "GET /\nhost: example.com:3000\nuser-agent: curl\n";
                const OUT: &str = "GET /\nhost: example.com:3000\nuser-agent: hurl\n";
                const CONFIG: &str =
                    "target(\"example.com:3000\"):req(sub(header(\"user-agent\"), \"tr c h\"))";

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }
        }
    }
}

mod response {
    mod set {
        mod header {
            use super::super::super::*;
            const CONFIG: &str =
                r#"target("example.com:3000"):resp(set(header("server"), "foobar"))"#;

            #[tokio::test]
            async fn missing() {
                const IN: &str = "200\nhost: example.com:3000\n";
                const OUT: &str = "200\nhost: example.com:3000\nserver: foobar\n";

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_res(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn set_override() {
                const IN: &str = "200\nhost: example.com:3000\nserver: nginx\n";
                const OUT: &str = "200\nhost: example.com:3000\nserver: foobar\n";

                let config = Config::test(CONFIG, ()).await.unwrap();

                filter_check::check_res(&config, IN, OUT).await;
            }
        }
    }
}

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
