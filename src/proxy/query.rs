use std::str::FromStr;

use http::uri::{InvalidUri, PathAndQuery};

#[derive(Default)]
pub struct Query {
    buf: String,
}

impl Query {
    pub fn push(&mut self, key: &str, value: Option<&str>) {
        if !self.buf.is_empty() {
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

impl From<&PathAndQuery> for Query {
    fn from(pq: &PathAndQuery) -> Self {
        pq.query().map(Query::from).unwrap_or_default()
    }
}

pub struct QueryIter<'a> {
    query: &'a Query,
    pos: usize,
}

impl<'a> Iterator for QueryIter<'a> {
    type Item = (&'a str, Option<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.query.buf.len() {
            return None;
        }

        let rest = &self.query.buf[self.pos..];
        let sub = if let Some(next) = rest.find('&') {
            self.pos = next + 1;
            &rest[0..next]
        } else {
            self.pos = self.query.buf.len();
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

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buf)
    }
}

impl std::fmt::Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.buf)
    }
}

#[test]
fn iteration() {
    assert_eq!(Query::from("").iter().collect::<Vec<_>>(), vec![]);
    assert_eq!(
        Query::from("subject=world").iter().collect::<Vec<_>>(),
        vec![("subject", Some("world"))]
    );
    assert_eq!(
        Query::from("subject").iter().collect::<Vec<_>>(),
        vec![("subject", None)]
    );
    assert_eq!(
        Query::from("subject=world&greeting=hello")
            .iter()
            .collect::<Vec<_>>(),
        vec![("subject", Some("world")), ("greeting", Some("hello"))]
    );
}
