use azure_core::errors::AzureError;
use azure_core::headers::{
    consistency_from_headers, date_from_headers, etag_from_headers, last_modified_from_headers,
    request_id_from_headers, request_server_encrypted_from_headers,
};
use azure_core::{ConsistencyCRC64, ConsistencyMD5, RequestId};
use chrono::{DateTime, Utc};
use http::HeaderMap;

#[derive(Debug, Clone)]
pub struct PutBlockBlobResponse {
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub content_md5: ConsistencyMD5,
    pub content_crc64: Option<ConsistencyCRC64>,
    pub request_id: RequestId,
    pub date: DateTime<Utc>,
    pub request_server_encrypted: bool,
}

impl PutBlockBlobResponse {
    pub fn from_headers(headers: &HeaderMap) -> Result<PutBlockBlobResponse, AzureError> {
        debug!("headers == {:#?}", headers);

        let etag = etag_from_headers(headers)?;
        let last_modified = last_modified_from_headers(headers)?;
        let (content_md5, content_crc64) = consistency_from_headers(headers)?;
        let request_id = request_id_from_headers(headers)?;
        let date = date_from_headers(headers)?;
        let request_server_encrypted = request_server_encrypted_from_headers(headers)?;

        Ok(PutBlockBlobResponse {
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
