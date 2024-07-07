use fantoccini::{Locator};
use rocket::local::asynchronous::Client as RocketClient;
use rocket_api::rocket;
use fantoccini::ClientBuilder;
use fantoccini::Wait;

#[rocket::async_test]
async fn test_certified_donation_process() {
    // Start the Rocket server
    let rocket = rocket().await.unwrap();
    let client = RocketClient::tracked(rocket).await.unwrap();

    // Create a new WebDriver client
    let mut c = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");

    // Navigate to the donation success page
    c.goto("http://localhost:1313/donate/certified/success/?payment_intent=test_payment_intent")
        .await
        .expect("failed to navigate");

    // Wait for the certificate and private key to be generated
    c.wait().for_element(Locator::Css("#certificate"))
        .await
        .expect("failed to find certificate");

    // Verify the certificate and private key are displayed
    let certificate = c.find(Locator::Css("#certificate")).await.expect("failed to find certificate");
    let certificate_value = certificate.text().await.expect("failed to get certificate text");
    assert!(certificate_value.contains("-----BEGIN FREENET DONATION CERTIFICATE-----"));

    let private_key = c.find(Locator::Css("#privateKey")).await.expect("failed to find private key");
    let private_key_value = private_key.text().await.expect("failed to get private key text");
    assert!(private_key_value.contains("-----BEGIN FREENET DONATION PRIVATE KEY-----"));

    // Close the WebDriver client
    c.close().await.expect("failed to close WebDriver client");
}
