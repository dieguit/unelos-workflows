pub fn isolated_config() -> bool {
    const CHILD: &str = "UNELOS_ISOLATED_TEST_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let config = assert_fs::TempDir::new().unwrap();
    let thread = std::thread::current();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", thread.name().unwrap(), "--nocapture"])
        .env(CHILD, "1")
        .env("UNELOS_CONFIG_DIR", config.path())
        .status()
        .unwrap();
    assert!(status.success());
    false
}
