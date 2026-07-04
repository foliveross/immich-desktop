use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "immich-desktop";
const API_KEY_ACCOUNT: &str = "api_key";

pub fn store_api_key(api_key: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, API_KEY_ACCOUNT)
        .context("Failed to create credential manager entry")?;
    entry
        .set_password(api_key)
        .context("Failed to store API key in Windows Credential Manager")
}

pub fn get_api_key() -> Result<Option<String>> {
    let entry = Entry::new(SERVICE_NAME, API_KEY_ACCOUNT)
        .context("Failed to create credential manager entry")?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("Failed to read API key from Credential Manager"),
    }
}

pub fn delete_api_key() -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, API_KEY_ACCOUNT)
        .context("Failed to create credential manager entry")?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e).context("Failed to delete API key from Credential Manager"),
    }
}

pub fn has_api_key() -> bool {
    get_api_key()
        .ok()
        .flatten()
        .map(|k| !k.is_empty())
        .unwrap_or(false)
}
