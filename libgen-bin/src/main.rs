pub mod libgen_cli;

#[tokio::main]
async fn main() {
    libgen_cli::init().await.unwrap();
}
