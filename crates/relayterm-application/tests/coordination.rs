mod support;
use domain::*;
use relayterm_application::*;
use std::{
    future::Future,
    pin::pin,
    sync::atomic::Ordering,
    task::{Context, Waker},
};
use support::*;

#[test]
fn continuity_gate_preserves_context_attribution_and_event_order() {
    let f = Fixture::new();
    f.run(
        Actor::LocalUser,
        Request::AddDefinition {
            display_name: "Neutral tool".into(),
            command: "synthetic-tool".into(),
            arguments: vec!["--interactive".into()],
            environment_allowlist: vec!["LANG".into()],
            capabilities: vec!["interactive".into()],
            enabled: true,
        },
    )
    .unwrap();
    let definition = f.state().definitions()[0].record().id;
    let first = f.instance(Some(definition));
    let second = f.instance(None);
    let task = f.task();
    f.ready(task);
    f.claim(task, first).unwrap();
    assert!(f.claim(task, second).is_err());
    f.run(
        Actor::Instance(first),
        Request::Progress {
            task_id: task,
            summary: "Synthetic private progress".into(),
            verification: "Tests passed".into(),
        },
    )
    .unwrap();
    let outcome = f
        .run(
            Actor::Instance(first),
            Request::Handover {
                task_id: task,
                content: handover(),
            },
        )
        .unwrap();
    assert_eq!(outcome.committed.events.len(), 3);
    assert_eq!(
        f.state().task(task).unwrap().record().status,
        TaskStatus::HandoverReady
    );
    assert!(f.state().current_claim(task).is_none());
    f.claim(task, second).unwrap();
    let history = block_on(f.service.history(f.workspace, task, Page::default())).unwrap();
    assert_eq!(
        history.progress[0].record().summary,
        "Synthetic private progress"
    );
    assert_eq!(
        history.handovers[0]
            .record()
            .content
            .recommended_next_action,
        "Review the tests"
    );
    f.run(
        Actor::Instance(second),
        Request::Transition {
            id: task,
            to: TaskStatus::Done,
        },
    )
    .unwrap();
    f.run(
        Actor::LocalUser,
        Request::Progress {
            task_id: task,
            summary: "Historical correction".into(),
            verification: "Not run: documentation".into(),
        },
    )
    .unwrap();
    let state = f.state();
    assert_eq!(state.task(task).unwrap().record().status, TaskStatus::Done);
    assert_eq!(state.claims().len(), 2);
    assert_eq!(
        state.claims()[0].record().close_reason,
        Some(CloseReason::Handover)
    );
    assert_eq!(
        state.claims()[1].record().close_reason,
        Some(CloseReason::Completion)
    );
    assert_eq!(state.progress()[1].record().agent_instance_id, None);
    for (n, event) in f.events().iter().enumerate() {
        assert_eq!(event.record().sequence, n as u64 + 1);
    }
    let serialized = serde_json::to_string(&f.events()).unwrap();
    for marker in [
        "Synthetic private",
        "synthetic-tool",
        "--interactive",
        "src/lib.rs",
        "Review the tests",
        "LANG",
    ] {
        assert!(!serialized.contains(marker));
    }
}

#[test]
fn recovery_gate_is_atomic_and_final_observations_are_idempotent() {
    let f = Fixture::new();
    let first = f.instance(None);
    let second = f.instance(None);
    let task = f.task();
    let unrelated = f.task();
    f.ready(task);
    f.claim(task, first).unwrap();
    let outcome = f.observe(first, InstanceStatus::Lost, None).unwrap();
    assert_eq!(outcome.committed.events.len(), 3);
    let state = f.state();
    let events = f.events();
    assert_eq!(
        state.task(task).unwrap().record().status,
        TaskStatus::Blocked
    );
    assert_eq!(
        state.task(unrelated).unwrap().record().status,
        TaskStatus::Backlog
    );
    assert_eq!(
        state.claims()[0].record().close_reason,
        Some(CloseReason::InstanceEnd)
    );
    assert_eq!(state.claims()[0].record().closed_by, Some(Actor::System));
    let calls = f.notifier.calls.lock().unwrap().len();
    let repeated = f.observe(first, InstanceStatus::Lost, None).unwrap();
    assert!(repeated.committed.events.is_empty());
    assert!(f.state() == state);
    assert_eq!(f.events(), events);
    assert_eq!(calls, f.notifier.calls.lock().unwrap().len());
    assert!(f.observe(first, InstanceStatus::Exited, Some(0)).is_err());
    f.clock.0.store(99, Ordering::SeqCst);
    let repeated = block_on(f.service.observe(
        f.workspace,
        Observation::Status {
            id: first,
            status: InstanceStatus::Lost,
            observed_at: Timestamp::try_from(100).unwrap(),
            exit_code: None,
        },
    ))
    .unwrap();
    assert!(repeated.committed.events.is_empty());
    assert!(f.state() == state);
    assert_eq!(f.events(), events);
    f.clock.0.store(100, Ordering::SeqCst);
    f.ready(task);
    f.claim(task, second).unwrap();
    assert_eq!(
        f.state()
            .task(task)
            .unwrap()
            .record()
            .claimed_by_instance_id,
        Some(second)
    );
}

