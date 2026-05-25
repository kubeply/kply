//! Command-line entrypoint for the Kply placeholder CLI.

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use kply_cli::cli::{AppCommand, Cli, ClusterCommand, Command, ConfigCommand};
use kply_config::{
    AppConfig, ConfigLoadError, ConfigValidationErrors, KplyConfig, load_config_path,
};
use std::ffi::OsString;
use std::fmt::Display;
use std::process::ExitCode;

const EXIT_USAGE: i32 = 2;
const EXIT_INTERNAL: i32 = 3;
const EXIT_BLOCKING: i32 = 1;

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("kply error: internal\n\n{error:#}");
            exit_code(EXIT_INTERNAL)
        }
    }
}

fn run() -> Result<ExitCode> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let wants_json = args_have_flag(&args, "--json");
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(error) => return render_parse_error(error, wants_json),
    };

    print_verbose_trace(&cli);

    match &cli.command {
        Some(Command::Help) => {
            Cli::command().print_help()?;
            println!();
            return Ok(ExitCode::SUCCESS);
        }
        Some(Command::Config {
            command: Some(ConfigCommand::Show),
        }) => return render_config_show(&cli),
        Some(Command::Config {
            command: Some(ConfigCommand::Validate),
        }) => return render_config_validate(&cli),
        Some(Command::App {
            command: Some(AppCommand::List),
        }) => return render_app_list(&cli),
        Some(Command::App {
            command: Some(AppCommand::Inspect { app }),
        }) => return render_app_inspect(&cli, app),
        Some(Command::Cluster {
            command: Some(ClusterCommand::Info),
        }) => return render_cluster_info(&cli),
        Some(command) => {
            if cli.json {
                let value = serde_json::json!({
                    "command": command.name(),
                    "status": "placeholder"
                });
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else {
                if !cli.quiet {
                    println!("kply {}", command.name());
                    println!("Command group is defined but behavior is intentionally pending.");
                }
            }
            return Ok(ExitCode::SUCCESS);
        }
        None => {}
    }

    if cli.version {
        if cli.json {
            let value = serde_json::json!({
                "name": "kply",
                "version": env!("CARGO_PKG_VERSION")
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
            return Ok(ExitCode::SUCCESS);
        }

        println!("kply {}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }

    if cli.json {
        let value = serde_json::json!({
            "name": "kply",
            "version": env!("CARGO_PKG_VERSION"),
            "status": "placeholder"
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        println!("kply {}", env!("CARGO_PKG_VERSION"));
        println!("Placeholder CLI. Roadmap and commands are intentionally pending.");
    }

    Ok(ExitCode::SUCCESS)
}

/// Render configured application targets.
fn render_app_list(cli: &Cli) -> Result<ExitCode> {
    let config = match resolved_config(cli) {
        Ok(config) => config,
        Err(error) => return render_config_load_error(&error, cli.json),
    };

    if let Err(errors) = config.validate() {
        return render_config_validation_error(&errors, cli.json);
    }

    let apps = config.apps().entries();
    if cli.json {
        let value = serde_json::json!({
            "apps": apps
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        println!("kply app list");
        if apps.is_empty() {
            println!("No apps configured.");
        } else {
            for app in apps {
                println!("{}", app_list_line(app));
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Render one configured app as stable human-readable output.
fn app_list_line(app: &AppConfig) -> String {
    let default_image = app.default_image().unwrap_or("<none>");
    format!(
        "{} namespace={} workload={} service={} route_strategy={} default_image={}",
        app.name(),
        app.namespace(),
        app.workload(),
        app.service(),
        app.route_strategy().as_str(),
        default_image
    )
}

/// Render config validation errors for commands that consume valid config.
fn render_config_validation_error(
    errors: &ConfigValidationErrors,
    wants_json: bool,
) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "status": "invalid",
            "errors": errors.errors().iter().map(ToString::to_string).collect::<Vec<_>>()
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: config validation\n\n{errors}");
    }

    Ok(exit_code(EXIT_BLOCKING))
}

/// Render one configured application target.
fn render_app_inspect(cli: &Cli, app_name: &str) -> Result<ExitCode> {
    let config = match resolved_config(cli) {
        Ok(config) => config,
        Err(error) => return render_config_load_error(&error, cli.json),
    };

    if let Err(errors) = config.validate() {
        return render_config_validation_error(&errors, cli.json);
    }

    let Some(app) = config
        .apps()
        .entries()
        .iter()
        .find(|app| app.name() == app_name)
    else {
        return render_app_not_found_error(app_name, cli.json);
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(app)?);
    } else if !cli.quiet {
        println!("kply app inspect {}", app.name());
        println!("name: {}", app.name());
        println!("namespace: {}", app.namespace());
        println!("workload: {}", app.workload());
        println!("service: {}", app.service());
        println!("route_strategy: {}", app.route_strategy().as_str());
        println!("default_image: {}", app.default_image().unwrap_or("<none>"));
    }

    Ok(ExitCode::SUCCESS)
}

/// Render a missing configured app as an input error.
fn render_app_not_found_error(app_name: &str, wants_json: bool) -> Result<ExitCode> {
    let message = format!("app `{app_name}` is not configured");
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "app_not_found",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: app\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render read-only cluster facts resolved from kubeconfig.
fn render_cluster_info(cli: &Cli) -> Result<ExitCode> {
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    let info = match runtime.block_on(kply_k8s::cluster_info()) {
        Ok(info) => info,
        Err(error) => return render_kubeconfig_error(&error, cli.json),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else if !cli.quiet {
        println!("kply cluster info");
        println!("cluster_url: {}", info.cluster_url);
        println!("default_namespace: {}", info.default_namespace);
    }

    Ok(ExitCode::SUCCESS)
}

/// Render Clap parse results through Kply's stable exit-code contract.
fn render_parse_error(error: clap::Error, wants_json: bool) -> Result<ExitCode> {
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            print!("{error}");
            Ok(ExitCode::SUCCESS)
        }
        _ => {
            if wants_json {
                render_json_usage_error(&error)?;
            } else {
                eprintln!("kply error: usage\n\n{error}");
            }
            Ok(exit_code(EXIT_USAGE))
        }
    }
}

/// Return true when a raw argument list includes a boolean flag.
fn args_have_flag(args: &[OsString], flag: &str) -> bool {
    args.iter().skip(1).any(|arg| arg == flag)
}

/// Render a usage error as structured JSON for agents.
fn render_json_usage_error(error: &clap::Error) -> Result<()> {
    let details = error.to_string();
    let message = details.lines().next().unwrap_or("usage error");
    let value = serde_json::json!({
        "error": {
            "code": "usage",
            "exit_code": EXIT_USAGE,
            "message": message,
            "details": details
        }
    });

    eprintln!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

/// Render the currently resolved configuration.
fn render_config_show(cli: &Cli) -> Result<ExitCode> {
    let config = match resolved_config(cli) {
        Ok(config) => config,
        Err(error) => return render_config_load_error(&error, cli.json),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else if !cli.quiet {
        println!("kply config show");
        println!("{}", serde_json::to_string_pretty(&config)?);
    }

    Ok(ExitCode::SUCCESS)
}

/// Validate the currently resolved configuration.
fn render_config_validate(cli: &Cli) -> Result<ExitCode> {
    let config = match resolved_config(cli) {
        Ok(config) => config,
        Err(error) => return render_config_load_error(&error, cli.json),
    };

    match config.validate() {
        Ok(()) => {
            if cli.json {
                let value = serde_json::json!({
                    "status": "valid",
                    "errors": []
                });
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else if !cli.quiet {
                println!("kply config validate");
                println!("Config is valid.");
            }

            Ok(ExitCode::SUCCESS)
        }
        Err(errors) => render_config_validation_error(&errors, cli.json),
    }
}

/// Render config file load errors as user-facing config errors.
fn render_config_load_error(error: &ConfigLoadError, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let message = error.to_string();
        let value = serde_json::json!({
            "error": {
                "code": "config",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: config\n\n{error}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render kubeconfig resolution errors as user-facing usage/auth errors.
fn render_kubeconfig_error(error: &impl Display, wants_json: bool) -> Result<ExitCode> {
    let message = error.to_string();
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "kubernetes_config",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: kubernetes config\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Resolve the configuration used by config commands.
///
/// If `--config` is provided, load that explicit file with [`load_config_path`].
/// Otherwise, return the default in-memory config shape. Automatic config file
/// discovery is intentionally not wired into CLI behavior yet.
fn resolved_config(cli: &Cli) -> std::result::Result<KplyConfig, ConfigLoadError> {
    if let Some(path) = &cli.config {
        return load_config_path(path);
    }

    Ok(KplyConfig::default())
}

/// Convert documented small integer exit codes into process exit codes.
fn exit_code(code: i32) -> ExitCode {
    let Ok(code) = u8::try_from(code) else {
        return ExitCode::from(EXIT_INTERNAL as u8);
    };
    ExitCode::from(code)
}

/// Print deterministic debug context when verbose mode is enabled.
fn print_verbose_trace(cli: &Cli) {
    if !cli.verbose {
        return;
    }

    let command = cli
        .command
        .as_ref()
        .map_or("<none>", |command| command.name());
    let config = cli
        .config
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<none>".to_owned());
    eprintln!(
        "debug: command={command} json={} quiet={} no_color={} config={} no_config={}",
        cli.json, cli.quiet, cli.no_color, config, cli.no_config
    );
}
