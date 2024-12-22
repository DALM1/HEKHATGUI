#[cfg(feature = "client")]
mod client;

#[tokio::main]
async fn main() {

    #[cfg(feature = "client")]
    client::run_client().await;
}