fn reach_task(f: &Fixture, id: TaskId, instance: AgentInstanceId, status: TaskStatus) {
    if status == TaskStatus::Backlog {
        return;
    }
    f.ready(id);
    if status == TaskStatus::Ready {
        return;
    }
    f.claim(id, instance).unwrap();
    match status {
        TaskStatus::Active => {}
        TaskStatus::HandoverReady => {
            f.run(
                Actor::LocalUser,
                Request::Handover {
                    task_id: id,
                    content: handover(),
                },
            )
            .unwrap();
        }
        _ => {
            f.run(Actor::LocalUser, Request::Transition { id, to: status })
                .unwrap();
        }
    }
}
#[test]
fn all_49_task_pairs_use_real_preconditions_and_preserve_rejected_state() {
    let allowed = [
        (0, 1),
        (0, 6),
        (1, 2),
        (1, 6),
        (2, 3),
        (2, 4),
        (2, 5),
        (2, 6),
        (3, 1),
        (3, 6),
        (4, 2),
        (4, 6),
    ];
    for (a, from) in TaskStatus::ALL.into_iter().enumerate() {
        for (b, to) in TaskStatus::ALL.into_iter().enumerate() {
            let f = Fixture::new();
            let instance = f.instance(None);
            let task = f.task();
            reach_task(&f, task, instance, from);
            let state = f.state();
            let events = f.events();
            let request = match to {
                TaskStatus::Active => Request::Claim {
                    task_id: task,
                    instance_id: instance,
                },
                TaskStatus::HandoverReady => Request::Handover {
                    task_id: task,
                    content: handover(),
                },
                _ => Request::Transition { id: task, to },
            };
            let result = f.run(Actor::LocalUser, request);
            assert_eq!(
                result.is_ok(),
                allowed.contains(&(a, b)),
                "{from:?} -> {to:?}"
            );
            if result.is_err() {
                assert!(f.state() == state);
                assert_eq!(f.events(), events);
            }
        }
    }
}
#[test]
fn all_36_instance_pairs_and_exit_metadata() {
    let allowed = [
        (0, 1),
        (0, 3),
        (0, 4),
        (0, 5),
        (1, 2),
        (1, 3),
        (1, 4),
        (1, 5),
    ];
    for (a, from) in InstanceStatus::ALL.into_iter().enumerate() {
        for (b, to) in InstanceStatus::ALL.into_iter().enumerate() {
            let f = Fixture::new();
            let outcome = block_on(f.service.register_instance(
                f.workspace,
                LaunchContext {
                    agent_definition_id: None,
                    task_id: None,
                    working_directory: "project".into(),
                    terminal_size: TerminalSize::new(1, 1000).unwrap(),
                },
            ))
            .unwrap();
            let id = outcome.committed.snapshot.state().unwrap().instances()[0]
                .record()
                .id;
            if from != InstanceStatus::Starting {
                f.observe(id, InstanceStatus::Running, None).unwrap();
                if from != InstanceStatus::Running {
                    f.observe(id, from, None).unwrap();
                }
            }
            let state = f.state();
            let events = f.events();
            f.clock.0.store(101, Ordering::SeqCst);
            let result = f.observe(id, to, None);
            assert_eq!(
                result.is_ok(),
                allowed.contains(&(a, b)),
                "{from:?} -> {to:?}"
            );
            if result.is_err() {
                assert!(f.state() == state);
                assert_eq!(f.events(), events);
            }
        }
    }
    let f = Fixture::new();
    let id = f.instance(None);
    f.observe(id, InstanceStatus::Exited, Some(7)).unwrap();
    assert_eq!(f.state().instance(id).unwrap().record().exit_code, Some(7));
    let other = f.instance(None);
    assert!(f.observe(other, InstanceStatus::Lost, Some(1)).is_err());
}

