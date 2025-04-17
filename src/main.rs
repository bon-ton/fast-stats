use fast_stats::start_server;

#[tokio::main]
async fn main() {
    start_server().await.unwrap()
}
