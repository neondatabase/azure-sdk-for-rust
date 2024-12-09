use azure_core::{AppendToUrlQuery, Url};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResType {
    Container,
}

impl ResType {
    pub fn to_str(&self) -> &str {
        match self {
            ResType::Container => "container",
        }
    }
}

impl AppendToUrlQuery for ResType {
    fn append_to_url_query(&self, url: &mut Url) {
        url.query_pairs_mut().append_pair("restype", self.to_str());
    }
}