#[test]
fn claims_authorization_release_and_administrative_intervention() {
    let f = Fixture::new();
    let first = f.instance(None);
    let other = f.instance(None);
    let task = f.task();
    let second_task = f.task();
    f.ready(task);
    f.ready(second_task);
    f.run(
        Actor::LocalUser,
        Request::Claim {
            task_id: task,
            instance_id: first,
        },
    )
    .unwrap();
    assert_eq!(
        f.state().claims()[0].record().requested_by,
        Actor::LocalUser
    );
    let state = f.state();
    let events = f.events();
    assert!(f.claim(task, first).is_err());
    assert!(f.claim(task, other).is_err());
    assert!(f.claim(second_task, first).is_err());
    for request in [
        Request::Release { task_id: task },
        Request::Transition {
            id: task,
            to: TaskStatus::Done,
        },
        Request::Handover {
            task_id: task,
            content: handover(),
        },
        Request::Progress {
            task_id: task,
            summary: "Invalid owner".into(),
            verification: "".into(),
        },
        Request::EditTask {
            id: task,
            content: content(),
        },
    ] {
        assert!(f.run(Actor::Instance(other), request).is_err());
    }
    assert!(
        f.run(
            Actor::System,
            Request::Transition {
                id: task,
                to: TaskStatus::Done
            }
        )
        .is_err()
    );
    assert!(
        f.run(
            Actor::LocalUser,
            Request::Transition {
                id: task,
                to: TaskStatus::Ready
            }
        )
        .is_err()
    );
    assert!(
        f.run(
            Actor::LocalUser,
            Request::Release {
                task_id: second_task
            }
        )
        .is_err()
    );
    assert!(f.state() == state);
    assert_eq!(f.events(), events);
    f.run(Actor::LocalUser, Request::Release { task_id: task })
        .unwrap();
    assert_eq!(
        f.state().claims()[0].record().closed_by,
        Some(Actor::LocalUser)
    );
    f.ready(task);
    f.claim(task, other).unwrap();
    f.run(
        Actor::LocalUser,
        Request::Handover {
            task_id: task,
            content: handover(),
        },
    )
    .unwrap();
    assert_eq!(
        f.state().handovers()[0].record().from_agent_instance_id,
        None
    );
    assert_eq!(
        f.state().claims()[1].record().closed_by,
        Some(Actor::LocalUser)
    );
    assert_eq!(f.state().claims().len(), 2);
}

#[test]
fn dependencies_are_informational_cycles_are_allowed_and_foreign_references_fail() {
    let f = Fixture::new();
    let first = f.task();
    let second = f.task();
    let mut c = content();
    c.dependency_ids = vec![second];
    f.run(
        Actor::LocalUser,
        Request::EditTask {
            id: first,
            content: c,
        },
    )
    .unwrap();
    let mut c = content();
    c.dependency_ids = vec![first];
    f.run(
        Actor::LocalUser,
        Request::EditTask {
            id: second,
            content: c,
        },
    )
    .unwrap();
    f.ready(first);
    let instance = f.instance(None);
    f.claim(first, instance).unwrap();
    f.run(
        Actor::LocalUser,
        Request::Transition {
            id: second,
            to: TaskStatus::Cancelled,
        },
    )
    .unwrap();
    assert_eq!(
        f.state().task(first).unwrap().record().status,
        TaskStatus::Active
    );
    let foreign = block_on(f.service.create_workspace("Other".into(), "other".into())).unwrap();
    let foreign_id = foreign
        .committed
        .snapshot
        .state()
        .unwrap()
        .workspace()
        .record()
        .id;
    let foreign_task = block_on(f.service.execute(
        foreign_id,
        Actor::LocalUser,
        Request::CreateTask(content()),
    ))
    .unwrap()
    .committed
    .snapshot
    .state()
    .unwrap()
    .tasks()[0]
        .record()
        .id;
    for deps in [vec![first], vec![second, second], vec![foreign_task]] {
        let mut c = content();
        c.dependency_ids = deps;
        assert!(
            f.run(
                Actor::LocalUser,
                Request::EditTask {
                    id: first,
                    content: c
                }
            )
            .is_err()
        );
    }
    let missing: AgentDefinitionId = "00000000-0000-4000-8000-000000000999".parse().unwrap();
    assert!(
        block_on(f.service.register_instance(
            f.workspace,
            LaunchContext {
                agent_definition_id: Some(missing),
                task_id: None,
                working_directory: "project".into(),
                terminal_size: TerminalSize::new(20, 80).unwrap()
            }
        ))
        .is_err()
    );
}

