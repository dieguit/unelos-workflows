use super::{slugify_run_name, store::RunStore, RunState, RuntimeError};
use camino::Utf8Path;

pub fn rename_run(root: &Utf8Path, run_id: &str, name: &str) -> Result<RunState, RuntimeError> {
    let name = slugify_run_name(name)?;
    let store = RunStore::open(root)?;
    let mut state = store.run(run_id)?;
    if state.name != name {
        store.ensure_start_available(Some(&name))?;
    }
    state.name = name;
    state.naming_pending = false;
    store.update(&state)?;
    store.run(&state.run_id)
}
