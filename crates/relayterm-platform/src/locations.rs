use directories::{BaseDirs, ProjectDirs};
use std::{ffi::OsString, path::PathBuf};

/// Explicit bootstrap inputs. Callers read only the named environment variable.
#[derive(Clone, Default)]
pub struct LocationOptions {
    pub explicit_home: Option<PathBuf>,
    pub relayterm_home: Option<OsString>,
}

impl LocationOptions {
    pub fn from_process(explicit_home: Option<PathBuf>) -> Self {
        Self {
            explicit_home,
            relayterm_home: std::env::var_os("RELAYTERM_HOME"),
        }
    }
}

/// Resolved paths only. Resolution never creates filesystem objects.
#[derive(Clone, Eq, PartialEq)]
pub struct PrivateLocations {
    config: PathBuf,
    data: PathBuf,
    runtime: PathBuf,
    cache: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocationAlias {
    Config,
    Data,
    Runtime,
    Cache,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocationError {
    Unavailable,
    InvalidOverride,
}

impl PrivateLocations {
    pub fn resolve(inputs: LocationOptions) -> Result<Self, LocationError> {
        if let Some(root) = inputs
            .explicit_home
            .or_else(|| inputs.relayterm_home.map(PathBuf::from))
        {
            if !root.is_absolute() || root.as_os_str().is_empty() {
                return Err(LocationError::InvalidOverride);
            }
            return Ok(Self::under(root));
        }

        let project =
            ProjectDirs::from("com", "relayterm", "Relayterm").ok_or(LocationError::Unavailable)?;
        let base = BaseDirs::new().ok_or(LocationError::Unavailable)?;
        let mut data = project.data_local_dir().to_path_buf();
        let mut config = project.config_dir().to_path_buf();
        if data == config {
            data = data.join("data");
            config = config.join("config");
        }
        let cache = project.cache_dir().to_path_buf();
        let runtime = base
            .runtime_dir()
            .map(|path| path.join("relayterm"))
            .unwrap_or_else(|| data.join("runtime"));
        Ok(Self {
            config,
            data,
            runtime,
            cache,
        })
    }

    fn under(root: PathBuf) -> Self {
        Self {
            config: root.join("config"),
            data: root.join("data"),
            runtime: root.join("runtime"),
            cache: root.join("cache"),
        }
    }

    pub fn config(&self) -> &std::path::Path {
        &self.config
    }
    pub fn data(&self) -> &std::path::Path {
        &self.data
    }
    pub fn runtime(&self) -> &std::path::Path {
        &self.runtime
    }
    pub fn cache(&self) -> &std::path::Path {
        &self.cache
    }
    pub fn path(&self, alias: LocationAlias) -> &std::path::Path {
        match alias {
            LocationAlias::Config => self.config(),
            LocationAlias::Data => self.data(),
            LocationAlias::Runtime => self.runtime(),
            LocationAlias::Cache => self.cache(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_input_wins_and_resolution_has_no_side_effects() {
        let temporary = tempfile::tempdir().unwrap();
        let explicit = temporary.path().join("explicit");
        let locations = PrivateLocations::resolve(LocationOptions {
            explicit_home: Some(explicit.clone()),
            relayterm_home: Some(temporary.path().join("environment").into_os_string()),
        })
        .unwrap();
        assert_eq!(locations.data(), explicit.join("data"));
        assert!(!explicit.exists());
    }

    #[test]
    fn relative_overrides_are_rejected() {
        assert_eq!(
            PrivateLocations::resolve(LocationOptions {
                explicit_home: Some(PathBuf::from("relative")),
                relayterm_home: None,
            })
            .err(),
            Some(LocationError::InvalidOverride)
        );
    }
}
