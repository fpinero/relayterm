use relayterm_domain::WorkspaceId;
use relayterm_platform::{
    PrivateLocations, create_private_dir, create_private_file, validate_private_file,
};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_RECORD_BYTES: usize = 4096;
const ROTATED_FILES: usize = 3;

pub(crate) struct DiagnosticLog {
    path: PathBuf,
}

#[derive(Serialize)]
struct Record<'a> {
    severity: &'a str,
    event: &'a str,
    workspace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation: Option<&'a str>,
}

impl DiagnosticLog {
    pub(crate) fn open(
        locations: &PrivateLocations,
        workspace_id: WorkspaceId,
    ) -> Result<Self, ()> {
        let directory = locations
            .data()
            .join("workspaces")
            .join(workspace_id.to_string())
            .join("logs");
        create_private_dir(&directory).map_err(|_| ())?;
        let path = directory.join("daemon.log");
        if path.exists() {
            validate_private_file(&path).map_err(|_| ())?;
        } else {
            create_private_file(&path).map_err(|_| ())?;
        }
        Ok(Self { path })
    }

    pub(crate) fn write(
        &self,
        severity: &'static str,
        event: &'static str,
        workspace_id: WorkspaceId,
        generation: Option<&str>,
    ) {
        if validate_private_file(&self.path).is_err() {
            return;
        }
        let Ok(mut bytes) = serde_json::to_vec(&Record {
            severity,
            event,
            workspace_id: workspace_id.to_string(),
            generation,
        }) else {
            return;
        };
        bytes.push(b'\n');
        if bytes.len() > MAX_RECORD_BYTES {
            return;
        }
        let current = fs::metadata(&self.path)
            .map(|value| value.len())
            .unwrap_or(0);
        if current.saturating_add(bytes.len() as u64) > MAX_FILE_BYTES && self.rotate().is_err() {
            return;
        }
        let Ok(mut file) = OpenOptions::new().append(true).open(&self.path) else {
            return;
        };
        let _ = file.write_all(&bytes);
        let _ = file.flush();
    }

    fn rotate(&self) -> Result<(), ()> {
        let oldest = self.path.with_extension(format!("log.{ROTATED_FILES}"));
        if oldest.exists() {
            validate_private_file(&oldest).map_err(|_| ())?;
            fs::remove_file(&oldest).map_err(|_| ())?;
        }
        for index in (1..ROTATED_FILES).rev() {
            let source = self.path.with_extension(format!("log.{index}"));
            if source.exists() {
                validate_private_file(&source).map_err(|_| ())?;
                fs::rename(
                    &source,
                    self.path.with_extension(format!("log.{}", index + 1)),
                )
                .map_err(|_| ())?;
            }
        }
        validate_private_file(&self.path).map_err(|_| ())?;
        fs::rename(&self.path, self.path.with_extension("log.1")).map_err(|_| ())?;
        create_private_file(&self.path).map_err(|_| ())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use relayterm_platform::{LocationOptions, PrivateLocations};

    #[test]
    fn records_are_bounded_and_contain_only_allowlisted_fields() {
        let temporary = tempfile::tempdir().unwrap();
        let locations = PrivateLocations::resolve(LocationOptions {
            explicit_home: Some(temporary.path().join("private")),
            relayterm_home: None,
        })
        .unwrap();
        let id: WorkspaceId = "00000000-0000-4000-8000-000000000501".parse().unwrap();
        let log = DiagnosticLog::open(&locations, id).unwrap();
        log.write("info", "daemon.ready", id, Some("synthetic-generation"));
        let text = fs::read_to_string(log.path).unwrap();
        assert!(text.len() <= MAX_RECORD_BYTES);
        assert!(text.contains("daemon.ready"));
        assert!(!text.contains(temporary.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn rotation_retains_at_most_four_bounded_files() {
        let temporary = tempfile::tempdir().unwrap();
        let locations = PrivateLocations::resolve(LocationOptions {
            explicit_home: Some(temporary.path().join("private")),
            relayterm_home: None,
        })
        .unwrap();
        let id: WorkspaceId = "00000000-0000-4000-8000-000000000502".parse().unwrap();
        let log = DiagnosticLog::open(&locations, id).unwrap();
        for _ in 0..30_000 {
            log.write(
                "info",
                "ipc.request_failed",
                id,
                Some("synthetic-generation"),
            );
        }
        let directory = log.path.parent().unwrap();
        let files = fs::read_dir(directory)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(files.len() <= ROTATED_FILES + 1);
        assert!(
            files
                .iter()
                .all(|entry| entry.metadata().unwrap().len() <= MAX_FILE_BYTES)
        );
    }
}
