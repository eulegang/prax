use cucumber::{cli, gherkin::Step, given, then, when, World};
use http::{HeaderName, HeaderValue, StatusCode};

use prax::{proxy::Config, Filter};

#[derive(Debug, Default, World)]
pub struct ResWorld {
    subject: prax::Res<Vec<u8>>,
}

#[given(expr = "the status is {}")]
fn given_status(world: &mut ResWorld, status: u16) {
    *world.subject.status_mut() = StatusCode::from_u16(status).unwrap();
}

#[given(expr = "a header {} is {}")]
fn given_header(world: &mut ResWorld, name: String, value: String) {
    let name = HeaderName::try_from(name).unwrap();
    let value = HeaderValue::try_from(value).unwrap();

    world.subject.headers_mut().insert(name, value);
}

#[given(expr = "the body is {}")]
fn given_body(world: &mut ResWorld, body: String) {
    *world.subject.body_mut() = body.as_bytes().to_vec();
}

#[when(expr = "handled by")]
async fn handled(world: &mut ResWorld, step: &Step) {
    let doc = step.docstring().expect("nonempty handler program");
    let config_text = String::leak(doc.clone());
    let config = Config::test(config_text, ()).await.unwrap();
    let mut host = String::from("example.com:3000");

    let _ = config
        .modify_response(&mut host, &mut world.subject)
        .await
        .unwrap();
}

#[when(expr = "filtered {}")]
async fn filter(world: &mut ResWorld, config: String) {
    let mut pre = r#"target("example.com:3000"):"#.to_string();
    pre.push_str(&config);
    let config_text: &'static str = String::leak(pre); // we don't care about leaks in tests
    let config = Config::test(&config_text, ()).await.unwrap();
    let mut host = String::from("example.com:3000");

    let _ = config
        .modify_response(&mut host, &mut world.subject)
        .await
        .unwrap();
}

#[then(expr = "the status is {}")]
fn then_status(world: &mut ResWorld, status: u16) {
    assert_eq!(world.subject.status().as_u16(), status);
}

#[then(expr = "header {} is {}")]
fn then_header(world: &mut ResWorld, name: String, value: String) {
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

#[then(expr = "body is {}")]
fn then_body(world: &mut ResWorld, body: String) {
    assert_eq!(world.subject.body(), body.as_bytes())
}

#[derive(cli::Args)]
struct CustomOpts {
    #[arg(long)]
    test_threads: Option<usize>,
}

fn main() {
    let opts = cli::Opts::<_, _, _, CustomOpts>::parsed();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(opts.custom.test_threads.unwrap_or(1))
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        ResWorld::cucumber()
            .with_cli(opts)
            .run_and_exit("cucumber/responses")
            .await;
    })
}
