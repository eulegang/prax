use crate::lines::ToLines;

#[test]
fn test_hyper_req_get() {
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
fn test_hyper_req_filledout() {
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
