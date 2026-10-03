use crate::{Client, errors::ApiError};

/// Requires real Quantframe credentials pasted into `user`/`pass` below, so it
/// cannot pass unattended. Kept rather than deleted: it becomes a usable
/// authentication check once this project runs its own API server.
/// See docs/superpowers/research/2026-10-03-self-hosting-the-quantframe-api.md
/// Run with: cargo test -p qf_api -- --ignored
#[ignore = "needs real credentials; see docs/superpowers/research/2026-10-03-self-hosting-the-quantframe-api.md"]
#[tokio::test]
async fn print_token() {
    let user = "";
    let pass = "";

    assert!(!user.is_empty());
    assert!(!pass.is_empty());

    let mut client = Client::new(
        "N/A",
        "default",
        "v1",
        "https://example.com",
        true,
        "https://example.com",
        "https://example.com",
        "https://example.com",
        "https://example.com",
        "https://example.com",
        false,
    );
    match client.authentication().signin(user, pass).await {
        Ok(_) => {
            // client.print_info();
        }
        Err(e) => match e {
            ApiError::InvalidCredentials(err) => {
                println!("Invalid credentials: {}", err);
            }
            _ => {
                println!("Error signing in: {}", e);
            }
        },
    }
    client.set_token("new_token");
}

/// Expects a Quantframe API server on http://localhost:6969 (the client's
/// DEVELOPMENT_URL). Kept rather than deleted: it is already a conformance test
/// for the cache endpoints of a self-hosted server.
/// See docs/superpowers/research/2026-10-03-self-hosting-the-quantframe-api.md
/// Run with: cargo test -p qf_api -- --ignored
#[ignore = "needs a local API server on :6969; see docs/superpowers/research/2026-10-03-self-hosting-the-quantframe-api.md"]
#[tokio::test]
async fn test_cache_extract() {
    let client = Client::new(
        "N/A",
        "default",
        "v1",
        "https://example.com",
        true,
        "https://example.com",
        "https://example.com",
        "https://example.com",
        "https://example.com",
        "https://example.com",
        false,
    );
    match client.cache().download_cache("cache").await {
        Ok(zip_data) => {
            println!("Successfully downloaded cache ({} bytes)", zip_data.len());

            let reader = std::io::Cursor::new(zip_data);
            match zip::ZipArchive::new(reader) {
                Ok(mut archive) => {
                    println!(
                        "Successfully opened zip archive with {} files",
                        archive.len()
                    );

                    let temp_dir = std::env::temp_dir().join("qf_cache_test");

                    // Clear existing test cache
                    if temp_dir.exists() {
                        let _ = std::fs::remove_dir_all(&temp_dir);
                    }

                    let mut total_size = 0u64;
                    for i in 0..archive.len() {
                        if let Ok(mut file) = archive.by_index(i) {
                            let output_path = temp_dir.join(file.mangled_name());

                            if file.is_dir() {
                                let _ = std::fs::create_dir_all(&output_path);
                            } else {
                                if let Some(parent) = output_path.parent()
                                    && !parent.exists() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }

                                if let Ok(mut output_file) = std::fs::File::create(&output_path) {
                                    total_size += file.size();
                                    let _ = std::io::copy(&mut file, &mut output_file);
                                }
                            }
                        }
                    }

                    println!("Successfully extracted cache ({} bytes total)", total_size);
                    assert!(total_size > 0, "Should have extracted some data");
                    println!("Extracted to {:?}", temp_dir);
                    // Cleanup
                    // let _ = std::fs::remove_dir_all(&temp_dir);s
                }
                Err(e) => {
                    println!("Failed to read zip archive: {}", e);
                    panic!("Zip extraction failed");
                }
            }
        }
        Err(e) => {
            println!("Failed to download cache: {:?}", e);
            panic!("Cache download failed");
        }
    }
}
