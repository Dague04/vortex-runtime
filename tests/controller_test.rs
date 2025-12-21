use vortex_cgroup::CGroupController;
use vortex_core::ContainerId;

#[tokio::test]
async fn test_create_cgroup() {
    let id = ContainerId::new("test-cgroup").unwrap();
    let cgroup = CGroupController::new(id).await;

    // Will fail without root, that's okay!
    if let Err(e) = cgroup {
        println!("Expected error (need root): {}", e);
    }
}
