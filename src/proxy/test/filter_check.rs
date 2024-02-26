use std::collections::HashSet;

use hyper::{Method, Uri};

use crate::{lines::LinesImprint, proxy::Config, Filter};

pub async fn check_req<F>(config: &Config<F>, input: &str, output: &str)
where
    F: Clone + Filter + Send + Sync + 'static,
{
    let input: Vec<String> = input.split('\n').map(ToString::to_string).collect();
    let output: Vec<String> = output.split('\n').map(ToString::to_string).collect();

    let mut input_req = hyper::Request::new(Vec::new());
    input_req.imprint(input).unwrap();

    if let Err(e) = config
        .modify_request("example.com:3000", &mut input_req)
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

pub async fn check_res<F>(config: &Config<F>, input: &str, output: &str)
where
    F: Clone + Filter + Send + Sync + 'static,
{
    let input: Vec<String> = input.split('\n').map(ToString::to_string).collect();
    let output: Vec<String> = output.split('\n').map(ToString::to_string).collect();

    let mut input_res = hyper::Response::new(Vec::new());
    input_res.imprint(input).unwrap();

    if let Err(e) = config
        .modify_response("example.com:3000", &mut input_res)
        .await
    {
        panic!("failed to process request {:?} {}", input_res, e);
    }

    let mut output_res = hyper::Response::new(Vec::new());
    output_res.imprint(output).unwrap();

    let validations = input_res.validate_with(&output_res);

    if !validations.is_empty() {
        let mut buf = String::new();
        for val in validations {
            buf.push_str(&format!("- {}\n", val));
        }
        panic!("{}\n{:#?}\n\n != \n\n{:#?}", buf, input_res, output_res);
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
    Status {
        actual: hyper::StatusCode,
        expected: hyper::StatusCode,
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
            ValError::Status { actual, expected } => {
                write!(f, "expected status {expected:?} but got {actual:?}")
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
                expected: expectation.method(),
            });
        }

        if self.uri() != expectation.uri() {
            vals.push(ValError::Uri {
                actual: self.uri(),
                expected: expectation.uri(),
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

impl Validate for hyper::Response<Vec<u8>> {
    fn validate_with<'a>(&'a self, expectation: &'a Self) -> Vec<ValError<'a>> {
        let mut vals = vec![];
        if self.status() != expectation.status() {
            vals.push(ValError::Status {
                actual: self.status(),
                expected: expectation.status(),
            });
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
