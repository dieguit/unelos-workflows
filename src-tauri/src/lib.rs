mod projects;
mod imports;
mod runs;
mod harnesses;
mod model_mappings;

use tauri::webview::PageLoadEvent;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_opener::OpenerExt;

fn external_navigation_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("external-navigation")
        .on_navigation(|webview, url| {
            let is_internal_host = matches!(
                url.host_str(),
                Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost") | Some("::1")
            );

            let is_internal = url.scheme() == "tauri" || is_internal_host;

            if is_internal {
                return true;
            }

            let is_external_link = matches!(url.scheme(), "http" | "https" | "mailto" | "tel");

            if is_external_link {
                log::info!("opening external link in system browser: {}", url);
                let _ = webview.opener().open_url(url.as_str(), None::<&str>);
                return false;
            }

            true
        })
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(imports::ImportSessions::default())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(external_navigation_plugin())
        .setup(|_| {
            tauri::async_runtime::spawn_blocking(harnesses::provision_bundled_cli);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            imports::import_prepare,
            imports::import_plan,
            imports::import_commit,
            imports::import_cancel,
            runs::list_runs,
            runs::get_run,
            runs::delete_run,
            runs::read_run_artifact,
            harnesses::list_harnesses,
            model_mappings::list_model_mappings,
            model_mappings::save_model_mapping,
            model_mappings::delete_model_mapping,
            model_mappings::harness_models,
            model_mappings::agent_model_options,
            projects::get_project_settings,
            projects::save_project_settings,
            projects::build_harness_config,
            projects::auto_build_harness,
            projects::get_agent,
            projects::get_task,
            projects::save_agent,
            projects::list_mcps,
            projects::get_mcp,
            projects::save_mcp,
            projects::remove_mcp,
            projects::list_global_mcps,
            projects::get_global_mcp,
            projects::save_global_mcp,
            projects::remove_global_mcp,
            projects::list_global_skills,
            projects::get_global_skill,
            projects::create_global_skill,
            projects::save_global_skill,
            projects::duplicate_global_skill,
            projects::remove_global_skill,
            projects::save_task,
            harnesses::install_harness,
            harnesses::uninstall_harness,
            projects::project_state,
            projects::open_project,
            projects::pick_project,
            projects::remove_recent,
            projects::list_workflows,
            projects::duplicate_agent,
            projects::duplicate_task,
            projects::duplicate_skill,
            projects::duplicate_workflow,
            projects::remove_agent,
            projects::remove_task,
            projects::remove_skill,
            projects::make_skill_global,
            projects::make_mcp_global,
            projects::make_task_global,
            projects::remove_workflow,
            projects::get_workflow,
            projects::validate_workflow,
            projects::save_workflow,
            projects::create_workflow,
            projects::list_agents,
            projects::list_tasks,
            projects::list_global_tasks,
            projects::get_global_task,
            projects::save_global_task,
            projects::duplicate_global_task,
            projects::remove_global_task,
            projects::list_skills,
            projects::get_skill,
            projects::create_skill,
            projects::save_skill
        ])
        .on_page_load(|webview, payload| {
            if webview.label() == "main" && matches!(payload.event(), PageLoadEvent::Finished) {
                log::info!("main webview finished loading");
                let _ = webview.window().show();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
