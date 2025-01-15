use crate::{
    federated_credentials_flow, token_credentials::cache::TokenCache, TokenCredentialOptions,
};
use async_lock::Mutex;
use azure_core::{
    auth::{AccessToken, Secret, TokenCredential},
    base64,
    error::{ErrorKind, ResultExt},
    Error, HttpClient,
};
use std::{
    path::PathBuf,
    str,
    sync::Arc,
    time::{Duration, SystemTime},
};
use time::OffsetDateTime;
use url::Url;

const AZURE_TENANT_ID_ENV_KEY: &str = "AZURE_TENANT_ID";
const AZURE_CLIENT_ID_ENV_KEY: &str = "AZURE_CLIENT_ID";
const AZURE_FEDERATED_TOKEN_FILE: &str = "AZURE_FEDERATED_TOKEN_FILE";
const AZURE_FEDERATED_TOKEN: &str = "AZURE_FEDERATED_TOKEN";

/// Enables authentication to Azure Active Directory using a client secret that was generated for an App Registration.
///
/// More information on how to configure a client secret can be found here:
/// <https://docs.microsoft.com/azure/active-directory/develop/quickstart-configure-app-access-web-apis#add-credentials-to-your-web-application>

#[derive(Debug)]
pub struct WorkloadIdentityCredential {
    http_client: Arc<dyn HttpClient>,
    authority_host: Url,
    tenant_id: String,
    client_id: String,
    token: TokenMode,
    cache: TokenCache,
}

#[derive(Debug)]
enum TokenMode {
    Env(Secret),
    File {
        path: PathBuf,
        token: Mutex<(Secret, Option<SystemTime>)>,
    },
}

impl WorkloadIdentityCredential {
    /// Create a new `WorkloadIdentityCredential`
    fn new(
        http_client: Arc<dyn HttpClient>,
        authority_host: Url,
        tenant_id: String,
        client_id: String,
        token: TokenMode,
    ) -> Self {
        Self {
            http_client,
            authority_host,
            tenant_id,
            client_id,
            token,
            cache: TokenCache::new(),
        }
    }

    pub fn create(
        options: impl Into<TokenCredentialOptions>,
    ) -> azure_core::Result<WorkloadIdentityCredential> {
        let options = options.into();
        let http_client = options.http_client();
        let authority_host = options.authority_host()?;
        let env = options.env();
        let tenant_id =
            env.var(AZURE_TENANT_ID_ENV_KEY)
                .with_context(ErrorKind::Credential, || {
                    format!(
                        "working identity credential requires {} environment variable",
                        AZURE_TENANT_ID_ENV_KEY
                    )
                })?;
        let client_id =
            env.var(AZURE_CLIENT_ID_ENV_KEY)
                .with_context(ErrorKind::Credential, || {
                    format!(
                        "working identity credential requires {} environment variable",
                        AZURE_CLIENT_ID_ENV_KEY
                    )
                })?;

        if let Ok(token) = env
            .var(AZURE_FEDERATED_TOKEN)
            .map_kind(ErrorKind::Credential)
        {
            return Ok(WorkloadIdentityCredential::new(
                http_client,
                authority_host,
                tenant_id,
                client_id,
                TokenMode::Env(token.into()),
            ));
        }

        if let Ok(token_file) = env
            .var(AZURE_FEDERATED_TOKEN_FILE)
            .map_kind(ErrorKind::Credential)
        {
            let path = PathBuf::from(token_file);
            let token =
                std::fs::read_to_string(&path).with_context(ErrorKind::Credential, || {
                    format!(
                        "failed to read federated token from file {}",
                        path.display()
                    )
                })?;

            let token: Secret = token.into();
            let expiration = parse_expiration(&token);
            return Ok(WorkloadIdentityCredential::new(
                http_client,
                authority_host,
                tenant_id,
                client_id,
                TokenMode::File {
                    path,
                    token: Mutex::new((token, expiration)),
                },
            ));
        }

        Err(Error::with_message(ErrorKind::Credential, || {
            format!("working identity credential requires {AZURE_FEDERATED_TOKEN} or {AZURE_FEDERATED_TOKEN_FILE} environment variables")
        }))
    }

    async fn get_token(&self, scopes: &[&str]) -> azure_core::Result<AccessToken> {
        let token_copy;
        let token = match &self.token {
            TokenMode::Env(secret) => secret,
            TokenMode::File { path, token } => {
                let mut lock = token.lock().await;
                if lock.1.is_some_and(|t| t < SystemTime::now()) {
                    let new_token = tokio::fs::read_to_string(path).await.with_context(
                        ErrorKind::Credential,
                        || {
                            format!(
                                "failed to read federated token from file {}",
                                path.display()
                            )
                        },
                    )?;

                    let new_token: Secret = new_token.into();
                    let expiration = parse_expiration(&new_token);
                    *lock = (new_token, expiration);
                }
                token_copy = lock.0.clone();
                &token_copy
            }
        };

        let res: AccessToken = federated_credentials_flow::perform(
            self.http_client.clone(),
            &self.client_id,
            token.secret(),
            scopes,
            &self.tenant_id,
            &self.authority_host,
        )
        .await
        .map(|r| {
            AccessToken::new(
                r.access_token().clone(),
                OffsetDateTime::now_utc() + Duration::from_secs(r.expires_in),
            )
        })
        .context(ErrorKind::Credential, "request token error")?;
        Ok(res)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl TokenCredential for WorkloadIdentityCredential {
    async fn get_token(&self, scopes: &[&str]) -> azure_core::Result<AccessToken> {
        self.cache.get_token(scopes, self.get_token(scopes)).await
    }

    async fn clear_cache(&self) -> azure_core::Result<()> {
        self.cache.clear().await
    }
}

/// Assume the token is a JWT and try extract the `exp` field.
fn parse_expiration(token: &Secret) -> Option<SystemTime> {
    #[derive(serde::Deserialize)]
    struct Payload {
        /// <https://datatracker.ietf.org/doc/html/rfc7519#section-4.1.4>.
        ///
        /// A JSON numeric value representing the number of seconds from
        /// 1970-01-01T00:00:00Z UTC until the specified UTC date/time
        exp: u64,
    }

    // split the JWT into the 3 main components. `<header>.<payload>.<signature>`
    let (body, _sig) = token.secret().rsplit_once('.')?;
    let (_header, payload) = body.rsplit_once('.')?;

    // base64 decode the payload.
    let payload = base64::decode_url_safe(payload).ok()?;

    // json parse the payload, assuming there is an `exp: u64` field.
    let payload = serde_json::from_slice::<Payload>(&payload).ok()?;

    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(payload.exp))
}
