#![cfg(all(test))]
use blob_blob::BlobBlobDownloadOptions;
use blob_client::builders::BlobClientOptionsBuilder;
// use azure_core::credentials::TokenCredential;
use blob_storage::blob_client::BlobClientOptions;
use blob_storage::*;
// use std::sync::Arc;
use tokio;
#[tokio::test] // For nearly an HOUR, weird "cannot find 'test' in tokio issue", it's because it was pulling in azure-core's tokio because I changed the prelude import to ::*
async fn test_download_blob() {
    let endpoint = String::from("https://vincenttranpublicac.blob.core.windows.net/");
    let options = Some(BlobClientOptions::default());
    let top_level_client = BlobClient::with_no_credential(endpoint, options).unwrap();

    let actual_blob_client = top_level_client.get_blob_blob_client();

    let download_resp = actual_blob_client
        .download(
            "public",
            "hello.txt",
            "80bc3c5e-3bb7-95f6-6c57-8ceb2c9155vc",
            "2024-08-04",
            Some(BlobBlobDownloadOptions::default()),
        )
        .await
        .unwrap();
    print!("{:?}", download_resp);
    print!(
        "\n{:?}",
        download_resp.into_body().collect_string().await.unwrap()
    );
}
