#[tokio::test]
async fn test_proxy_rules() {
    let mut dir_list = tokio::fs::read_dir("src/proxy/test").await.unwrap();
    loop {
        let ent = dir_list.next_entry().await.unwrap();
        let Some(ent) = ent else {
            break;
        };

        let metadata = ent.metadata().await.unwrap();
        if metadata.is_dir() {
            todo!("test example case");
        }
    }
}
