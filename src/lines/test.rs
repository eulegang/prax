mod hyper_req_lines {
    use crate::lines::ToLines;

    #[test]
    fn get() {
        let bytes = b"hello\nworld\n".to_vec();
        let req = hyper::Request::new(bytes);
        assert_eq!(
            req.to_lines().unwrap(),
            vec![
                "GET /".to_string(),
                "".to_string(),
                "hello".to_string(),
                "world".to_string()
            ]
        );
    }

    #[test]
    fn filled_out() {
        let bytes = b"hallo\nwelt\n".to_vec();

        let req = hyper::Request::builder()
            .method("POST")
            .uri("/foobar?baz=true")
            .header("authorization", "Bearer xyz")
            .body(bytes)
            .unwrap();

        assert_eq!(
            req.to_lines().unwrap(),
            vec![
                "POST /foobar?baz=true".to_string(),
                "authorization: Bearer xyz".to_string(),
                "".to_string(),
                "hallo".to_string(),
                "welt".to_string()
            ]
        );
    }
}

mod hyper_res {
    use crate::lines::ToLines;

    #[test]
    fn ok() {
        let bytes = b"hello\nworld\n".to_vec();
        let req = hyper::Response::new(bytes);

        assert_eq!(
            req.to_lines().unwrap(),
            vec![
                "200".to_string(),
                "".to_string(),
                "hello".to_string(),
                "world".to_string()
            ]
        );
    }

    #[test]
    fn filled_out() {
        let bytes = b"hello\nworld\n".to_vec();
        let req = hyper::Response::builder()
            .status(404)
            .header("server", "nginx")
            .body(bytes)
            .unwrap();

        assert_eq!(
            req.to_lines().unwrap(),
            vec![
                "404".to_string(),
                "server: nginx".to_string(),
                "".to_string(),
                "hello".to_string(),
                "world".to_string()
            ]
        );
    }
}

mod hist_req {
    use std::collections::HashMap;

    use crate::hist::Request;
    use crate::lines::ToLines;

    #[test]
    fn get() {
        let req = Request {
            method: "GET".to_string(),
            path: "/".to_string(),
            query: HashMap::new(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            body: b"hello\nworld".to_vec().into(),
        };

        assert_eq!(
            req.to_lines().unwrap(),
            vec![
                "GET /".to_string(),
                "".to_string(),
                "hello".to_string(),
                "world".to_string()
            ]
        );
    }

    #[test]
    fn filled_out() {
        let mut query = HashMap::new();
        query.insert("baz".to_string(), "true".to_string());

        let mut headers = HashMap::new();
        headers.insert("user-agent".to_string(), "curl".to_string());

        let req = Request {
            method: "POST".to_string(),
            path: "/foobar".to_string(),
            query,
            version: "HTTP/1.1".to_string(),
            headers,
            body: b"hello\nworld".to_vec().into(),
        };

        assert_eq!(
            req.to_lines().unwrap(),
            vec![
                "POST /foobar?baz=true".to_string(),
                "user-agent: curl".to_string(),
                "".to_string(),
                "hello".to_string(),
                "world".to_string()
            ]
        );
    }
}

mod hist_res {
    use std::collections::HashMap;

    use crate::hist::Response;
    use crate::lines::ToLines;

    #[test]
    fn get() {
        let req = Response {
            status: 200,
            headers: HashMap::new(),
            body: b"hello\nworld".to_vec().into(),
        };

        assert_eq!(
            req.to_lines().unwrap(),
            vec![
                "200".to_string(),
                "".to_string(),
                "hello".to_string(),
                "world".to_string()
            ]
        );
    }

    #[test]
    fn filled_out() {
        let mut headers = HashMap::new();
        headers.insert("server".to_string(), "nginx".to_string());

        let req = Response {
            status: 200,
            headers,
            body: b"hello\nworld".to_vec().into(),
        };

        assert_eq!(
            req.to_lines().unwrap(),
            vec![
                "200".to_string(),
                "server: nginx".to_string(),
                "".to_string(),
                "hello".to_string(),
                "world".to_string()
            ]
        );
    }
}

mod hyper_req_imprint {
    use hyper::Method;

    use crate::lines::LinesImprint;

    #[test]
    fn get() {
        let lines = vec![
            "POST /foobar?xyz=abc".to_string(),
            "user-agent: curl".to_string(),
            "".to_string(),
            "{\"foobar\": true}".to_string(),
        ];

        let bytes = b"hello\nworld\n".to_vec();
        let mut req = hyper::Request::new(bytes);

        req.imprint(lines).unwrap();

        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.uri().query(), Some("xyz=abc"));
        assert_eq!(req.uri().path(), "/foobar");

        assert_eq!(req.headers().len(), 1);
        assert_eq!(
            req.headers().get("user-agent"),
            Some(hyper::header::HeaderValue::from_static("curl")).as_ref()
        );
    }
}

mod hyper_res_imprint {
    use hyper::StatusCode;

    use crate::lines::LinesImprint;

    #[test]
    fn get() {
        let lines = vec![
            "404".to_string(),
            "server: nginx".to_string(),
            "".to_string(),
            "{\"foobar\": true}".to_string(),
        ];

        let bytes = b"hello\nworld\n".to_vec();
        let mut res = hyper::Response::new(bytes);

        res.imprint(lines).unwrap();

        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        assert_eq!(res.headers().len(), 1);
        assert_eq!(
            res.headers().get("server"),
            Some(hyper::header::HeaderValue::from_static("nginx")).as_ref()
        );
    }
}
