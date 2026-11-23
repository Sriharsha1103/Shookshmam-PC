//! Phase 3 safety-regression integration tests: the auto evaluator, the safety
//! policy and the gated controller together must never turn a lost sensor into a
//! low-duty command, and must refuse unvalidated channels.

use shookshmam_control::{
    preset, AutoController, AutoOutcome, ChannelBounds, ChannelInfo, CommandSource, ControlCommand,
    ControlError, Decision, FanController, GatedFanController, SafetyPolicy, SensorLossBehavior,
    TempSample, Watchdog,
};

fn enabled_policy() -> SafetyPolicy {
    let mut p = SafetyPolicy::new();
    p.register_channel(
        "cpu_fan",
        ChannelBounds { min_duty: 20.0, max_duty: 100.0, allow_stop: false, validated: true },
    );
    assert!(p.enable("cpu_fan"));
    p
}

/// Translate an auto outcome into the duty that would actually be commanded,
/// via the safety policy. Returns None when control is released.
fn commanded_duty(policy: &SafetyPolicy, outcome: AutoOutcome) -> Option<Decision> {
    match outcome {
        AutoOutcome::Apply(d) | AutoOutcome::Fallback(d) => Some(policy.evaluate(&ControlCommand {
            channel: "cpu_fan".into(),
            duty: d,
            source: CommandSource::Automatic,
        })),
        AutoOutcome::Release => None,
    }
}

#[test]
fn lost_sensor_never_produces_low_duty() {
    let mut curve = preset("balanced").unwrap();
    curve.sensor_loss = SensorLossBehavior::SafeFallbackDuty(100.0);
    let policy = enabled_policy();
    let mut auto = AutoController::new(30.0);

    // Sensor goes stale: the outcome must be the safe fallback, applied high.
    let outcome = auto.step(&curve, &[TempSample::stale("cpu.temperature.tctl")], 1000);
    assert_eq!(outcome, AutoOutcome::Fallback(100.0));
    match commanded_duty(&policy, outcome) {
        Some(Decision::Apply(duty)) => assert!(duty >= 90.0, "fallback must be high, got {duty}"),
        other => panic!("unexpected decision: {other:?}"),
    }
}

#[test]
fn release_on_sensor_loss_relinquishes_control() {
    let mut curve = preset("balanced").unwrap();
    curve.sensor_loss = SensorLossBehavior::ReleaseToFirmware;
    let policy = enabled_policy();
    let mut auto = AutoController::new(50.0);
    let outcome = auto.step(&curve, &[], 1000);
    assert_eq!(outcome, AutoOutcome::Release);
    assert!(commanded_duty(&policy, outcome).is_none());
}

#[test]
fn unvalidated_channel_is_refused_end_to_end() {
    // Channel registered but not validated -> cannot enable -> command rejected.
    let mut policy = SafetyPolicy::new();
    policy.register_channel("gpu_fan", ChannelBounds::new(20.0, 100.0));
    assert!(!policy.enable("gpu_fan"));
    let decision = policy.evaluate(&ControlCommand {
        channel: "gpu_fan".into(),
        duty: 40.0,
        source: CommandSource::Automatic,
    });
    assert!(matches!(decision, Decision::Reject(_)));
}

#[test]
fn gated_controller_refuses_apply_but_allows_release() {
    let mut ctl = GatedFanController::new(vec![ChannelInfo {
        id: "cpu_fan".into(),
        label: "CPU Fan".into(),
        min_duty: 20.0,
        max_duty: 100.0,
        allow_stop: false,
        validated: false,
    }]);
    assert_eq!(ctl.apply_duty("cpu_fan", 50.0), Err(ControlError::NotValidated));
    assert_eq!(ctl.release("cpu_fan"), Ok(()));
}

#[test]
fn watchdog_trip_signals_release() {
    let wd = Watchdog::new(3000, 1000);
    assert!(!wd.is_tripped(2000));
    // No heartbeat for > timeout -> control must be released by the caller.
    assert!(wd.is_tripped(5000));
}
