use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use hyper::{Method, Uri};

use crate::{lines::LinesImprint, proxy::Config, Filter};

pub async fn run_check(module: &str) {
    let base = PathBuf::from(format!("src/proxy/test/{}/", module));

    let mut config = base.clone();
    config.push("config.lua");

    let config = Config::load(&config, ()).unwrap();

    for req in find_requests(&base).await {
        let name = extract_name(&req);

        let input = lines(&req).await;
        let mut input_req = hyper::Request::new(Vec::new());
        input_req.imprint(input).unwrap();

        if let Err(e) = config
            .modify_request("example.com:3000", &mut input_req)
            .await
        {
            panic!(
                "failed to process request {} {:?} {}",
                req.display(),
                input_req,
                e
            );
        }

        let expectation = expectation(&req);
        let output = lines(&expectation).await;
        let mut output_req = hyper::Request::new(Vec::new());
        output_req.imprint(output).unwrap();

        let validations = input_req.validate_with(&output_req);

        if !validations.is_empty() {
            let mut buf = String::new();
            for val in validations {
                buf.push_str(&format!("- {}\n", val));
            }
            panic!(
                "{name}\n{}\n{:#?}\n\n != \n\n{:#?}",
                buf, input_req, output_req
            );
        }
    }
}

pub enum ValError<'a> {
    Method {
        actual: &'a Method,
        expected: &'a Method,
    },

    Uri {
        actual: &'a Uri,
        expected: &'a Uri,
    },

    MissingHeader(String),
    ExtraHeader(String),

    HeaderMismatch {
        key: String,
        actual: String,
        expected: String,
    },

    Body {
        actual: &'a [u8],
        expected: &'a [u8],
    },
}

impl<'a> std::fmt::Display for ValError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValError::Method { actual, expected } => {
                write!(f, "expected method {expected:?} but got {actual:?}")
            }
            ValError::Uri { actual, expected } => {
                write!(f, "expected uri {expected:?} but got {actual:?}")
            }
            ValError::MissingHeader(key) => write!(f, "missing header \"{key}\""),
            ValError::ExtraHeader(key) => write!(f, "extra header \"{key}\""),
            ValError::HeaderMismatch {
                key,
                actual,
                expected,
            } => write!(
                f,
                "header mismatch \"{key}\" expected \"{expected}\" but found \"{actual}\""
            ),
            ValError::Body { actual, expected } => {
                write!(f, "expected body {expected:?} but got {actual:?}")
            }
        }
    }
}

pub trait Validate {
    fn validate_with<'a>(&'a self, expectation: &'a Self) -> Vec<ValError<'a>>;
}

impl Validate for hyper::Request<Vec<u8>> {
    fn validate_with<'a>(&'a self, expectation: &'a Self) -> Vec<ValError<'a>> {
        let mut vals = vec![];
        if self.method() != expectation.method() {
            vals.push(ValError::Method {
                actual: self.method(),
                expected: self.method(),
            });
        }

        if self.uri() != expectation.uri() {
            vals.push(ValError::Uri {
                actual: self.uri(),
                expected: self.uri(),
            })
        }

        let actual_keys: HashSet<String> = self.headers().keys().map(ToString::to_string).collect();
        let expect_keys: HashSet<String> = expectation
            .headers()
            .keys()
            .map(ToString::to_string)
            .collect();

        for key in expect_keys.difference(&actual_keys) {
            vals.push(ValError::MissingHeader(key.to_string()));
        }

        for key in actual_keys.difference(&expect_keys) {
            vals.push(ValError::ExtraHeader(key.to_string()));
        }

        for key in actual_keys.intersection(&expect_keys) {
            let actual = &self.headers()[key];
            let expected = &expectation.headers()[key];

            if actual != expected {
                vals.push(ValError::HeaderMismatch {
                    key: key.to_string(),
                    actual: actual.to_str().unwrap().to_string(),
                    expected: expected.to_str().unwrap().to_string(),
                });
            }
        }

        if self.body() != expectation.body() {
            vals.push(ValError::Body {
                actual: self.body(),
                expected: expectation.body(),
            });
        }

        vals
    }
}

async fn find_requests(base: &Path) -> Vec<PathBuf> {
    let mut dir = tokio::fs::read_dir(base).await.unwrap();
    let mut res = Vec::new();

    loop {
        let ent = dir.next_entry().await.unwrap();
        let Some(ent) = ent else {
            break;
        };

        let path = ent.path();
        let Some(os_str) = path.file_name() else {
            continue;
        };

        let Some(name) = os_str.to_str() else {
            continue;
        };

        if name.ends_with(".in.req") {
            res.push(ent.path());
        }
    }

    res
}

async fn lines(path: &Path) -> Vec<String> {
    let Ok(content) = tokio::fs::read(path).await else {
        panic!("failed to read file {}", path.display());
    };

    let mut res = Vec::new();
    for line in content.split(|b| *b == b'\n') {
        let Ok(line) = std::str::from_utf8(line) else {
            panic!("binary content detected in {}", path.display());
        };

        res.push(line.to_string());
    }

    res
}

fn expectation(path: &Path) -> PathBuf {
    let mut out = path.to_path_buf();
    let filename = out.file_name().unwrap().to_str().unwrap().to_string();
    out.pop();

    out.push(filename.replace(".in.", ".out."));

    out
}

fn extract_name(path: &Path) -> String {
    let Some(os) = path.file_name() else { panic!() };
    let Some(s) = os.to_str() else { panic!() };
    s.replace(".in.req", "").replace(".in.res", "")
}
