fn main() {
    // The app embeds the built front end (`ui/dist`, made by `npm --prefix ui run build`, which
    // `cargo tauri dev` and `cargo tauri build` run first). Plain `cargo build` and `cargo test` need
    // the folder to exist, so leave a placeholder page when the front end has not been built.
    let dist = std::path::Path::new("../ui/dist");
    if !dist.join("index.html").exists() {
        let _ = std::fs::create_dir_all(dist);
        let _ = std::fs::write(
            dist.join("index.html"),
            "<!doctype html><meta charset=\"utf-8\"><title>pampas</title>\
             <p>The interface has not been built. Run <code>npm --prefix ui install</code> and \
             <code>npm --prefix ui run build</code>, or start the app with <code>./pampas --gui</code>.</p>",
        );
    }
    tauri_build::build()
}
