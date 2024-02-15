use cargo_scout_macros::warn;

use crate::config::Config;
use colored::Colorize;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

/// This struct represents a Cargo project configuration.
pub struct CargoConfig {
    root: PathBuf,
    members: Vec<String>,
}

impl Config for CargoConfig {
    #[must_use]
    fn members(&self) -> Vec<String> {
        self.members.clone()
    }

    #[must_use]
    fn root(&self) -> &PathBuf {
        &self.root
    }
}

impl CargoConfig {
    /// This function will instantiate a Config from a Cargo.toml path.
    ///
    /// If in a workspace, `get_members` will return the members
    /// of the [[workspace]] members section in Cargo.toml.
    ///
    /// Else, it will return `vec![".".to_string()]`
    ///
    /// # cargo-scout-lib example
    /// ```
    /// # use cargo_scout_lib::config::Config;
    /// # use cargo_scout_lib::config::rust::CargoConfig;
    /// let config = CargoConfig::from_manifest_path("Cargo.toml")?;
    /// // There is only one directory to lint, which is the current one.
    /// assert_eq!(vec!["."], config.members());
    /// # Ok::<(), cargo_scout_lib::Error>(())
    /// ```
    ///
    /// # cargo-scout workspace example
    /// ```
    /// # use cargo_scout_lib::config::Config;
    /// # use cargo_scout_lib::config::rust::CargoConfig;
    /// let config = CargoConfig::from_manifest_path("../Cargo.toml")?;
    /// // We will lint `./cargo-scout` and `./cargo-scout-lib`.
    /// assert_eq!(vec!["cargo-scout".to_string(), "cargo-scout-lib".to_string()], config.members());
    /// # Ok::<(), cargo_scout_lib::Error>(())
    /// ```
    #[allow(clippy::missing_errors_doc)]
    pub fn from_manifest_path(
        p: impl AsRef<Path> + Clone,
        only_members: &[String],
    ) -> Result<Self, crate::error::Error> {
        Ok(Self::from_manifest(
            p.clone(),
            cargo_toml::Manifest::from_path(p)?,
            only_members,
        ))
    }

    fn from_manifest(
        p: impl AsRef<Path>,
        m: cargo_toml::Manifest,
        only_members: &[String],
    ) -> Self {
        if let Some(w) = m.workspace {
            Self {
                root: std::fs::canonicalize(p.as_ref().parent().unwrap())
                    .unwrap()
                    .to_path_buf(),
                members: w
                    .members
                    .into_iter()
                    .filter(|m| {
                        if only_members.is_empty() {
                            return true;
                        }
                        // return the last path segment as the member name
                        let Ok(pb) = PathBuf::from_str(m) else {
                            warn!("failed to parse member name {} in predicate", m);
                            return false;
                        };
                        let Some(final_path_seg) = pb.file_name().and_then(|f| f.to_str()) else {
                            warn!("failed to convert member {} pathbuf to str", m);
                            return false;
                        };

                        // filter by the last named segment of the workspace member--- the manifest folder
                        only_members.contains(&final_path_seg.to_owned())
                    })
                    .collect(),
            }
        } else {
            Self {
                root: std::fs::canonicalize(p.as_ref().parent().unwrap())
                    .unwrap()
                    .to_path_buf(),
                // Project root only
                members: vec![".".to_string()],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::rust::CargoConfig;
    use crate::config::Config;

    #[test]
    fn test_not_workspace_manifest() {
        let manifest = cargo_toml::Manifest::from_path("Cargo.toml").unwrap();
        // Make sure we actually parsed the manifest
        assert_eq!("cargo-scout-lib", manifest.clone().package.unwrap().name);
        let config = CargoConfig::from_manifest(manifest, &[]);
        assert_eq!(vec!["."], config.members());
    }
    #[test]
    fn test_not_workspace_path() {
        let config = CargoConfig::from_manifest_path("Cargo.toml", &[]).unwrap();
        assert_eq!(vec!["."], config.members());
    }
    #[test]
    fn test_neqo_members_manifest() {
        let neqo_toml = r#"[workspace]
        members = [
          "neqo-client",
          "neqo-common",
          "neqo-crypto",
          "neqo-http3",
          "neqo-http3-server",
          "neqo-qpack",
          "neqo-server",
          "neqo-transport",
          "neqo-interop",
          "test-fixture",
        ]"#;

        let manifest = cargo_toml::Manifest::from_slice(neqo_toml.as_bytes()).unwrap();
        let config = CargoConfig::from_manifest(manifest);
        assert_eq!(
            vec![
                "neqo-client",
                "neqo-common",
                "neqo-crypto",
                "neqo-http3",
                "neqo-http3-server",
                "neqo-qpack",
                "neqo-server",
                "neqo-transport",
                "neqo-interop",
                "test-fixture"
            ],
            config.members()
        );
    }
}
