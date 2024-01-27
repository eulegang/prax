mod filter_check;

#[tokio::test]
async fn test_set_header() {
    filter_check::run_check("headers").await;
}

#[tokio::test]
async fn test_set_subst_func() {
    filter_check::run_check("subst_func").await;
}
