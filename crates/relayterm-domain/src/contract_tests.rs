use super::*;
use crate::validation::{paths, set, text};
use std::str::FromStr;

fn id<T: FromStr>(n: u64) -> T {
    format!("00000000-0000-4000-8000-{n:012x}")
        .parse()
        .ok()
        .unwrap()
}
fn at(n: i128) -> Timestamp {
    Timestamp::try_from(n).unwrap()
}
fn content() -> TaskContent {
    TaskContent {
        title: "Task".into(),
        description: "".into(),
        priority: Priority::Normal,
        scope_paths: vec![],
        acceptance_notes: "".into(),
        dependency_ids: vec![],
    }
}
fn definition() -> AgentDefinitionRecord {
    AgentDefinitionRecord {
        id: id(2),
        workspace_id: id(1),
        display_name: "Tool".into(),
        command: "tool".into(),
        arguments: vec![],
        environment_allowlist: vec![],
        capabilities: vec![],
        enabled: true,
    }
}
fn handover() -> HandoverContent {
    HandoverContent {
        summary: "Summary".into(),
        decisions: "".into(),
        changed_paths: vec![],
        verification_performed: "Not run: test fixture".into(),
        open_questions: "".into(),
        recommended_next_action: "Review".into(),
    }
}

#[test]
fn all_identifier_types_round_trip_and_are_distinct() {
    let types = [
        std::any::TypeId::of::<WorkspaceId>(),
        std::any::TypeId::of::<TaskId>(),
        std::any::TypeId::of::<AgentDefinitionId>(),
        std::any::TypeId::of::<AgentInstanceId>(),
        std::any::TypeId::of::<TerminalSessionId>(),
        std::any::TypeId::of::<ClaimId>(),
        std::any::TypeId::of::<ProgressEntryId>(),
        std::any::TypeId::of::<HandoverId>(),
        std::any::TypeId::of::<EventId>(),
        std::any::TypeId::of::<WorktreeId>(),
    ];
    assert_eq!(
        types
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        10
    );
    macro_rules! check { ($($ty:ty),*) => { $(
        let value: $ty = id(7); assert_eq!(value.to_string().parse::<$ty>().unwrap(), value);
        assert!("invalid".parse::<$ty>().is_err());
    )* }; }
    check!(
        WorkspaceId,
        TaskId,
        AgentDefinitionId,
        AgentInstanceId,
        TerminalSessionId,
        ClaimId,
        ProgressEntryId,
        HandoverId,
        EventId,
        WorktreeId
    );
}
#[test]
fn text_boundaries_utf8_controls_and_secrets_are_validated_without_echo() {
    for limit in [256, 4096, 8192, 16384] {
        assert!(text(&"a".repeat(limit), "field", limit, true, true).is_ok());
        assert!(text(&"a".repeat(limit + 1), "field", limit, true, true).is_err());
        assert!(text(&"é".repeat(limit / 2), "field", limit, true, true).is_ok());
        assert!(text(&"é".repeat(limit / 2 + 1), "field", limit, true, true).is_err());
    }
    for invalid in ["", " \t\n", "nul\0value", "escape\u{1b}", "control\u{7f}"] {
        assert!(text(invalid, "field", 256, true, true).is_err());
    }
    assert!(text("line\n\ttwo", "field", 256, true, true).is_ok());
    assert!(text("line\nsecond", "field", 256, true, false).is_err());
    assert!(text("tab\tsecond", "field", 256, true, false).is_err());
    let synthetic = ["ghp", "_", &"z".repeat(36)].concat();
    let error = text(&synthetic, "summary", 8192, true, true).unwrap_err();
    assert_eq!(error, Error::Validation("summary"));
    assert!(!format!("{error:?} {error}").contains(&synthetic));
}
#[test]
fn definition_list_and_command_limits_are_enforced_without_normalization() {
    let mut r = definition();
    r.arguments = vec![" a ".into(), " a ".into()];
    let d = AgentDefinition::restore(r.clone()).unwrap();
    assert_eq!(d.record().arguments, r.arguments);
    for command in ["", "\0", "\n"] {
        r.command = command.into();
        assert!(AgentDefinition::restore(r.clone()).is_err());
    }
    r.command = "tool".into();
    r.arguments = vec!["a".repeat(4096); 8];
    assert!(AgentDefinition::restore(r.clone()).is_ok());
    r.arguments.push("x".into());
    assert!(AgentDefinition::restore(r.clone()).is_err());
    r.arguments = vec!["x".into(); 128];
    assert!(AgentDefinition::restore(r.clone()).is_ok());
    r.arguments.push("x".into());
    assert!(AgentDefinition::restore(r.clone()).is_err());
    r.arguments = vec!["x".repeat(4097)];
    assert!(AgentDefinition::restore(r.clone()).is_err());
    r.arguments.clear();
    for invalid in ["KEY=value", "1NAME", "NAME SPACE", ""] {
        r.environment_allowlist = vec![invalid.into()];
        assert!(AgentDefinition::restore(r.clone()).is_err());
    }
    r.environment_allowlist = (0..128).map(|n| format!("VAR_{n}")).collect();
    assert!(AgentDefinition::restore(r.clone()).is_ok());
    r.environment_allowlist.push("VAR_0".into());
    assert!(AgentDefinition::restore(r.clone()).is_err());
    r.environment_allowlist.clear();
    r.capabilities = (0..128).map(|n| format!("capability_{n}")).collect();
    assert!(AgentDefinition::restore(r.clone()).is_ok());
    r.capabilities.push("extra".into());
    assert!(AgentDefinition::restore(r.clone()).is_err());
    r.capabilities = vec!["same".into(), "same".into()];
    assert!(AgentDefinition::restore(r).is_err());
}
#[test]
fn relative_paths_sets_and_dependency_limits_are_portable() {
    for invalid in [
        "/absolute",
        "C:\\absolute",
        "C:relative",
        "\\\\host\\share",
        "../escape",
        "src/../../escape",
        "src\\..\\escape",
        "",
        "nul\0",
    ] {
        assert!(paths(&[invalid.into()], "paths").is_err(), "{invalid:?}");
    }
    assert!(paths(&["src/lib.rs".into(), "src\\main.rs".into()], "paths").is_ok());
    assert!(paths(&["a".repeat(4096)], "paths").is_ok());
    assert!(paths(&["a".repeat(4097)], "paths").is_err());
    let mut values: Vec<_> = (0..128).map(|n| format!("src/{n}")).collect();
    assert!(paths(&values, "paths").is_ok());
    values.push("more".into());
    assert!(paths(&values, "paths").is_err());
    assert!(set(&[id::<TaskId>(1), id(1)], "dependency_ids").is_err());
    let mut c = content();
    c.dependency_ids = (1..=128).map(id).collect();
    assert!(c.validate().is_ok());
    c.dependency_ids.push(id(129));
    assert!(c.validate().is_err());
}
#[test]
fn handover_required_fields_individual_and_total_limits() {
    let mut h = handover();
    h.summary = "s".repeat(8192);
    h.decisions = "d".repeat(16384);
    h.open_questions = "q".repeat(16384);
    h.verification_performed = "v".repeat(8192);
    h.recommended_next_action = "n".repeat(8192);
    h.changed_paths = vec!["a".repeat(4096), "b".repeat(4096)];
    assert!(h.validate().is_ok());
    h.changed_paths.push("x".into());
    assert_eq!(h.validate(), Err(Error::Validation("handover")));
    for field in 0..5 {
        let mut h = handover();
        match field {
            0 => h.summary = "".into(),
            1 => h.verification_performed = "".into(),
            2 => h.recommended_next_action = "".into(),
            3 => h.decisions = "d".repeat(16385),
            _ => h.open_questions = "q".repeat(16385),
        }
        assert!(h.validate().is_err());
    }
}
#[test]
fn reconstruction_rejects_invalid_records_and_temporal_sequences() {
    let workspace = WorkspaceRecord {
        id: id(1),
        display_name: "Workspace".into(),
        project_root: "project".into(),
        created_at: at(10),
        updated_at: at(20),
        schema_version: 1,
    };
    let mut bad = workspace.clone();
    bad.updated_at = at(9);
    assert!(Workspace::restore(bad).is_err());
    let mut bad = workspace.clone();
    bad.schema_version = 2;
    assert!(matches!(Workspace::restore(bad), Err(Error::Version)));
    let mut bad = workspace.clone();
    bad.project_root = "project/../outside".into();
    assert!(Workspace::restore(bad).is_err());
    let task = TaskRecord {
        id: id(2),
        workspace_id: id(1),
        content: content(),
        status: TaskStatus::Backlog,
        claimed_by_instance_id: None,
        worktree_id: None,
        created_at: at(10),
        updated_at: at(20),
    };
    let mut bad = task.clone();
    bad.status = TaskStatus::Active;
    assert!(Task::restore(bad).is_err());
    let mut bad = task.clone();
    bad.worktree_id = Some(id(3));
    assert!(Task::restore(bad).is_err());
    let mut bad = task.clone();
    bad.content.dependency_ids = vec![id(2)];
    assert!(Task::restore(bad).is_err());
    let mut bad = task.clone();
    bad.content.dependency_ids = vec![id(99)];
    assert!(
        WorkspaceState::restore(
            Workspace::restore(workspace.clone()).unwrap(),
            WorkspaceRows {
                tasks: vec![Task::restore(bad).unwrap()],
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut foreign = task.clone();
    foreign.workspace_id = id(99);
    assert!(
        WorkspaceState::restore(
            Workspace::restore(workspace.clone()).unwrap(),
            WorkspaceRows {
                tasks: vec![Task::restore(foreign).unwrap()],
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut active = task.clone();
    active.status = TaskStatus::Active;
    active.claimed_by_instance_id = Some(id(7));
    assert!(
        WorkspaceState::restore(
            Workspace::restore(workspace.clone()).unwrap(),
            WorkspaceRows {
                tasks: vec![Task::restore(active).unwrap()],
                ..Default::default()
            }
        )
        .is_err()
    );
    let task = Task::restore(task).unwrap();
    assert!(
        WorkspaceState::restore(
            Workspace::restore(workspace).unwrap(),
            WorkspaceRows {
                tasks: vec![task.clone(), task],
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(Timestamp::try_from(i128::MAX).is_err());
    let offset = time::UtcOffset::from_hms(2, 0, 0).unwrap();
    assert_eq!(
        Timestamp::new(at(0).value().to_offset(offset)).unwrap(),
        at(0)
    );
}
#[test]
fn claim_closure_reconstruction_and_dimensions_are_checked() {
    for dimensions in [(0, 80), (24, 0), (1001, 80), (24, 1001)] {
        assert!(TerminalSize::new(dimensions.0, dimensions.1).is_err());
    }
    assert_eq!(TerminalSize::new(1, 1000).unwrap().columns(), 1000);
    let record = ClaimRecord {
        id: id(1),
        workspace_id: id(2),
        task_id: id(3),
        instance_id: id(4),
        requested_by: Actor::Instance(id(4)),
        opened_at: at(10),
        closed_at: None,
        close_reason: None,
        closed_by: None,
    };
    let mut claim = Claim::restore(record.clone()).unwrap();
    let original = claim.clone();
    assert!(
        claim
            .close(CloseReason::ExplicitRelease, Actor::Instance(id(4)), at(9))
            .is_err()
    );
    assert!(claim == original);
    claim
        .close(CloseReason::ExplicitRelease, Actor::Instance(id(4)), at(11))
        .unwrap();
    let closed = claim.clone();
    assert!(
        claim
            .close(CloseReason::Completion, Actor::LocalUser, at(12))
            .is_err()
    );
    assert!(claim == closed);
    let mut invalid = record.clone();
    invalid.closed_at = Some(at(11));
    assert!(Claim::restore(invalid).is_err());
    let mut invalid = record;
    invalid.requested_by = Actor::System;
    assert!(Claim::restore(invalid).is_err());
}
#[test]
fn instance_metadata_and_observation_times_are_validated() {
    let record = AgentInstanceRecord {
        id: id(1),
        session_id: id(2),
        workspace_id: id(3),
        agent_definition_id: None,
        task_id: None,
        launch_definition: None,
        working_directory: "project".into(),
        status: InstanceStatus::Starting,
        started_at: at(10),
        last_observed_at: at(10),
        ended_at: None,
        exit_code: None,
        terminal_size: TerminalSize::new(24, 80).unwrap(),
    };
    let mut instance = AgentInstance::restore(record.clone()).unwrap();
    assert!(
        instance
            .observe(InstanceStatus::Failed, at(11), Some(1))
            .is_err()
    );
    instance
        .observe(InstanceStatus::Running, at(20), None)
        .unwrap();
    assert!(
        instance
            .observe(InstanceStatus::Exited, at(19), Some(0))
            .is_err()
    );
    instance
        .observe(InstanceStatus::Exited, at(21), Some(4))
        .unwrap();
    assert!(
        !instance
            .observe(InstanceStatus::Exited, at(21), Some(4))
            .unwrap()
    );
    assert!(
        instance
            .observe(InstanceStatus::Exited, at(21), None)
            .is_err()
    );
    let mut failed = AgentInstance::restore(record.clone()).unwrap();
    failed
        .observe(InstanceStatus::Failed, at(11), None)
        .unwrap();
    assert_eq!(failed.record().exit_code, None);
    let mut invalid = record;
    invalid.status = InstanceStatus::Lost;
    assert!(AgentInstance::restore(invalid).is_err());
}
#[test]
fn every_event_variant_round_trips_with_strict_versions_and_safe_errors() {
    let payloads = vec![
        EventPayload::WorkspaceCreated { id: id(1) },
        EventPayload::DefinitionCreated { id: id(2) },
        EventPayload::DefinitionUpdated {
            id: id(2),
            fields: vec![DefinitionField::Command, DefinitionField::Enabled],
        },
        EventPayload::TaskCreated { id: id(3) },
        EventPayload::TaskEdited {
            id: id(3),
            fields: vec![TaskField::Description, TaskField::ScopePaths],
        },
        EventPayload::TaskTransitioned {
            id: id(3),
            from: TaskStatus::Ready,
            to: TaskStatus::Active,
        },
        EventPayload::InstanceRegistered {
            id: id(4),
            session_id: id(5),
        },
        EventPayload::InstanceObserved {
            id: id(4),
            from: InstanceStatus::Running,
            to: InstanceStatus::Lost,
        },
        EventPayload::ClaimOpened {
            id: id(6),
            task_id: id(3),
            instance_id: id(4),
        },
        EventPayload::ClaimClosed {
            id: id(6),
            task_id: id(3),
            instance_id: id(4),
            reason: CloseReason::InstanceEnd,
        },
        EventPayload::ProgressAdded {
            id: id(7),
            task_id: id(3),
        },
        EventPayload::HandoverPrepared {
            id: id(8),
            task_id: id(3),
        },
    ];
    for (n, payload) in payloads.into_iter().enumerate() {
        let pending = PendingEvent {
            event_id: id(n as u64 + 20),
            workspace_id: id(1),
            timestamp: at(10),
            actor: Actor::LocalUser,
            payload,
        };
        assert!(WorkspaceEvent::confirm(pending.clone(), 0).is_err());
        let event = WorkspaceEvent::confirm(pending, n as u64 + 1).unwrap();
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(
            serde_json::from_str::<WorkspaceEvent>(&json).unwrap(),
            event
        );
        let value = serde_json::to_value(&event).unwrap();
        for field in [
            "sequence",
            "event_id",
            "workspace_id",
            "event_type",
            "entity_id",
            "timestamp",
            "payload_version",
            "payload",
            "actor",
        ] {
            assert!(value.get(field).is_some());
        }
        let mut invalid = value.clone();
        invalid["payload_version"] = 2.into();
        assert!(serde_json::from_value::<WorkspaceEvent>(invalid).is_err());
        let mut invalid = value.clone();
        invalid["payload"]["kind"] = "SENSITIVE_UNKNOWN_VARIANT".into();
        let error = serde_json::from_value::<WorkspaceEvent>(invalid).unwrap_err();
        assert!(!error.to_string().contains("SENSITIVE"));
        let mut invalid = value;
        invalid["payload"]["SENSITIVE_UNKNOWN_FIELD"] = "private input".into();
        let error = serde_json::from_value::<WorkspaceEvent>(invalid).unwrap_err();
        assert!(!error.to_string().contains("SENSITIVE"));
        assert!(!error.to_string().contains("private input"));
    }
}

#[test]
fn environment_name_comparison_follows_native_platform_semantics() {
    let mut record = definition();
    record.environment_allowlist = vec!["PATH".into(), "path".into()];
    assert_eq!(AgentDefinition::restore(record).is_err(), cfg!(windows));
}
