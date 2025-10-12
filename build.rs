use std::fs;
use std::path::Path;
use globwalk::GlobWalkerBuilder;
use minifier::css::minify as minify_css;
use minifier::js::minify as minify_js;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("assets.rs");

    let mut embedded_assets = String::new();

    // JS and CSS files
    let walker = GlobWalkerBuilder::from_patterns(
        "assets",
        &["**/*.js", "**/*.css", "**/*.png", "**/*.jpg", "**/*.jpeg", "**/*.svg", "**/*.ico"]
    )
    .build()
    .unwrap();

    for entry in walker {
        let entry = entry.unwrap();
        let path = entry.path();
        let extension = path.extension().unwrap().to_str().unwrap();

        let var_name = path
            .to_str().unwrap()
            .replace("/", "_")
            .replace(".", "_")
            .replace("@", "_")
            .replace("-", "_")
            .to_uppercase();

        match extension {
            "js" => {
                let content = fs::read_to_string(&path).expect("Failed to read JS file");
                let minified = if path.file_stem().unwrap().to_str().unwrap().contains(".min") {
                    content
                } else {
                    minify_js(&content).to_string()
                };

                embedded_assets.push_str(&format!(
                    "pub const {}: &str = r##########\"{}\"##########;\n",
                    var_name,
                    minified
                ));
            },
            "css" => {
                let content = fs::read_to_string(&path).expect("Failed to read CSS file");
                let minified = if path.file_stem().unwrap().to_str().unwrap().contains(".min") {
                    content
                } else {
                    minify_css(&content).expect("Failed to minify CSS").to_string()
                };
                
                embedded_assets.push_str(&format!(
                    "pub const {}: &str = r##########\"{}\"##########;\n",
                    var_name,
                    minified
                ));
            },
            "png" | "jpg" | "jpeg" | "svg" | "ico" => {
                let bytes = fs::read(&path).expect("Failed to read image file");
                embedded_assets.push_str(&format!(
                    "pub const {}: &[u8] = &{:?};\n",
                    var_name,
                    bytes
                ));
            },
            ext => panic!("Unknown extension: {}", ext),
        }
    }

    fs::write(&dest_path, embedded_assets).expect("Unable to write file");
    println!("cargo:rerun-if-changed=assets/");
}

