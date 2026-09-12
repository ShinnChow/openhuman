//! JSON-RPC / CLI controller surface for credentials and app session auth.

mod composio;
mod login_tokens;
mod oauth;
mod provider_credentials;
mod session_lifecycle;

pub use composio::{
    clear_composio_api_key, get_composio_api_key, rpc_store_composio_api_key,
    store_composio_api_key, COMPOSIO_DIRECT_PROVIDER,
};
pub use login_tokens::{auth_create_channel_link_token, consume_login_token};
pub use oauth::{
    oauth_connect, oauth_fetch_client_key, oauth_fetch_integration_tokens,
    oauth_list_integrations, oauth_revoke_integration,
};
pub use provider_credentials::{
    list_provider_credentials, list_provider_credentials_by_prefix, remove_provider_credentials,
    store_provider_credentials,
};
pub use session_lifecycle::{
    auth_get_me, auth_get_session_token_json, auth_get_state, clear_session, decrypt_secret,
    encrypt_secret, start_login_gated_services, stop_login_gated_services, store_session,
    store_session_with_deferred_validation,
};

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;
