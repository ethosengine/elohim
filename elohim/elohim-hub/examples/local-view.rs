//! Standalone harness for the same node composition entry point.
#[tokio::main]
async fn main() {
    let argument = std::env::args().nth(1).expect("config path or --contract");
    if argument == "--contract" {
        print!("{}", elohim_hub::local_views::typescript_contract());
        return;
    }
    if let Err(error) = elohim_hub::local_views::serve(std::path::Path::new(&argument)).await {
        eprintln!("local view refused: {error}");
        std::process::exit(2);
    }
}
