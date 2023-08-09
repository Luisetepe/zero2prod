use crate::test_utils::TestContext;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let test_app = TestContext::create_stub_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", &test_app.server_address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());

    // Cleanup
    test_app.cleanup().await;
}
