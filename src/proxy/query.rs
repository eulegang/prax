use std::str::FromStr;

use http::uri::{InvalidUri, PathAndQuery};

#[derive(Default)]
pub struct Query {
    buf: String,
}

impl Query {
    pub fn push(&mut self, key: &str, value: Option<&str>) {
        if self.buf.is_empty() {
            self.buf.push('&')
        }

        self.buf.push_str(key);
        if let Some(value) = value {
            self.buf.push('=');
            self.buf.push_str(value);
        }
    }

    pub fn iter(&self) -> QueryIter {
        QueryIter {
            query: self,
            pos: 0,
        }
    }

    pub fn to_path_and_query(&self, path: &str) -> Result<PathAndQuery, InvalidUri> {
        PathAndQuery::from_str(&format!("{}?{}", path, self.buf))
    }
}

pub struct QueryIter<'a> {
    query: &'a Query,
    pos: usize,
}

impl<'a> Iterator for QueryIter<'a> {
    type Item = (&'a str, Option<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == usize::MAX {
            return None;
        }

        let rest = &self.query.buf[self.pos..];
        let sub = if let Some(next) = rest.find('&') {
            self.pos = next;
            &rest[0..next]
        } else {
            self.pos = usize::MAX;
            rest
        };

        if let Some((k, v)) = sub.split_once('=') {
            Some((k, Some(v)))
        } else {
            Some((sub, None))
        }
    }
}

impl From<&str> for Query {
    fn from(value: &str) -> Self {
        Query {
            buf: value.to_string(),
        }
    }
}
