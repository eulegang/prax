use crate::proxy::Config;
use std::path::PathBuf;

mod filter_check;

mod request {
    mod set {
        mod header {
            use super::super::super::*;

            #[tokio::test]
            async fn missing() {
                const IN: &str = "GET /\nhost: example.com:3000\n";
                const OUT: &str =
                    "GET /\nhost: example.com:3000\nauthentication: Bearer foobarxyz\n";

                let from = PathBuf::from("src/proxy/test/headers.lua");
                let config = Config::load(&from, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn set_override() {
                const IN: &str = "GET /\nhost: example.com:3000\nauthentication: Bearer abc\n";
                const OUT: &str =
                    "GET /\nhost: example.com:3000\nauthentication: Bearer foobarxyz\n";

                let from = PathBuf::from("src/proxy/test/headers.lua");
                let config = Config::load(&from, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn set_query_without_other() {
                const IN: &str = "GET /\nhost: example.com:3000\n";
                const OUT: &str = "GET /?q=hello-google\nhost: example.com:3000\n";

                let from = PathBuf::from("src/proxy/test/query.lua");
                let config = Config::load(&from, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn set_query_with_other() {
                const IN: &str = "GET /?subject=hello\nhost: example.com:3000\n";
                const OUT: &str = "GET /?subject=hello&q=hello-google\nhost: example.com:3000\n";

                let from = PathBuf::from("src/proxy/test/query.lua");
                let config = Config::load(&from, ()).await.unwrap();

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

                let from = PathBuf::from("src/proxy/test/subst_func.lua");
                let config = Config::load(&from, ()).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn sub_method() {
                const IN: &str = "GET /\nhost: example.com:3000\nuser-agent: curl\n";
                const OUT: &str = "POST /\nhost: example.com:3000\nuser-agent: curl\n";
                const CONFIG: &str =
                    "target(\"example.com:3000\"):req(sub(method, function(s) return \"POST\" end))";

                let config = Config::test(CONFIG).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn sub_systemccmd_method() {
                const IN: &str = "GET /\nhost: example.com:3000\nuser-agent: curl\n";
                const OUT: &str = "GET /\nhost: example.com:3000\nuser-agent: hurl\n";
                const CONFIG: &str =
                    "target(\"example.com:3000\"):req(sub(header(\"user-agent\"), \"tr c h\"))";

                let config = Config::test(CONFIG).await.unwrap();

                filter_check::check_req(&config, IN, OUT).await;
            }
        }
    }
}

mod response {
    mod set {
        mod header {
            use super::super::super::*;

            #[tokio::test]
            async fn missing() {
                const IN: &str = "200\nhost: example.com:3000\n";
                const OUT: &str = "200\nhost: example.com:3000\nserver: foobar\n";

                let from = PathBuf::from("src/proxy/test/headers.lua");
                let config = Config::load(&from, ()).await.unwrap();

                filter_check::check_res(&config, IN, OUT).await;
            }

            #[tokio::test]
            async fn set_override() {
                const IN: &str = "200\nhost: example.com:3000\nserver: nginx\n";
                const OUT: &str = "200\nhost: example.com:3000\nserver: foobar\n";

                let from = PathBuf::from("src/proxy/test/headers.lua");
                let config = Config::load(&from, ()).await.unwrap();

                filter_check::check_res(&config, IN, OUT).await;
            }
        }
    }
}