#[test]
fn two_transactions_race_without_sleeps_and_only_one_claim_commits() {
    let f = Fixture::new();
    let first = f.instance(None);
    let second = f.instance(None);
    let task = f.task();
    f.ready(task);
    let before = f.events().len();
    let mut a = pin!(f.service.execute(
        f.workspace,
        Actor::Instance(first),
        Request::Claim {
            task_id: task,
            instance_id: first
        }
    ));
    let mut b = pin!(f.service.execute(
        f.workspace,
        Actor::Instance(second),
        Request::Claim {
            task_id: task,
            instance_id: second
        }
    ));
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    assert!(a.as_mut().poll(&mut cx).is_pending());
    assert!(b.as_mut().poll(&mut cx).is_pending());
    block_on(a).unwrap();
    assert!(matches!(block_on(b), Err(Error::Conflict)));
    assert_eq!(f.state().claims().len(), 1);
    assert_eq!(f.events().len(), before + 2);
}

#[test]
fn failures_rollback_handover_and_loss_and_notifier_failure_keeps_commit() {
    let f = Fixture::new();
    let instance = f.instance(None);
    let task = f.task();
    f.ready(task);
    f.claim(task, instance).unwrap();
    let state = f.state();
    let events = f.events();
    f.memory.inner.lock().unwrap().fail_validation = true;
    assert!(
        f.run(
            Actor::Instance(instance),
            Request::Handover {
                task_id: task,
                content: handover()
            }
        )
        .is_err()
    );
    assert!(f.observe(instance, InstanceStatus::Lost, None).is_err());
    assert!(f.state() == state);
    assert_eq!(f.events(), events);
    f.memory.inner.lock().unwrap().fail_validation = false;
    f.memory.inner.lock().unwrap().fail_before = true;
    assert!(
        f.run(
            Actor::Instance(instance),
            Request::Release { task_id: task }
        )
        .is_err()
    );
    f.memory.inner.lock().unwrap().fail_before = false;
    assert!(f.state() == state);
    assert_eq!(f.events(), events);
    f.notifier.fail.store(true, Ordering::SeqCst);
    let outcome = f
        .run(
            Actor::Instance(instance),
            Request::Handover {
                task_id: task,
                content: handover(),
            },
        )
        .unwrap();
    assert!(!outcome.notification_delivered);
    assert_eq!(f.state().handovers().len(), 1);
    assert_eq!(f.events().len(), events.len() + 3);
}

#[test]
fn backward_clock_rejected_without_writes_and_reconciliation_is_repeatable() {
    let f = Fixture::new();
    let first = f.instance(None);
    let second = f.instance(None);
    let task = f.task();
    f.ready(task);
    f.claim(task, first).unwrap();
    let state = f.state();
    let events = f.events();
    f.clock.0.store(99, Ordering::SeqCst);
    assert!(matches!(
        f.run(Actor::LocalUser, Request::Release { task_id: task }),
        Err(Error::Time)
    ));
    assert!(f.observe(first, InstanceStatus::Lost, None).is_err());
    assert!(f.state() == state);
    assert_eq!(f.events(), events);
    f.clock.0.store(101, Ordering::SeqCst);
    block_on(f.service.observe(f.workspace, Observation::ReconcileLost)).unwrap();
    assert_eq!(
        f.state().instance(second).unwrap().record().status,
        InstanceStatus::Lost
    );
    assert_eq!(
        f.state().task(task).unwrap().record().status,
        TaskStatus::Blocked
    );
    let events = f.events();
    block_on(f.service.observe(f.workspace, Observation::ReconcileLost)).unwrap();
    assert_eq!(f.events(), events);
}

#[test]
fn pagination_and_final_task_rules_preserve_append_only_history() {
    let f = Fixture::new();
    let task = f.task();
    f.run(
        Actor::LocalUser,
        Request::Transition {
            id: task,
            to: TaskStatus::Cancelled,
        },
    )
    .unwrap();
    assert!(
        f.run(
            Actor::LocalUser,
            Request::EditTask {
                id: task,
                content: content()
            }
        )
        .is_err()
    );
    for n in 0..51 {
        f.run(
            Actor::LocalUser,
            Request::Progress {
                task_id: task,
                summary: format!("Correction {n}"),
                verification: "".into(),
            },
        )
        .unwrap();
    }
    let first = block_on(f.service.history(f.workspace, task, Page::default())).unwrap();
    assert_eq!(first.progress.len(), 50);
    let second = block_on(
        f.service
            .history(f.workspace, task, Page::new(50, 200).unwrap()),
    )
    .unwrap();
    assert_eq!(second.progress.len(), 1);
    assert_eq!(second.progress[0].record().summary, "Correction 50");
    assert!(Page::new(0, 0).is_err());
    assert!(Page::new(0, 201).is_err());
}

