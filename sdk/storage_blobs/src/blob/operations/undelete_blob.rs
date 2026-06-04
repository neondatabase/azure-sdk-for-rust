use crate::prelude::*;
use azure_core::{headers::*, RequestId};
use time::OffsetDateTime;

operation! {
    UndeleteBlob,
    client: BlobClient,
}

impl UndeleteBlobBuilder {
    pub fn into_future(mut self) -> UndeleteBlob {
        Box::pin(async move {
            let mut url = self.client.url()?;

            url.query_pairs_mut().append_pair("comp", "undelete");

            let mut request =
                BlobClient::finalize_request(url, azure_core::Method::Put, Headers::new(), None)?;

            let response = self.client.send(&mut self.context, &mut request).await?;
            UndeleteBlobResponse::from_headers(response.headers())
        })
    }
}

azure_storage::response_from_headers!(UndeleteBlobResponse,
    request_id_from_headers => request_id: RequestId,
    date_from_headers => date: OffsetDateTime
);
