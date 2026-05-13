use crate::{blob::SourceContentMD5, prelude::*};
use azure_core::{headers::*, prelude::*, RequestId, Url};
use azure_storage::{headers::consistency_from_headers, ConsistencyCRC64, ConsistencyMD5};
use time::OffsetDateTime;

// Source customer-provided encryption key headers on `Put Blob From URL`
// require API version 2026-02-06. Override per-request so other operations
// keep using the SDK-wide default.
const PUT_BLOB_FROM_URL_VERSION: HeaderValue = HeaderValue::from_static("2026-02-06");

operation! {
    PutBlobFromUrl,
    client: BlobClient,
    source_url: Url,
    ?content_type: BlobContentType,
    ?content_encoding: BlobContentEncoding,
    ?content_language: BlobContentLanguage,
    ?content_disposition: BlobContentDisposition,
    ?metadata: Metadata,
    ?access_tier: AccessTier,
    ?tags: Tags,
    ?lease_id: LeaseId,
    ?encryption_key: CPKInfo,
    ?source_encryption_key: SourceCPKInfo,
    ?encryption_scope: EncryptionScope,
    ?if_modified_since: IfModifiedSinceCondition,
    ?if_match: IfMatchCondition,
    ?if_tags: IfTags,
    ?if_source_since: IfSourceModifiedSinceCondition,
    ?if_source_match: IfSourceMatchCondition,
    ?source_content_md5: SourceContentMD5
}

impl PutBlobFromUrlBuilder {
    pub fn into_future(mut self) -> PutBlobFromUrl {
        Box::pin(async move {
            let url = self.client.url()?;

            let mut headers = Headers::new();
            headers.insert(BLOB_TYPE, "BlockBlob");
            headers.insert(COPY_SOURCE, self.source_url.to_string());
            headers.add(self.content_type);
            headers.add(self.content_encoding);
            headers.add(self.content_language);
            headers.add(self.content_disposition);
            headers.add(self.tags);
            if let Some(metadata) = &self.metadata {
                for m in metadata.iter() {
                    headers.add(m);
                }
            }
            headers.add(self.access_tier);
            headers.add(self.lease_id);
            headers.add(self.encryption_key);
            headers.add(self.source_encryption_key);
            headers.add(self.encryption_scope);
            headers.add(self.if_modified_since);
            headers.add(self.if_match);
            headers.add(self.if_tags);
            headers.add(self.if_source_since);
            headers.add(self.if_source_match);
            headers.add(self.source_content_md5);

            let mut request =
                BlobClient::finalize_request(url, azure_core::Method::Put, headers, None)?;
            request.insert_header(VERSION, PUT_BLOB_FROM_URL_VERSION);

            let response = self.client.send(&mut self.context, &mut request).await?;
            PutBlobFromUrlResponse::from_headers(response.headers())
        })
    }
}

#[derive(Debug, Clone)]
pub struct PutBlobFromUrlResponse {
    pub etag: String,
    pub last_modified: OffsetDateTime,
    pub content_md5: Option<ConsistencyMD5>,
    pub content_crc64: Option<ConsistencyCRC64>,
    pub request_id: RequestId,
    pub date: OffsetDateTime,
    pub request_server_encrypted: bool,
}

impl PutBlobFromUrlResponse {
    pub fn from_headers(headers: &Headers) -> azure_core::Result<PutBlobFromUrlResponse> {
        let etag = etag_from_headers(headers)?;
        let last_modified = last_modified_from_headers(headers)?;
        let (content_md5, content_crc64) = consistency_from_headers(headers)?;
        let request_id = request_id_from_headers(headers)?;
        let date = date_from_headers(headers)?;
        let request_server_encrypted = request_server_encrypted_from_headers(headers)?;

        Ok(PutBlobFromUrlResponse {
            etag,
            last_modified,
            content_md5,
            content_crc64,
            request_id,
            date,
            request_server_encrypted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::ClientBuilder;
    use azure_core::{
        BytesStream, Context, Policy, PolicyResult, Request, Response, StatusCode, TransportOptions,
    };
    use azure_storage::StorageCredentials;
    use bytes::Bytes;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct CapturingTransport {
        captured: Mutex<Option<Request>>,
    }

    #[async_trait::async_trait]
    impl Policy for CapturingTransport {
        async fn send(
            &self,
            _ctx: &Context,
            request: &mut Request,
            _next: &[Arc<dyn Policy>],
        ) -> PolicyResult {
            *self.captured.lock().unwrap() = Some(request.clone());
            // Synthesize an empty 201 so the pipeline returns cleanly. The
            // operation's response parser will fail on the empty headers,
            // but the request has already been captured by then.
            let body: BytesStream = Bytes::new().into();
            Ok(Response::new(
                StatusCode::Created,
                Headers::new(),
                Box::pin(body),
            ))
        }
    }

    #[tokio::test]
    async fn put_blob_from_url_overrides_x_ms_version_to_2026_02_06() {
        let captor = Arc::new(CapturingTransport::default());
        let transport = TransportOptions::new_custom_policy(captor.clone());

        let creds = StorageCredentials::bearer_token("dummy".to_string());
        let blob = ClientBuilder::new("acct", creds)
            .transport(transport)
            .blob_client("container", "blob");

        let src: Url = "https://other.blob.core.windows.net/c/b".parse().unwrap();
        // Result is ignored: response parsing fails on the synthetic empty
        // body, but the request has been captured by the transport policy.
        let _ = blob.put_blob_from_url(src).into_future().await;

        let captured = captor
            .captured
            .lock()
            .unwrap()
            .clone()
            .expect("transport policy never received a request");

        assert_eq!(
            captured.headers().get_optional_str(&VERSION),
            Some("2026-02-06"),
            "put_blob_from_url must send x-ms-version: 2026-02-06",
        );
    }
}
