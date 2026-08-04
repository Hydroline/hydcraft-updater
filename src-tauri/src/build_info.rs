use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildIdentity {
    pub version: &'static str,
    pub commit_sha: &'static str,
    pub platform: &'static str,
}

pub fn current() -> BuildIdentity {
    BuildIdentity {
        version: env!("HYDCRAFT_UPDATER_VERSION"),
        commit_sha: env!("HYDCRAFT_UPDATER_COMMIT"),
        platform: env!("HYDCRAFT_UPDATER_PLATFORM"),
    }
}

pub fn identity_json() -> String {
    serde_json::to_string(&current()).expect("Updater build identity must serialize")
}