#[test]
fn duplicate_event_ids_abort_before_any_state_or_history_write() {
    struct FixedIds(EventId);
    impl IdGenerator for FixedIds {
        fn next(&self) -> Result<EventId> {
            Ok(self.0)
        }
    }
    let f = Fixture::new();
    let instance = f.instance(None);
    let task = f.task();
    f.ready(task);
    f.claim(task, instance).unwrap();
    let events = f.events();
    let state = f.state();
    let service = Service::new(
        f.memory.clone(),
        f.clock.clone(),
        FixedIds(events[0].record().event_id),
        f.notifier.clone(),
    );
    assert!(matches!(
        block_on(service.execute(
            f.workspace,
            Actor::LocalUser,
            Request::Handover {
                task_id: task,
                content: handover()
            }
        )),
        Err(Error::Conflict)
    ));
    assert!(f.state() == state);
    assert_eq!(f.events(), events);
}

#[test]
fn reconstruction_checks_history_and_round_trips_committed_entities() {
    fn rows(state: &WorkspaceState) -> WorkspaceRows {
        WorkspaceRows {
            definitions: state.definitions().to_vec(),
            tasks: state.tasks().to_vec(),
            instances: state.instances().to_vec(),
            claims: state.claims().to_vec(),
            progress: state.progress().to_vec(),
            handovers: state.handovers().to_vec(),
        }
    }
    let f = Fixture::new();
    let first = f.instance(None);
    let second = f.instance(None);
    let task = f.task();
    f.ready(task);
    f.claim(task, first).unwrap();
    f.run(
        Actor::Instance(first),
        Request::Handover {
            task_id: task,
            content: handover(),
        },
    )
    .unwrap();
    f.claim(task, second).unwrap();
    f.run(
        Actor::Instance(second),
        Request::Transition {
            id: task,
            to: TaskStatus::Done,
        },
    )
    .unwrap();
    let state = f.state();
    assert!(WorkspaceState::restore(state.workspace().clone(), rows(&state)).unwrap() == state);
    let mut invalid = rows(&state);
    invalid.handovers.clear();
    assert!(WorkspaceState::restore(state.workspace().clone(), invalid).is_err());
    let mut invalid = rows(&state);
    invalid.claims.clear();
    assert!(WorkspaceState::restore(state.workspace().clone(), invalid).is_err());
    let mut invalid = rows(&state);
    invalid.instances.push(state.instances()[0].clone());
    assert!(WorkspaceState::restore(state.workspace().clone(), invalid).is_err());
    let mut record = state.claims()[0].record().clone();
    record.closed_by = Some(Actor::Instance(second));
    assert!(Claim::restore(record).is_err());
}

#[test]
fn generic_transitions_cannot_bypass_claim_or_handover_and_disabled_definitions_cannot_launch() {
    let f = Fixture::new();
    let instance = f.instance(None);
    let task = f.task();
    f.ready(task);
    assert!(
        f.run(
            Actor::LocalUser,
            Request::Transition {
                id: task,
                to: TaskStatus::Active
            }
        )
        .is_err()
    );
    f.claim(task, instance).unwrap();
    assert!(
        f.run(
            Actor::LocalUser,
            Request::Transition {
                id: task,
                to: TaskStatus::HandoverReady
            }
        )
        .is_err()
    );
    let mut edited = content();
    edited.title = "New title".into();
    f.run(
        Actor::Instance(instance),
        Request::EditTask {
            id: task,
            content: edited,
        },
    )
    .unwrap();
    assert_eq!(
        f.state()
            .task(task)
            .unwrap()
            .record()
            .claimed_by_instance_id,
        Some(instance)
    );
    assert_eq!(
        f.state().task(task).unwrap().record().status,
        TaskStatus::Active
    );
    f.run(
        Actor::LocalUser,
        Request::AddDefinition {
            display_name: "Disabled".into(),
            command: "tool".into(),
            arguments: vec![],
            environment_allowlist: vec![],
            capabilities: vec![],
            enabled: false,
        },
    )
    .unwrap();
    let definition = f.state().definitions()[0].record().id;
    assert!(matches!(
        block_on(f.service.register_instance(
            f.workspace,
            LaunchContext {
                agent_definition_id: Some(definition),
                task_id: None,
                working_directory: "project".into(),
                terminal_size: TerminalSize::new(24, 80).unwrap()
            }
        )),
        Err(Error::Unavailable)
    ));
}
