fn main() {
    dotenvy::dotenv().ok();
    if let Ok(url) = std::env::var("LEAGUEEYE_SERVER_URL") {
        println!("cargo:rustc-env=LEAGUEEYE_SERVER_URL={}", url);
    }
    tauri_build::build()
}
