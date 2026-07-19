use crate::contracts::UpdateManifest;
use crate::state::UpdaterState;

pub async fn load(state: &UpdaterState) -> Result<UpdateManifest, String> {
    let request = reqwest::Client::new().get(format!(
        "{}/api/updater/manifest",
        state.console_origin.trim_end_matches('/')
    ));
    let response = if let Some(access_token) = state.access_token.read().await.clone() {
        request.bearer_auth(access_token)
    } else {
        request
    }
    .send()
    .await
    .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Manifest request failed: {}", response.status()));
    }
    response
        .json::<UpdateManifest>()
        .await
        .map_err(|error| error.to_string())
}
