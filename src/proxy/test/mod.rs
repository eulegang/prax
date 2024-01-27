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

#[tokio::test]
async fn test_set_subst_func() {
    filter_check::run_check("subst_func").await;
}
