//! Isolated tests for the current experimental TME Clockwork implementation.
//! No disk persistence, network, clock, wallet, Academy or CHIMERA state is used.
use transformer_memory_engine::{
    memory_levels::MemoryLevel,
    retention::{RecallOutcome, RetentionEvent},
    retention_dynamics::RetentionPolicy,
    token_architecture::{TokenInertia, TokenKind},
    Memory, StateToken,
};

fn memory(id: &str, stability: f32, retention: f32, inertia_s: f32, inertia_r: f32) -> Memory {
    let mut m = Memory::new();
    m.insert(StateToken {
        id: id.into(),
        vector: vec![1.0, 0.0],
    })
    .unwrap();
    m.set_token_kind(id, TokenKind::Memory).unwrap();
    m.set_token_dynamics(id, 0.5, stability).unwrap();
    m.set_retention_estimate(id, retention).unwrap();
    m.set_token_inertia(
        id,
        TokenInertia {
            activation: 0.0,
            stability: inertia_s,
            retention: inertia_r,
        },
    )
    .unwrap();
    m
}

fn sample(m: &Memory, id: &str, label: &str) {
    let s = m.dynamics(id).unwrap().stability;
    let r = m.retention_estimate(id).unwrap();
    println!(
        "{label:<24} stability={s:.5} retention={r:.5} mean={:.5} level={:?}",
        (s + r) / 2.0,
        m.memory_level(id).unwrap()
    );
}

#[test]
fn seventy_inscriptions_show_qualified_recall_but_do_not_invent_stability() {
    let id = "inscribe-experience";
    let mut m = memory(id, 0.01, 0.75, 0.30, 0.70);
    m.set_memory_l3_inertia_bonus(0.5).unwrap();
    let policy = RetentionPolicy {
        minimum_gap_ms: 12 * 60 * 60 * 1000,
        adjustment_rate: 0.20,
    };
    sample(&m, id, "initial");
    let mut qualified = 0;
    for day in 0..14_i64 {
        for session in 0..5_i64 {
            let number = day * 5 + session;
            let event = RetentionEvent {
                id: format!("inscribe-recall-{number}"),
                token_id: id.into(),
                attempted_at_unix_ms: day * 86_400_000 + session * 60_000,
                outcome: RecallOutcome::Success,
                reference: format!("clockwork/synthetic/day-{day}/item-{session}"),
            };
            let result = m.apply_retention_event(event, policy).unwrap();
            if result.qualified {
                qualified += 1;
            }
        }
        if [0, 6, 13].contains(&day) {
            sample(&m, id, &format!("after day {}", day + 1));
        }
    }
    println!("70 events, {qualified} qualified gaps");
    assert_eq!(m.retention_events_for(id).unwrap().len(), 70);
    assert_eq!(
        qualified, 13,
        "Five same-day repetitions must not count as five long-gap successes"
    );
    assert!(m.retention_estimate(id).unwrap() > 0.75);
    assert_eq!(
        m.dynamics(id).unwrap().stability,
        0.01,
        "Existing retention events must not silently change stability"
    );
    // A threshold crossing is not promised: this measures the actual outcome.
}

#[test]
fn six_months_without_events_do_not_yet_cause_passive_decay() {
    let id = "markdown-memory";
    let m = memory(id, 0.85, 0.85, 0.25, 0.25);
    let before = (
        m.dynamics(id).unwrap().stability,
        m.retention_estimate(id).unwrap(),
        m.memory_level(id),
    );
    let _simulated_day_180_unix_ms: i64 = 180 * 86_400_000;
    let after = (
        m.dynamics(id).unwrap().stability,
        m.retention_estimate(id).unwrap(),
        m.memory_level(id),
    );
    sample(&m, id, "after 180 days idle");
    assert_eq!(
        before, after,
        "No elapsed-time decay should be claimed without a real update mechanism"
    );
    assert_eq!(m.memory_level(id), Some(MemoryLevel::L1));
}

#[test]
fn anchor_twins_dampen_both_directions_without_an_extra_update_rule() {
    let mut ordinary = memory("normal", 0.30, 0.30, 0.20, 0.20);
    let mut anchor = memory("anchor", 0.30, 0.30, 0.90, 0.95);
    anchor.set_anchor("anchor", true).unwrap();
    assert_eq!(anchor.is_anchor("anchor"), Some(true));
    ordinary.apply_stability_target("normal", 0.90).unwrap();
    anchor.apply_stability_target("anchor", 0.90).unwrap();
    let policy = RetentionPolicy {
        minimum_gap_ms: 1000,
        adjustment_rate: 1.0,
    };
    for (m, id) in [(&mut ordinary, "normal"), (&mut anchor, "anchor")] {
        for (i, t) in [100_i64, 2100].iter().enumerate() {
            m.apply_retention_event(
                RetentionEvent {
                    id: format!("{id}-{i}"),
                    token_id: id.into(),
                    attempted_at_unix_ms: *t,
                    outcome: RecallOutcome::Success,
                    reference: "clockwork/synthetic".into(),
                },
                policy,
            )
            .unwrap();
        }
    }
    sample(&ordinary, "normal", "ordinary after success");
    sample(&anchor, "anchor", "anchor after success");
    assert!(
        anchor.dynamics("anchor").unwrap().stability
            < ordinary.dynamics("normal").unwrap().stability
    );
    assert!(
        anchor.retention_estimate("anchor").unwrap()
            < ordinary.retention_estimate("normal").unwrap()
    );
    let ordinary_before = ordinary.dynamics("normal").unwrap().stability;
    let anchor_before = anchor.dynamics("anchor").unwrap().stability;
    ordinary.apply_stability_target("normal", 0.0).unwrap();
    anchor.apply_stability_target("anchor", 0.0).unwrap();
    assert!(
        (anchor.dynamics("anchor").unwrap().stability - anchor_before).abs()
            < (ordinary.dynamics("normal").unwrap().stability - ordinary_before).abs()
    );
}

#[test]
fn l3_slows_stability_but_can_still_climb_into_l2() {
    let id = "old-memory";
    let mut m = memory(id, 0.01, 0.75, 0.20, 0.20);
    m.set_memory_l3_inertia_bonus(0.50).unwrap();
    assert_eq!(m.memory_level(id), Some(MemoryLevel::L3));
    for index in 1..=12 {
        m.apply_stability_target(id, 0.90).unwrap();
        sample(&m, id, &format!("reactivation {index}"));
        if m.memory_level(id) == Some(MemoryLevel::L2) {
            return;
        }
    }
    panic!("L3 must not become an irreversible state with inertia below 1.0");
}
