//! Contract tests for CommandRunner + IdempotentRunner ports.

use repro::builder::{CommandRunner, DryRunner, IdempotentRunner, MockRunner, ProcessRunner};

// --- C1.1: run() returns Ok on success ---

#[test]
fn c1_1_process_runner_succeeds_on_true() {
    let runner = ProcessRunner;
    let cmd: Vec<String> = vec!["true".into()];
    assert!(runner.run(&cmd).is_ok());
}

#[test]
fn c1_1_dry_runner_always_succeeds() {
    let runner = DryRunner;
    let cmd: Vec<String> = vec!["anything".into(), "--flag".into()];
    assert!(runner.run(&cmd).is_ok());
}

#[test]
fn c1_1_mock_runner_always_succeeds() {
    let runner = MockRunner::new();
    let cmd: Vec<String> = vec!["test".into(), "arg1".into()];
    assert!(runner.run(&cmd).is_ok());
}

// --- C1.2: run() returns Err on failure ---

#[test]
fn c1_2_process_runner_fails_on_false() {
    let runner = ProcessRunner;
    let cmd: Vec<String> = vec!["false".into()];
    assert!(runner.run(&cmd).is_err());
}

// --- C1.3: args are passed without mutation ---

#[test]
fn c1_3_mock_runner_captures_all_args() {
    let runner = MockRunner::new();
    let cmd: Vec<String> = vec!["prog".into(), "--foo".into(), "bar".into(), "baz".into()];
    runner.run(&cmd).unwrap();
    let captured = runner.commands();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0], cmd);
}

#[test]
fn c1_3_mock_runner_preserves_order_across_calls() {
    let runner = MockRunner::new();
    let cmd1: Vec<String> = vec!["first".into()];
    let cmd2: Vec<String> = vec!["second".into()];
    runner.run(&cmd1).unwrap();
    runner.run(&cmd2).unwrap();
    let captured = runner.commands();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], cmd1);
    assert_eq!(captured[1], cmd2);
}

// --- C1.4: run_no_check never errors (void return) ---

#[test]
fn c1_4_process_runner_no_check_does_not_panic_on_false() {
    let runner = ProcessRunner;
    let cmd: Vec<String> = vec!["false".into()];
    // This should not panic — it's fire-and-forget
    runner.run_no_check(&cmd);
}

#[test]
fn c1_4_dry_runner_no_check_does_not_panic() {
    let runner = DryRunner;
    let cmd: Vec<String> = vec!["nonexistent-binary".into()];
    runner.run_no_check(&cmd);
}

#[test]
fn c1_4_mock_runner_no_check_does_not_panic() {
    let runner = MockRunner::new();
    let cmd: Vec<String> = vec!["anything".into()];
    runner.run_no_check(&cmd);
}

// --- C1.5: run_no_check still captures/executes ---

#[test]
fn c1_5_mock_runner_no_check_captures_command() {
    let runner = MockRunner::new();
    let cmd: Vec<String> = vec!["setup".into(), "--create".into()];
    runner.run_no_check(&cmd);
    let captured = runner.commands();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0], cmd);
}
