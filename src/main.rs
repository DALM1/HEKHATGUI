#[cfg(feature = "server")]
mod server;
#[cfg(feature = "client")]
mod client;

#[tokio::main]
async fn main() {
    #[cfg(feature = "server")]
    server::run_server().await;

    #[cfg(feature = "client")]
    client::run_client().await;
}
