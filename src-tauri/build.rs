fn main() {
    // Carrega variáveis do arquivo .env (da raiz ou da pasta src-tauri) e passa para o rustc
    let entries = dotenvy::from_filename_iter("../.env")
        .or_else(|_| dotenvy::dotenv_iter());

    if let Ok(iter) = entries {
        for item in iter {
            if let Ok((key, val)) = item {
                println!("cargo:rustc-env={}={}", key, val);
            }
        }
    }

    println!("cargo:rerun-if-changed=../.env");
    println!("cargo:rerun-if-changed=.env");

    tauri_build::build()
}
