//! Background GitHub Releases update operations for the Windows tray app.

use self_update::backends::github;

const REPOSITORY_OWNER: &str = "bkntr";
const REPOSITORY_NAME: &str = "tibetan-keyboard";
const BINARY_NAME: &str = "tibetan-ewts-keyboard";
const ASSET_IDENTIFIER: &str = "standalone";

pub fn check() -> Result<Option<String>, String> {
    let updater = configured_updater(None)?;
    let Some(release) = updater.is_update_available().map_err(display_error)? else {
        return Ok(None);
    };

    let target = self_update::get_target();
    if release.asset_for(target, Some(ASSET_IDENTIFIER)).is_none() {
        return Err(format!(
            "Release v{} does not contain a standalone build for {target}.",
            release.version()
        ));
    }

    Ok(Some(release.version().to_owned()))
}

pub fn install(version: &str) -> Result<String, String> {
    let release_tag = format!("v{version}");
    let status = configured_updater(Some(&release_tag))?
        .update()
        .map_err(display_error)?;
    Ok(status.version().to_owned())
}

fn configured_updater(release_tag: Option<&str>) -> Result<github::Update, String> {
    let mut builder = github::Update::configure();
    builder
        .repo_owner(REPOSITORY_OWNER)
        .repo_name(REPOSITORY_NAME)
        .bin_name(BINARY_NAME)
        .asset_identifier(ASSET_IDENTIFIER)
        .current_version(env!("CARGO_PKG_VERSION"))
        .unattended()
        .timeout(std::time::Duration::from_secs(30))
        .retries(2);
    if let Some(release_tag) = release_tag {
        builder.release_tag(release_tag);
    }
    builder.build().map_err(display_error)
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_asset_contract_is_unambiguous() {
        let target = self_update::get_target();
        let asset = format!(
            "{BINARY_NAME}-{}-{target}-{ASSET_IDENTIFIER}.exe",
            env!("CARGO_PKG_VERSION")
        );
        let release = self_update::Release::builder()
            .version(env!("CARGO_PKG_VERSION"))
            .asset(self_update::ReleaseAsset::new(
                &asset,
                "https://example.invalid",
            ))
            .build()
            .unwrap();
        let selected = release.asset_for(target, Some(ASSET_IDENTIFIER)).unwrap();

        assert_eq!(selected.name(), asset);
    }
}
