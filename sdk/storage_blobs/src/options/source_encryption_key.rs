use azure_core::headers::{self, AsHeaders, HeaderName, HeaderValue};

const DEFAULT_ENCRYPTION_ALGORITHM: &str = "AES256";

/// Source-side customer-provided encryption key, used to decrypt the source
/// blob when performing a server-side copy via `Put Blob From URL`.
#[derive(Clone, Debug)]
pub struct SourceCPKInfo {
    encryption_key: String,
    encryption_key_sha256: String,
    encryption_algorithm: Option<String>,
}

impl SourceCPKInfo {
    pub fn new(key: String, key_sha256: String, algorithm: Option<String>) -> Self {
        Self {
            encryption_key: key,
            encryption_key_sha256: key_sha256,
            encryption_algorithm: algorithm,
        }
    }
}

impl From<(String, String)> for SourceCPKInfo {
    fn from(s: (String, String)) -> Self {
        Self::new(s.0, s.1, None)
    }
}

impl From<(String, String, String)> for SourceCPKInfo {
    fn from(s: (String, String, String)) -> Self {
        Self::new(s.0, s.1, Some(s.2))
    }
}

impl AsHeaders for SourceCPKInfo {
    type Iter = std::vec::IntoIter<(HeaderName, HeaderValue)>;

    fn as_headers(&self) -> Self::Iter {
        let algorithm = self
            .encryption_algorithm
            .as_deref()
            .unwrap_or(DEFAULT_ENCRYPTION_ALGORITHM)
            .to_owned();
        let headers = vec![
            (headers::SOURCE_ENCRYPTION_ALGORITHM, algorithm.into()),
            (
                headers::SOURCE_ENCRYPTION_KEY,
                self.encryption_key.clone().into(),
            ),
            (
                headers::SOURCE_ENCRYPTION_KEY_SHA256,
                self.encryption_key_sha256.clone().into(),
            ),
        ];
        headers.into_iter()
    }
}

impl AsHeaders for &SourceCPKInfo {
    type Iter = <SourceCPKInfo as AsHeaders>::Iter;

    fn as_headers(&self) -> Self::Iter {
        (*self).as_headers()
    }
}
