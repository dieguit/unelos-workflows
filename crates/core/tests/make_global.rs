use camino::Utf8Path;
use std::{fs, process::Command};
use unelos_core::*;

#[test]
fn moves_and_preflight() {
    if std::env::var_os("UNELOS_MAKE_GLOBAL_CHILD").is_none() {
        let config = assert_fs::TempDir::new().unwrap();
        assert!(Command::new(std::env::current_exe().unwrap()).args(["--exact", "moves_and_preflight"])
            .env("UNELOS_MAKE_GLOBAL_CHILD", "1").env("UNELOS_CONFIG_DIR", config.path()).status().unwrap().success());
        return;
    }
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    for folder in ["skills/s", "skills/promoted", "agents/a", "tasks/t", "workflows", "mcps"] {
        fs::create_dir_all(root.join(".unelos").join(folder)).unwrap();
    }
    fs::write(root.join(".unelos/skills/s/SKILL.md"), "skill").unwrap();
    fs::write(root.join(".unelos/skills/promoted/SKILL.md"), "promoted").unwrap();
    let agent = root.join(".unelos/agents/a/agent.yaml");
    let task = root.join(".unelos/tasks/t/task.yaml");
    let workflow = root.join(".unelos/workflows/f.yaml");
    fs::write(&agent, "instructions: agent\nskills: [s]\nmcps: [m]\n").unwrap();
    fs::write(&task, "instructions: task\nskills: [s, promoted]\n").unwrap();
    fs::write(&workflow, "name: f\nstart: inline\nnodes:\n  inline:\n    agent: a\n    taskDefinition:\n      instructions: inline\n      skills: [s]\n    next: named\n  named:\n    agent: a\n    task: t\n    next: done\n  done:\n    type: end\n").unwrap();
    fs::write(root.join(".unelos/mcps/m.yaml"), "type: local\ncommand: [test]\n").unwrap();
    make_skill_global(root, "s").unwrap();
    assert!(!root.join(".unelos/skills/s").exists());
    for file in [&agent, &task, &workflow] { assert!(fs::read_to_string(file).unwrap().contains("global/s")); }
    make_mcp_global(root, "m").unwrap();
    assert!(fs::read_to_string(&agent).unwrap().contains("global/m"));
    assert!(!root.join(".unelos/mcps/m.yaml").exists());
    let global = std::path::PathBuf::from(std::env::var_os("UNELOS_CONFIG_DIR").unwrap());
    fs::create_dir_all(global.join("skills/promoted")).unwrap();
    let before = fs::read_to_string(&task).unwrap();
    assert!(make_task_global(root, "t").unwrap_err().contains("already exists"));
    assert_eq!(fs::read_to_string(&task).unwrap(), before);
    assert!(!global.join("tasks/t").exists());
    fs::remove_dir(global.join("skills/promoted")).unwrap();
    make_task_global(root, "t").unwrap();
    assert!(!task.exists());
    assert!(fs::read_to_string(global.join("tasks/t/task.yaml")).unwrap().contains("global/promoted"));
    assert!(fs::read_to_string(&workflow).unwrap().contains("global/t"));
    assert!(!root.join(".unelos/skills/promoted").exists());
}
