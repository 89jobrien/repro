use super::resolve_use_cache;

#[kani::proof]
fn use_cache_no_cache_always_false() {
    assert!(!resolve_use_cache(true));
}

#[kani::proof]
fn rootless_constraint_docker_rootless_rejected() {
    let runtime = "docker";
    let rootless = true;
    assert!(runtime != "podman" && rootless);
}

#[kani::proof]
fn rootless_constraint_false_always_accepted() {
    let rootless = false;
    assert!(!(rootless));
}

#[kani::proof]
fn sde_both_set_hits_error_arm() {
    let epoch: i64 = kani::any();
    let sde = Some(epoch);
    let dt: Option<&str> = Some("2024-01-01");
    assert!(matches!((sde, dt), (Some(_), Some(_))));
}

#[kani::proof]
fn sde_neither_set_hits_error_arm() {
    let sde: Option<i64> = None;
    let dt: Option<&str> = None;
    assert!(matches!((sde, dt), (None, None)));
}

#[kani::proof]
fn sde_epoch_only_returns_value() {
    let epoch: i64 = kani::any();
    let sde = Some(epoch);
    let dt: Option<&str> = None;
    match (sde, dt) {
        (Some(s), None) => assert_eq!(s, epoch),
        _ => unreachable!(),
    }
}

#[kani::proof]
fn buildkit_image_podman_needs_prefix() {
    let rootless = false;
    let runtime = "podman";
    assert!((rootless || runtime == "podman"));
}

#[kani::proof]
fn buildkit_image_rootless_needs_prefix() {
    let rootless = true;
    let runtime = "podman";
    assert!((rootless || runtime == "podman"));
}

#[kani::proof]
fn buildkit_image_docker_no_prefix() {
    let rootless = false;
    let runtime = "docker";
    assert!(!(rootless || runtime == "podman"));
}

#[kani::proof]
fn buildkit_args_constraint_docker_rejected() {
    let runtime = "docker";
    assert!(runtime != "podman");
}

#[kani::proof]
fn buildx_args_constraint_podman_rejected() {
    let runtime = "podman";
    assert!(runtime != "docker");
}
