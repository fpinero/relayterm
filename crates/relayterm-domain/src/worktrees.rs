use crate::validation::{native_path, text};
use crate::*;
use std::path::PathBuf;

pub const WORKTREE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreePhase {
    Prepared,
    Applying,
    Ready,
    Failed,
    NeedsAttention,
}

impl WorktreePhase {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Ready | Self::Failed)
    }

    pub fn allows(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Prepared,
                Self::Applying | Self::Failed | Self::NeedsAttention
            ) | (
                Self::Applying,
                Self::Ready | Self::Failed | Self::NeedsAttention
            ) | (Self::NeedsAttention, Self::Ready | Self::Failed)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeHealth {
    Ready,
    Missing,
    Mismatch,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeReason {
    GitMissing,
    GitUnsupported,
    NotRepository,
    UnsupportedRoot,
    InvalidReference,
    BranchConflict,
    DestinationConflict,
    PathRejected,
    Busy,
    UnsupportedCheckoutFilter,
    StorageUnavailable,
    OutcomeUncertain,
    CancelledTask,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ApprovedRootRecord {
    pub id: ApprovedRootId,
    pub workspace_id: WorkspaceId,
    pub canonical_parent: PathBuf,
    pub filesystem_identity: Option<Vec<u8>>,
    pub private_default: bool,
    pub created_at: Timestamp,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ApprovedRoot(ApprovedRootRecord);

impl ApprovedRoot {
    pub fn restore(record: ApprovedRootRecord) -> Result<Self> {
        native_path(&record.canonical_parent, "approved_root")?;
        if record
            .filesystem_identity
            .as_ref()
            .is_some_and(|value| value.len() > 1024)
        {
            return Err(Error::Validation("filesystem_identity"));
        }
        Ok(Self(record))
    }

    pub fn record(&self) -> &ApprovedRootRecord {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct WorktreeIntentRecord {
    pub id: WorktreeOperationId,
    pub workspace_id: WorkspaceId,
    pub task_id: TaskId,
    pub worktree_id: WorktreeId,
    pub root_id: ApprovedRootId,
    pub actor: Actor,
    pub schema_version: u32,
    pub expected_revision: u64,
    pub repository_identity: PathBuf,
    pub common_directory_identity: PathBuf,
    pub destination: PathBuf,
    pub branch: String,
    pub base_expression: String,
    pub resolved_commit: String,
    pub request_fingerprint: [u8; 32],
    pub phase: WorktreePhase,
    pub reason: Option<WorktreeReason>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, PartialEq, Eq)]
pub struct WorktreeIntent(WorktreeIntentRecord);

impl WorktreeIntent {
    pub fn restore(record: WorktreeIntentRecord) -> Result<Self> {
        if record.schema_version != WORKTREE_SCHEMA_VERSION || record.actor != Actor::LocalUser {
            return Err(Error::Version);
        }
        native_path(&record.repository_identity, "repository_identity")?;
        native_path(
            &record.common_directory_identity,
            "common_directory_identity",
        )?;
        native_path(&record.destination, "destination")?;
        validate_ref_text(&record.branch, "branch")?;
        validate_ref_text(&record.base_expression, "base_expression")?;
        validate_object_id(&record.resolved_commit)?;
        if record.expected_revision == 0
            || (record.reason.is_some()
                && !matches!(
                    record.phase,
                    WorktreePhase::Failed | WorktreePhase::NeedsAttention
                ))
            || (record.reason.is_none()
                && matches!(
                    record.phase,
                    WorktreePhase::Failed | WorktreePhase::NeedsAttention
                ))
        {
            return Err(Error::State);
        }
        record.updated_at.not_before(record.created_at)?;
        Ok(Self(record))
    }

    pub fn record(&self) -> &WorktreeIntentRecord {
        &self.0
    }

    pub fn transition(
        &self,
        phase: WorktreePhase,
        reason: Option<WorktreeReason>,
        at: Timestamp,
    ) -> Result<Self> {
        if phase == self.0.phase {
            if reason == self.0.reason {
                return Ok(self.clone());
            }
            return Err(Error::Conflict);
        }
        if !self.0.phase.allows(phase) {
            return Err(Error::State);
        }
        at.not_before(self.0.updated_at)?;
        let mut record = self.0.clone();
        record.phase = phase;
        record.reason = reason;
        record.updated_at = at;
        Self::restore(record)
    }

    pub fn matches_fingerprint(&self, fingerprint: [u8; 32]) -> Result<()> {
        if self.0.request_fingerprint == fingerprint {
            Ok(())
        } else {
            Err(Error::Conflict)
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct WorktreeRecord {
    pub id: WorktreeId,
    pub workspace_id: WorkspaceId,
    pub task_id: TaskId,
    pub operation_id: WorktreeOperationId,
    pub root_id: ApprovedRootId,
    pub checkout_path: PathBuf,
    pub common_directory_identity: PathBuf,
    pub branch_ref: String,
    pub initial_base_commit: String,
    pub health: WorktreeHealth,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Worktree(WorktreeRecord);

impl Worktree {
    pub fn restore(record: WorktreeRecord) -> Result<Self> {
        native_path(&record.checkout_path, "checkout_path")?;
        native_path(
            &record.common_directory_identity,
            "common_directory_identity",
        )?;
        validate_ref_text(&record.branch_ref, "branch_ref")?;
        validate_object_id(&record.initial_base_commit)?;
        record.updated_at.not_before(record.created_at)?;
        Ok(Self(record))
    }

    pub fn record(&self) -> &WorktreeRecord {
        &self.0
    }

    pub fn observe_health(&self, health: WorktreeHealth, at: Timestamp) -> Result<Self> {
        at.not_before(self.0.updated_at)?;
        let mut record = self.0.clone();
        record.health = health;
        record.updated_at = at;
        Self::restore(record)
    }
}

pub fn validate_association(task: &Task, worktree: &Worktree) -> Result<()> {
    if task.record().workspace_id != worktree.record().workspace_id
        || task.record().id != worktree.record().task_id
        || worktree.record().health != WorktreeHealth::Ready
    {
        return Err(Error::Reference);
    }
    Ok(())
}

fn validate_ref_text(value: &str, field: &'static str) -> Result<()> {
    text(value, field, 256, true, false)?;
    if value.starts_with('-') {
        return Err(Error::Validation(field));
    }
    Ok(())
}

fn validate_object_id(value: &str) -> Result<()> {
    if !(4..=128).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::Validation("resolved_commit"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id<T: std::str::FromStr>(suffix: u64) -> T {
        format!("00000000-0000-4000-8000-{suffix:012x}")
            .parse()
            .ok()
            .unwrap()
    }

    fn at(value: i128) -> Timestamp {
        Timestamp::try_from(value).unwrap()
    }

    fn intent(phase: WorktreePhase, reason: Option<WorktreeReason>) -> WorktreeIntent {
        WorktreeIntent::restore(WorktreeIntentRecord {
            id: id(1),
            workspace_id: id(2),
            task_id: id(3),
            worktree_id: id(4),
            root_id: id(5),
            actor: Actor::LocalUser,
            schema_version: 1,
            expected_revision: 1,
            repository_identity: PathBuf::from("/repository"),
            common_directory_identity: PathBuf::from("/repository/.git"),
            destination: PathBuf::from("/private/worktrees/task"),
            branch: "rt/task-id".into(),
            base_expression: "HEAD".into(),
            resolved_commit: "01234567".into(),
            request_fingerprint: [7; 32],
            phase,
            reason,
            created_at: at(1),
            updated_at: at(1),
        })
        .unwrap()
    }

    fn content() -> TaskContent {
        TaskContent {
            title: "Task".into(),
            description: String::new(),
            priority: Priority::Normal,
            scope_paths: vec![],
            acceptance_notes: String::new(),
            dependency_ids: vec![],
        }
    }

    #[test]
    fn intent_transition_matrix_and_receipt_identity_are_strict() {
        for from in [
            WorktreePhase::Prepared,
            WorktreePhase::Applying,
            WorktreePhase::Ready,
            WorktreePhase::Failed,
            WorktreePhase::NeedsAttention,
        ] {
            for to in [
                WorktreePhase::Prepared,
                WorktreePhase::Applying,
                WorktreePhase::Ready,
                WorktreePhase::Failed,
                WorktreePhase::NeedsAttention,
            ] {
                let reason = matches!(to, WorktreePhase::Failed | WorktreePhase::NeedsAttention)
                    .then_some(WorktreeReason::OutcomeUncertain);
                let result = intent(
                    from,
                    matches!(from, WorktreePhase::Failed | WorktreePhase::NeedsAttention)
                        .then_some(WorktreeReason::OutcomeUncertain),
                )
                .transition(to, reason, at(2));
                assert_eq!(
                    result.is_ok(),
                    from == to || from.allows(to),
                    "{from:?} -> {to:?}"
                );
            }
        }
        assert!(
            intent(WorktreePhase::Prepared, None)
                .matches_fingerprint([7; 32])
                .is_ok()
        );
        assert_eq!(
            intent(WorktreePhase::Prepared, None).matches_fingerprint([8; 32]),
            Err(Error::Conflict)
        );
    }

    #[test]
    fn creation_batch_associates_atomically_and_rejects_cross_task_selection() {
        let workspace_id = id(2);
        let workspace = Workspace::restore(WorkspaceRecord {
            id: workspace_id,
            display_name: "Workspace".into(),
            project_root: PathBuf::from("/repository"),
            created_at: at(1),
            updated_at: at(1),
            schema_version: 1,
        })
        .unwrap();
        let state = WorkspaceState::new(workspace);
        let task_id = id(3);
        let changes = state
            .execute(
                Actor::LocalUser,
                Command::CreateTask {
                    id: task_id,
                    content: content(),
                },
                at(2),
            )
            .unwrap();
        let changes = changes
            .after()
            .execute(
                Actor::LocalUser,
                Command::Transition {
                    id: task_id,
                    to: TaskStatus::Ready,
                },
                at(3),
            )
            .unwrap();
        let root = ApprovedRoot::restore(ApprovedRootRecord {
            id: id(5),
            workspace_id,
            canonical_parent: PathBuf::from("/private/worktrees"),
            filesystem_identity: None,
            private_default: true,
            created_at: at(4),
        })
        .unwrap();
        let prepared = intent(WorktreePhase::Prepared, None);
        let changes = changes
            .after()
            .execute(
                Actor::LocalUser,
                Command::AddWorktreeIntent {
                    root,
                    intent: Box::new(prepared),
                },
                at(4),
            )
            .unwrap();
        let changes = changes
            .after()
            .execute(
                Actor::LocalUser,
                Command::MarkWorktreeApplying {
                    operation_id: id(1),
                },
                at(5),
            )
            .unwrap();
        let worktree = Worktree::restore(WorktreeRecord {
            id: id(4),
            workspace_id,
            task_id,
            operation_id: id(1),
            root_id: id(5),
            checkout_path: PathBuf::from("/private/worktrees/task"),
            common_directory_identity: PathBuf::from("/repository/.git"),
            branch_ref: "refs/heads/rt/task-id".into(),
            initial_base_commit: "01234567".into(),
            health: WorktreeHealth::Ready,
            created_at: at(4),
            updated_at: at(6),
        })
        .unwrap();
        let changes = changes
            .after()
            .execute(
                Actor::LocalUser,
                Command::FinalizeWorktree {
                    operation_id: id(1),
                    worktree,
                    select: true,
                },
                at(6),
            )
            .unwrap();
        assert_eq!(
            changes.after().task(task_id).unwrap().record().worktree_id,
            Some(id(4))
        );
        assert_eq!(
            changes.after().worktree_intents()[0].record().phase,
            WorktreePhase::Ready
        );
        assert_eq!(changes.payloads().len(), 3);
        let original = changes.after().clone();
        assert_eq!(
            original
                .execute(
                    Actor::LocalUser,
                    Command::SelectWorktree {
                        task_id,
                        worktree_id: Some(id(99))
                    },
                    at(7)
                )
                .err(),
            Some(Error::Reference)
        );
        assert_eq!(
            original.task(task_id).unwrap().record().worktree_id,
            Some(id(4))
        );
    }
}
