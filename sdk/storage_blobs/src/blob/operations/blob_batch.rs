use crate::{
    blob::{
        blob_batch_operations::{BlobBatchOperation, BlobBatchOperations},
        ResType,
    },
    prelude::*,
};
use azure_core::{headers::*, prelude::*, Method, Request, RequestId};
use time::OffsetDateTime;

operation! {
    BlobBatch,
    client: ContainerClient,
    operations: BlobBatchOperations,
    ?timeout: Timeout
}

impl BlobBatchBuilder {
    /// Delete a key
    pub fn delete<BN: Into<String>>(mut self, blob_name: BN) -> azure_core::Result<Self> {
        let entity_client = self.client.blob_client(blob_name);
        let url = entity_client.url()?;

        let mut request = Request::new(url, Method::Delete);
        request.insert_header(ACCEPT, "application/json;odata=minimalmetadata");

        request.set_body("");

        self.operations.add(BlobBatchOperation::new(request));
        Ok(self)
    }

    pub fn into_future(mut self) -> BlobBatch {
        Box::pin(async move {
            let mut url = self.client.url()?;

            url.query_pairs_mut().append_pair("comp", "batch");
            self.timeout.append_to_url_query(&mut url);
            ResType::Container.append_to_url_query(&mut url);

            let request_body = Some(self.operations.to_string()?.into());

            let mut headers = Headers::new();
            headers.insert(
                CONTENT_TYPE,
                format!(
                    "multipart/mixed; boundary=batch_{}",
                    self.operations.batch_uuid().hyphenated()
                ),
            );

            let mut request =
                ContainerClient::finalize_request(url, Method::Post, headers, request_body)?;

            let response = self.client.send(&mut self.context, &mut request).await?;
            BlobBatchResponse::from_headers(response.headers())
        })
    }
}

azure_storage::response_from_headers!(BlobBatchResponse,
    request_id_from_headers => request_id: RequestId,
    date_from_headers => date: OffsetDateTime
);
