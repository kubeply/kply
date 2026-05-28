//! Command-line entrypoint for the Kply placeholder CLI.

mod demo;
mod doctor;
mod local;

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::Service;
use kply_cli::cli::{
    AppCommand, CheckCommand, Cli, ClusterCommand, Command, ConfigCommand, DemoCommand,
    PolicyCommand, ReportCommand, ReportExportFormat, RouteCommand, SessionCommand,
};
use kply_config::{
    AppConfig, AppConfigs, CheckConfigs, ConfigLoadError, ConfigValidationErrors, ConfigVersion,
    DatabaseRiskWarningPolicy, KplyConfig, MutationModePolicy, PolicyConfig, PolicyConfigs,
    RouteStrategy, RoutingConfig, load_config_path,
};
use kply_core::{
    AppGraph, CheckResultStatus, ConfidenceLevel, GraphRelationship, ImageRef,
    KubernetesResourceRef, MetadataEntry, PlannedCheck, PlannedCleanupStep, RelationshipConfidence,
    RequiredPermission, RiskNote, RouteSelector, SandboxManifestError, ServiceRef, SessionId,
    SessionName, SessionOperation, SessionPlan, SessionPolicy, SessionStatus, TimeToLive,
    UnsupportedFeatureWarning, WorkloadRef, sandbox_deployment_manifest,
    sandbox_route_placeholder_manifest, sandbox_service_manifest,
};
use kply_k8s::{
    DeploymentRolloutPhase, DeploymentSummary, DiscoveryError, KubeconfigError, MutationError,
    MutationErrorCode, ResourceDeletionSummary, ServiceSummary, SessionSummary,
};
use kply_routing::{
    GatewayHttpRouteCleanupTarget, GatewayRouteCleanupSelector, gateway_http_route_cleanup_target,
    gateway_route_cleanup_selector,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::io::{ErrorKind as IoErrorKind, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

const EXIT_USAGE: i32 = 2;
const EXIT_INTERNAL: i32 = 3;
const EXIT_BLOCKING: i32 = 1;
const SESSION_HEADER_NAME: &str = "x-kply-session";
const ROUTE_STRATEGY_AUTO: &str = "auto";
const ROUTE_STRATEGY_NONE: &str = "none";
const ROUTE_STRATEGY_PREVIEW: &str = "preview";
const ROUTE_STRATEGY_PREVIEW_SERVICE: &str = "preview-service";
const UNSUPPORTED_FEATURE_EDGE_ROUTE_VALIDATION: &str = "edge_route_validation";
const UNSUPPORTED_REASON_PREVIEW_SKIPS_EDGE_ROUTE_VALIDATION: &str =
    "preview_service_skips_edge_route_validation";
const UNSUPPORTED_REASON_NONE_SKIPS_ROUTE_VALIDATION: &str =
    "route_strategy_none_skips_route_validation";
const RISK_CATEGORY_DATABASE: &str = "database";
const RISK_SEVERITY_WARNING: &str = "warning";
const RISK_REASON_DATABASE_REFERENCE_REQUIRES_MANUAL_REVIEW: &str =
    "database_reference_requires_manual_review";
const EXPERIMENTAL_APPLY_STAGE: &str = "experimental";
const SANDBOX_WORKLOAD_KIND: &str = "Deployment";
const SESSION_STATUS_ANNOTATION: &str = "kply.dev/session-status";
const SUPPORTED_ROUTE_STRATEGIES: [RouteStrategy; 3] = [
    RouteStrategy::Header,
    RouteStrategy::Host,
    RouteStrategy::Preview,
];

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
        Some(Command::Doctor { capability_report }) => {
            return doctor::render_doctor(&cli, *capability_report);
        }
        Some(Command::Init {
            from_cluster,
            output,
            overwrite,
        }) => {
            return render_init_from_cluster(&cli, *from_cluster, output.as_deref(), *overwrite);
        }
        Some(Command::Config {
            command: Some(ConfigCommand::Show),
        }) => return render_config_show(&cli),
        Some(Command::Config {
            command: Some(ConfigCommand::Validate),
        }) => return render_config_validate(&cli),
        Some(Command::Policy {
            command: Some(PolicyCommand::Check),
        }) => return render_policy_check(&cli),
        Some(Command::App {
            command: Some(AppCommand::List),
        }) => return render_app_list(&cli),
        Some(Command::App {
            command: Some(AppCommand::Inspect { app }),
        }) => return render_app_inspect(&cli, app),
        Some(Command::App {
            command: Some(AppCommand::Graph { app }),
        }) => return render_app_graph(&cli, app),
        Some(Command::Session {
            command: Some(SessionCommand::List { namespace }),
        }) => return render_session_list(&cli, namespace.as_deref()),
        Some(Command::Session {
            command: Some(SessionCommand::Status { session, namespace }),
        }) => return render_session_status(&cli, session, namespace.as_deref()),
        Some(Command::Session {
            command:
                Some(SessionCommand::Cleanup {
                    session,
                    apply,
                    dry_run,
                    namespace,
                }),
        }) => return render_session_cleanup(&cli, session, *apply, *dry_run, namespace.as_deref()),
        Some(Command::Session {
            command:
                Some(SessionCommand::Create {
                    app,
                    apply,
                    image,
                    namespace,
                    time_to_live,
                    route_strategy,
                }),
        }) => {
            return render_session_create(
                &cli,
                app,
                *apply,
                image.as_deref(),
                namespace.as_deref(),
                time_to_live.as_deref(),
                route_strategy.as_deref(),
            );
        }
        Some(Command::Session {
            command:
                Some(SessionCommand::Plan {
                    app,
                    image,
                    namespace,
                    time_to_live,
                    route_strategy,
                }),
        }) => {
            return render_session_plan(
                &cli,
                app,
                image.as_deref(),
                namespace.as_deref(),
                time_to_live.as_deref(),
                route_strategy.as_deref(),
            );
        }
        Some(Command::Session {
            command:
                Some(SessionCommand::Manifests {
                    app,
                    yaml,
                    image,
                    namespace,
                    time_to_live,
                    route_strategy,
                }),
        }) => {
            return render_session_manifests(
                &cli,
                app,
                *yaml,
                image.as_deref(),
                namespace.as_deref(),
                time_to_live.as_deref(),
                route_strategy.as_deref(),
            );
        }
        Some(Command::Cluster {
            command: Some(ClusterCommand::Info),
        }) => return render_cluster_info(&cli),
        Some(Command::Check {
            command: Some(CheckCommand::Run { session, namespace }),
        }) => return render_check_run(&cli, session, namespace.as_deref()),
        Some(Command::Route {
            command: Some(RouteCommand::Plan { session, namespace }),
        }) => return render_route_plan(&cli, session, namespace.as_deref()),
        Some(Command::Route {
            command:
                Some(RouteCommand::Apply {
                    session,
                    namespace,
                    confirm_route_mutation,
                }),
        }) => {
            return render_route_apply(
                &cli,
                session,
                namespace.as_deref(),
                *confirm_route_mutation,
            );
        }
        Some(Command::Route {
            command: Some(RouteCommand::Cleanup { session, namespace }),
        }) => return render_route_cleanup(&cli, session, namespace.as_deref()),
        Some(Command::Report {
            command: Some(ReportCommand::Show { session, namespace }),
        }) => return render_report_show(&cli, session, namespace.as_deref()),
        Some(Command::Report {
            command:
                Some(ReportCommand::Export {
                    session,
                    namespace,
                    format,
                }),
        }) => return render_report_export(session, namespace.as_deref(), *format),
        Some(Command::Demo {
            command: DemoCommand::Doctor,
        }) => return demo::doctor::render_demo_doctor(&cli),
        Some(Command::Demo {
            command: DemoCommand::Install,
        }) => return demo::install::render_demo_install(&cli),
        Some(Command::Demo {
            command: DemoCommand::Reset,
        }) => return demo::reset::render_demo_reset(&cli),
        Some(Command::Demo {
            command: DemoCommand::Teardown,
        }) => return demo::teardown::render_demo_teardown(&cli),
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

/// Render the cluster-backed init workflow and write a starter config.
fn render_init_from_cluster(
    cli: &Cli,
    from_cluster: bool,
    output: Option<&Path>,
    overwrite: bool,
) -> Result<ExitCode> {
    if !from_cluster {
        return render_init_usage_error(cli.json);
    }

    let output_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(kply_config::CANONICAL_CONFIG_FILENAME));
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let mut reserved_output = if overwrite {
        None
    } else {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output_path)
        {
            Ok(file) => Some(file),
            Err(error) if error.kind() == IoErrorKind::AlreadyExists => {
                return render_init_output_exists_error(&output_path, cli.json);
            }
            Err(error) => return Err(error.into()),
        }
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    let discovery = match runtime.block_on(discover_init_from_cluster()) {
        Ok(discovery) => discovery,
        Err(error) => {
            if reserved_output.is_some() {
                let _ = std::fs::remove_file(&output_path);
            }
            return render_discovery_error(&error, cli.json);
        }
    };
    let config = init_config_from_apps(&discovery.apps);
    if let Err(errors) = config.validate() {
        if reserved_output.is_some() {
            let _ = std::fs::remove_file(&output_path);
        }
        return render_config_validation_error(&errors, cli.json);
    }
    let config_yaml = render_init_config_yaml(&discovery.apps)?;

    if let Some(file) = reserved_output.as_mut() {
        file.write_all(config_yaml.as_bytes())?;
        file.sync_all()?;
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&output_path)?
            .write_all(config_yaml.as_bytes())?;
    }

    let report = InitReport {
        command: "init",
        source: "cluster",
        status: "generated",
        output_path: output_path.to_string_lossy().into_owned(),
        cluster: InitClusterReport {
            cluster_url: discovery.cluster.cluster_url,
            default_namespace: discovery.cluster.default_namespace,
            namespaces_scanned: discovery.namespaces.len(),
        },
        apps: discovery.apps,
        skipped_services: discovery.skipped_services,
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if !cli.quiet {
        print!("{}", init_report_text(&report, color_enabled(cli)));
    }

    Ok(ExitCode::SUCCESS)
}

async fn discover_init_from_cluster() -> Result<InitDiscovery, kply_k8s::DiscoveryError> {
    let cluster = kply_k8s::cluster_info()
        .await
        .map_err(|error| kply_k8s::DiscoveryError::from_kubeconfig_error_redacted(&error))?;
    let loaded_client = kply_k8s::load_discovery_client_with_info().await?;
    let namespaces = kply_k8s::list_namespaces(loaded_client.client.clone())
        .await
        .map_err(|error| {
            kply_k8s::DiscoveryError::from_kubernetes_api_error("list Namespaces", &error)
        })?;
    let mut apps = Vec::new();
    let mut skipped_services = Vec::new();

    for namespace in &namespaces {
        let deployments = kply_k8s::list_deployments(loaded_client.client.clone(), &namespace.name)
            .await
            .map_err(|error| {
                kply_k8s::DiscoveryError::from_kubernetes_api_error("list Deployments", &error)
            })?;
        let services = kply_k8s::list_services(loaded_client.client.clone(), &namespace.name)
            .await
            .map_err(|error| {
                kply_k8s::DiscoveryError::from_kubernetes_api_error("list Services", &error)
            })?;
        let namespace_discovery = discover_namespace_apps(&namespace.name, &deployments, &services);
        apps.extend(namespace_discovery.apps);
        skipped_services.extend(namespace_discovery.skipped_services);
    }

    deduplicate_app_names(&mut apps);
    apps.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.workload.cmp(&right.workload))
            .then_with(|| left.service.cmp(&right.service))
    });
    skipped_services.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.service.cmp(&right.service))
    });

    Ok(InitDiscovery {
        cluster,
        namespaces,
        apps,
        skipped_services,
    })
}

fn discover_namespace_apps(
    namespace: &str,
    deployments: &[kply_k8s::DeploymentSummary],
    services: &[kply_k8s::ServiceSummary],
) -> NamespaceInitDiscovery {
    let mut service_matches = Vec::new();
    let mut skipped_services = Vec::new();

    for service in services {
        if service.selector.is_empty() {
            skipped_services.push(InitSkippedService {
                namespace: namespace.to_owned(),
                service: service.name.clone(),
                reason: "missing_selector".to_owned(),
                matched_workloads: Vec::new(),
            });
            continue;
        }

        let mut matched_workloads = deployments
            .iter()
            .filter(|deployment| {
                selector_matches_labels(&service.selector, &deployment.pod_template_labels)
            })
            .map(|deployment| deployment.name.clone())
            .collect::<Vec<_>>();
        matched_workloads.sort_unstable();
        if matched_workloads.len() == 1 {
            service_matches.push((service, matched_workloads[0].clone()));
        } else {
            skipped_services.push(InitSkippedService {
                namespace: namespace.to_owned(),
                service: service.name.clone(),
                reason: if matched_workloads.is_empty() {
                    "unmatched_selector".to_owned()
                } else {
                    "ambiguous_selector".to_owned()
                },
                matched_workloads,
            });
        }
    }

    let mut apps = Vec::new();
    for deployment in deployments {
        let mut matched_services = service_matches
            .iter()
            .filter(|(_, workload)| workload == &deployment.name)
            .map(|(service, _)| *service)
            .collect::<Vec<_>>();
        matched_services.sort_by(|left, right| left.name.cmp(&right.name));
        let Some(service) = matched_services.first().copied() else {
            continue;
        };

        apps.push(InitDiscoveredApp {
            name: deployment.name.clone(),
            namespace: namespace.to_owned(),
            workload: deployment.name.clone(),
            workload_kind: "Deployment".to_owned(),
            service: service.name.clone(),
            route_strategy: RouteStrategy::Preview.as_str().to_owned(),
            ports: service.ports.iter().map(|port| port.port).collect(),
        });

        for extra_service in matched_services.iter().skip(1) {
            skipped_services.push(InitSkippedService {
                namespace: namespace.to_owned(),
                service: extra_service.name.clone(),
                reason: "duplicate_workload_service".to_owned(),
                matched_workloads: vec![deployment.name.clone()],
            });
        }
    }

    NamespaceInitDiscovery {
        apps,
        skipped_services,
    }
}

fn selector_matches_labels(
    selector: &[kply_k8s::LabelSelectorEntry],
    labels: &[kply_k8s::LabelSelectorEntry],
) -> bool {
    selector.iter().all(|selector_entry| {
        labels
            .iter()
            .any(|label| label.key == selector_entry.key && label.value == selector_entry.value)
    })
}

fn deduplicate_app_names(apps: &mut [InitDiscoveredApp]) {
    let mut counts = BTreeMap::<String, usize>::new();
    for app in apps.iter() {
        *counts.entry(app.name.clone()).or_default() += 1;
    }

    for app in apps {
        if counts.get(&app.name).copied().unwrap_or_default() > 1 {
            app.name = format!("{}-{}", app.namespace, app.workload);
        }
    }
}

fn init_config_from_apps(apps: &[InitDiscoveredApp]) -> KplyConfig {
    KplyConfig::new(
        ConfigVersion::CURRENT,
        AppConfigs::new(
            apps.iter()
                .map(|app| {
                    AppConfig::new(
                        app.name.clone(),
                        app.namespace.clone(),
                        app.workload.clone(),
                        app.service.clone(),
                        None,
                        RouteStrategy::Preview,
                    )
                    .with_workload_kind(app.workload_kind.clone())
                })
                .collect(),
        ),
        RoutingConfig,
        CheckConfigs::default(),
        PolicyConfigs::default(),
    )
}

fn render_init_config_yaml(apps: &[InitDiscoveredApp]) -> Result<String> {
    let document = InitConfigDocument {
        version: ConfigVersion::CURRENT.get(),
        apps: apps
            .iter()
            .map(|app| InitConfigDocumentApp {
                name: &app.name,
                namespace: &app.namespace,
                workload: &app.workload,
                workload_kind: &app.workload_kind,
                service: &app.service,
                route_strategy: &app.route_strategy,
            })
            .collect(),
        routing: BTreeMap::<String, String>::new(),
        checks: Vec::<String>::new(),
        policies: Vec::<String>::new(),
    };
    let mut yaml = serde_norway::to_string(&document)?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}

fn init_report_text(report: &InitReport, color: bool) -> String {
    let mut output = String::new();
    output.push_str("kply init --from-cluster\n\n");
    output.push_str(&format!("{}\n", style_heading("Cluster", color)));
    output.push_str(&format!(
        "  server              {}\n",
        report.cluster.cluster_url
    ));
    output.push_str(&format!(
        "  default_namespace   {}\n",
        report.cluster.default_namespace
    ));
    output.push_str(&format!(
        "  namespaces_scanned  {}\n\n",
        report.cluster.namespaces_scanned
    ));
    output.push_str(&format!("{}\n", style_heading("Discovered Apps", color)));
    if report.apps.is_empty() {
        output.push_str(&format!(
            "  {}\n",
            style_warning("no apps discovered", color)
        ));
    } else {
        for app in &report.apps {
            output.push_str(&format!(
                "  {} {}/{}",
                style_success("✓", color),
                app.namespace,
                app.workload
            ));
            output.push('\n');
            output.push_str(&format!("      app             {}\n", app.name));
            output.push_str(&format!("      service         {}\n", app.service));
            output.push_str(&format!("      route_strategy  {}\n", app.route_strategy));
            output.push_str(&format!(
                "      ports           {}\n",
                format_ports(&app.ports)
            ));
        }
    }
    if !report.skipped_services.is_empty() {
        output.push('\n');
        output.push_str(&format!("{}\n", style_heading("Skipped Services", color)));
        for service in &report.skipped_services {
            output.push_str(&format!(
                "  {} {}/{} reason={}",
                style_warning("!", color),
                service.namespace,
                service.service,
                service.reason
            ));
            output.push('\n');
        }
    }
    output.push('\n');
    output.push_str(&format!("{}\n", style_heading("Generated", color)));
    output.push_str(&format!("  path  {}\n", report.output_path));
    output.push_str(&format!("  apps  {}\n\n", report.apps.len()));
    output.push_str(&format!("{}\n", style_heading("Next", color)));
    output.push_str(&format!(
        "  kply --config {} app list\n",
        report.output_path
    ));
    if let Some(app) = report.apps.first() {
        output.push_str(&format!(
            "  kply --config {} app inspect {}",
            report.output_path, app.name
        ));
        output.push('\n');
    }
    output
}

fn format_ports(ports: &[i32]) -> String {
    if ports.is_empty() {
        return "<none>".to_owned();
    }
    ports
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn color_enabled(cli: &Cli) -> bool {
    !cli.no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

fn style_heading(value: &str, color: bool) -> String {
    if color {
        format!("\x1b[1m{value}\x1b[0m")
    } else {
        value.to_owned()
    }
}

fn style_success(value: &str, color: bool) -> String {
    if color {
        format!("\x1b[32m{value}\x1b[0m")
    } else {
        value.to_owned()
    }
}

fn style_warning(value: &str, color: bool) -> String {
    if color {
        format!("\x1b[33m{value}\x1b[0m")
    } else {
        value.to_owned()
    }
}

fn render_init_usage_error(wants_json: bool) -> Result<ExitCode> {
    let message = "kply init requires --from-cluster";
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "usage",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: usage\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

fn render_init_output_exists_error(path: &Path, wants_json: bool) -> Result<ExitCode> {
    let message = format!(
        "refusing to overwrite `{}`; pass --overwrite or choose --output <path>",
        path.display()
    );
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "output_exists",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: output exists\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

#[derive(Debug)]
struct NamespaceInitDiscovery {
    apps: Vec<InitDiscoveredApp>,
    skipped_services: Vec<InitSkippedService>,
}

#[derive(Debug)]
struct InitDiscovery {
    cluster: kply_k8s::ClusterInfo,
    namespaces: Vec<kply_k8s::NamespaceSummary>,
    apps: Vec<InitDiscoveredApp>,
    skipped_services: Vec<InitSkippedService>,
}

#[derive(Debug, Serialize)]
struct InitReport {
    command: &'static str,
    source: &'static str,
    status: &'static str,
    output_path: String,
    cluster: InitClusterReport,
    apps: Vec<InitDiscoveredApp>,
    skipped_services: Vec<InitSkippedService>,
}

#[derive(Debug, Serialize)]
struct InitClusterReport {
    cluster_url: String,
    default_namespace: String,
    namespaces_scanned: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct InitDiscoveredApp {
    name: String,
    namespace: String,
    workload: String,
    workload_kind: String,
    service: String,
    route_strategy: String,
    ports: Vec<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct InitSkippedService {
    namespace: String,
    service: String,
    reason: String,
    matched_workloads: Vec<String>,
}

#[derive(Serialize)]
struct InitConfigDocument<'a> {
    version: u16,
    apps: Vec<InitConfigDocumentApp<'a>>,
    routing: BTreeMap<String, String>,
    checks: Vec<String>,
    policies: Vec<String>,
}

#[derive(Serialize)]
struct InitConfigDocumentApp<'a> {
    name: &'a str,
    namespace: &'a str,
    workload: &'a str,
    workload_kind: &'a str,
    service: &'a str,
    route_strategy: &'a str,
}

/// Render the explicit session create command without mutating cluster state.
fn render_session_create(
    cli: &Cli,
    app_name: &str,
    apply: bool,
    image: Option<&str>,
    namespace: Option<&str>,
    time_to_live: Option<&str>,
    route_strategy: Option<&str>,
) -> Result<ExitCode> {
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

    let plan = match session_plan_from_config_with_policies(
        app,
        config.policies(),
        image,
        namespace,
        time_to_live,
        route_strategy,
    ) {
        Ok(plan) => plan,
        Err(SessionPlanBuildError::Config(message)) => {
            return render_session_plan_config_error(&message, cli.json);
        }
        Err(SessionPlanBuildError::Policy(message)) => {
            return render_session_plan_policy_error(&message, cli.json);
        }
        Err(SessionPlanBuildError::Usage(message)) => {
            return render_session_plan_error(&message, cli.json);
        }
    };

    if apply {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let applied = match apply_session_resources(
            &plan,
            |namespace, deployment| {
                runtime.block_on(create_session_deployment(namespace, deployment))
            },
            |namespace, service| runtime.block_on(create_session_service(namespace, service)),
            |namespace, name| {
                runtime.block_on(wait_for_session_deployment_readiness(namespace, name))
            },
            |resources, status| runtime.block_on(record_session_state_metadata(resources, status)),
        ) {
            Ok(applied) => applied,
            Err(SessionCreateApplyError::Policy(message)) => {
                return render_session_plan_policy_error(&message, cli.json);
            }
            Err(SessionCreateApplyError::Manifest(error)) => {
                return render_session_manifest_error(&error, cli.json);
            }
            Err(SessionCreateApplyError::Mutation(error)) => {
                return render_session_create_apply_error(&error, cli.json);
            }
            Err(SessionCreateApplyError::PartialMutation {
                error,
                created_resources,
                pending_resources,
                recorded_resources,
            }) => {
                return render_session_create_partial_apply_error(
                    &error,
                    &created_resources,
                    &pending_resources,
                    &recorded_resources,
                    cli.json,
                );
            }
        };

        if cli.json {
            let value = serde_json::json!({
                "app": app_name,
                "session_id": plan.id(),
                "status": "partially_applied",
                "mutation": "applied",
                "apply": true,
                "apply_stage": EXPERIMENTAL_APPLY_STAGE,
                "created_resources": applied.created_resources,
                "pending_resources": applied.pending_resources,
                "readiness": applied.readiness,
                "state": applied.state,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        } else if !cli.quiet {
            println!("kply session create {app_name}");
            println!("session_id: {}", plan.id());
            println!("status: partially_applied");
            println!("mutation: applied");
            println!("apply: true");
            println!("apply_stage: {EXPERIMENTAL_APPLY_STAGE}");
            println!("created_resources: {}", applied.created_resources.len());
            for resource in &applied.created_resources {
                println!(
                    "  created: {} {}/{}",
                    resource.kind, resource.namespace, resource.name
                );
            }
            println!("pending_resources: {}", applied.pending_resources.len());
            for resource in &applied.pending_resources {
                println!(
                    "  pending: {} {}/{}",
                    resource.kind, resource.namespace, resource.name
                );
            }
            println!(
                "readiness: {} {}/{} phase={}",
                applied.readiness.resource.kind,
                applied.readiness.resource.namespace,
                applied.readiness.resource.name,
                applied.readiness.phase.as_str()
            );
            println!("state: {}", applied.state.status.as_str());
            for resource in &applied.state.resources {
                println!(
                    "  state: {} {}/{}",
                    resource.kind, resource.namespace, resource.name
                );
            }
        }

        return Ok(ExitCode::SUCCESS);
    }

    if cli.json {
        let value = serde_json::json!({
            "app": app_name,
            "session_id": plan.id(),
            "status": "planned",
            "mutation": "not_applied",
            "apply": false,
            "planned_resources": plan.planned_resources(),
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        println!("kply session create {app_name}");
        println!("session_id: {}", plan.id());
        println!("status: planned");
        println!("mutation: not_applied");
        println!("apply: false");
        println!("planned_resources: {}", plan.planned_resources().len());
        for resource in plan.planned_resources() {
            println!("  resource: {resource}");
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Render sandbox sessions recorded in cluster metadata.
fn render_session_list(cli: &Cli, namespace: Option<&str>) -> Result<ExitCode> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let namespace = match namespace {
        Some(namespace) => namespace.to_owned(),
        None => match runtime.block_on(kply_k8s::cluster_info()) {
            Ok(info) => info.default_namespace,
            Err(error) => return render_kubeconfig_error(&error, cli.json),
        },
    };
    let sessions = match runtime.block_on(list_sessions_in_namespace(&namespace)) {
        Ok(sessions) => sessions,
        Err(error) => return render_discovery_error(&error, cli.json),
    };

    if cli.json {
        let value = serde_json::json!({
            "namespace": namespace,
            "sessions": sessions,
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        println!("kply session list");
        println!("namespace: {namespace}");
        println!("sessions: {}", sessions.len());
        for session in &sessions {
            println!(
                "  session: {} status={} app={} workload={}/{}",
                session.id,
                session.status.as_deref().unwrap_or("unknown"),
                session.app.as_deref().unwrap_or("unknown"),
                session.workload_kind,
                session.workload_name
            );
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Render one sandbox session recorded in cluster metadata.
fn render_session_status(cli: &Cli, session: &str, namespace: Option<&str>) -> Result<ExitCode> {
    let session_id = match SessionId::new(session) {
        Ok(session_id) => session_id,
        Err(error) => return render_session_status_error(&error.to_string(), cli.json),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let namespace = match namespace {
        Some(namespace) => namespace.to_owned(),
        None => match runtime.block_on(kply_k8s::cluster_info()) {
            Ok(info) => info.default_namespace,
            Err(error) => return render_kubeconfig_error(&error, cli.json),
        },
    };
    let session = match runtime.block_on(get_session_in_namespace(&namespace, session_id.as_str()))
    {
        Ok(session) => session,
        Err(error) => return render_discovery_error(&error, cli.json),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&session)?);
    } else if !cli.quiet {
        println!("kply session status {}", session.id);
        println!("namespace: {}", session.namespace);
        println!("status: {}", session.status.as_deref().unwrap_or("unknown"));
        println!("app: {}", session.app.as_deref().unwrap_or("unknown"));
        println!(
            "workload: {}/{}",
            session.workload_kind, session.workload_name
        );
    }

    Ok(ExitCode::SUCCESS)
}

/// Render the report lookup surface for one sandbox session.
fn render_report_show(cli: &Cli, session: &str, namespace: Option<&str>) -> Result<ExitCode> {
    let session_id = match SessionId::new(session) {
        Ok(session_id) => session_id,
        Err(error) => return render_report_show_error(&error.to_string(), cli.json),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let namespace = match namespace {
        Some(namespace) => namespace.to_owned(),
        None => match runtime.block_on(kply_k8s::cluster_info()) {
            Ok(info) => info.default_namespace,
            Err(error) => return render_kubeconfig_error(&error, cli.json),
        },
    };
    let session = match runtime.block_on(get_session_in_namespace(&namespace, session_id.as_str()))
    {
        Ok(session) => session,
        Err(error) => return render_discovery_error(&error, cli.json),
    };
    let unavailable = report_show_unavailable_from_session(&session);

    if cli.json {
        println!("{}", render_report_unavailable_json(&unavailable)?);
    } else if !cli.quiet {
        print!("{}", render_report_unavailable_text(&unavailable));
    }

    Ok(ExitCode::SUCCESS)
}

/// Render the JSON report export surface for one sandbox session.
fn render_report_export(
    session: &str,
    namespace: Option<&str>,
    format: ReportExportFormat,
) -> Result<ExitCode> {
    let session_id = match SessionId::new(session) {
        Ok(session_id) => session_id,
        Err(error) => return render_report_export_error(&error.to_string()),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let namespace = match namespace {
        Some(namespace) => namespace.to_owned(),
        None => match runtime.block_on(kply_k8s::cluster_info()) {
            Ok(info) => info.default_namespace,
            Err(error) => return render_kubeconfig_error(&error, true),
        },
    };
    let session = match runtime.block_on(get_session_in_namespace(&namespace, session_id.as_str()))
    {
        Ok(session) => session,
        Err(error) => return render_discovery_error(&error, true),
    };
    let unavailable = report_show_unavailable_from_session(&session);

    match format {
        ReportExportFormat::Json => println!("{}", render_report_unavailable_json(&unavailable)?),
        ReportExportFormat::Markdown => {
            println!("{}", render_report_unavailable_markdown(&unavailable))
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Render verification checks for one sandbox session.
fn render_check_run(cli: &Cli, session: &str, namespace: Option<&str>) -> Result<ExitCode> {
    let session_id = match SessionId::new(session) {
        Ok(session_id) => session_id,
        Err(error) => return render_check_run_error(&error.to_string(), cli.json),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let namespace = match namespace {
        Some(namespace) => namespace.to_owned(),
        None => match runtime.block_on(kply_k8s::cluster_info()) {
            Ok(info) => info.default_namespace,
            Err(error) => return render_kubeconfig_error(&error, cli.json),
        },
    };
    let session = match runtime.block_on(get_session_in_namespace(&namespace, session_id.as_str()))
    {
        Ok(session) => session,
        Err(error) => return render_discovery_error(&error, cli.json),
    };
    let report = check_run_report_from_session(&session);
    let exit_code_value = if report.status.is_blocking() {
        EXIT_BLOCKING
    } else {
        0
    };

    if cli.json {
        println!("{}", render_check_run_json_report(&report)?);
    } else if !cli.quiet {
        print!("{}", render_check_run_text_report(&report));
    }

    Ok(exit_code(exit_code_value))
}

/// Render a non-mutating sandbox session cleanup plan.
fn render_session_cleanup(
    cli: &Cli,
    session: &str,
    apply: bool,
    dry_run: bool,
    namespace: Option<&str>,
) -> Result<ExitCode> {
    let session_id = match SessionId::new(session) {
        Ok(session_id) => session_id,
        Err(error) => return render_session_cleanup_error(&error.to_string(), cli.json),
    };

    if apply && dry_run {
        return render_session_cleanup_error("--dry-run cannot be used with --apply", cli.json);
    }

    if !apply && !dry_run && namespace.is_some() {
        return render_session_cleanup_error("--namespace requires --apply or --dry-run", cli.json);
    }

    if apply {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let loaded_client = match runtime.block_on(kply_k8s::load_mutation_client()) {
            Ok(loaded_client) => loaded_client,
            Err(error) => {
                let error = SessionCleanupApplyError {
                    error,
                    deletion_accepted_resources: Vec::new(),
                    pending_resources: Vec::new(),
                };
                return render_session_cleanup_apply_error(&error, cli.json);
            }
        };
        let namespace = match namespace {
            Some(namespace) => namespace.to_owned(),
            None => loaded_client.default_namespace.clone(),
        };
        let deletion_accepted_resources =
            match runtime.block_on(kply_k8s::delete_session_resources(
                loaded_client.client,
                &namespace,
                session_id.as_str(),
            )) {
                Ok(deletion_accepted_resources) => deletion_accepted_resources,
                Err(error) => {
                    let error = session_cleanup_error_from_cleanup_error(error);
                    return render_session_cleanup_apply_error(&error, cli.json);
                }
            };

        if cli.json {
            let value = serde_json::json!({
                "session_id": session_id.as_str(),
                "namespace": namespace,
                "status": "cleanup_requested",
                "mutation": "applied",
                "apply": true,
                "deletion_accepted_resources": deletion_accepted_resources,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        } else if !cli.quiet {
            println!("kply session cleanup {}", session_id.as_str());
            println!("namespace: {namespace}");
            println!("status: cleanup_requested");
            println!("mutation: applied");
            println!("apply: true");
            println!(
                "deletion_accepted_resources: {}",
                deletion_accepted_resources.len()
            );
            for resource in &deletion_accepted_resources {
                println!(
                    "  deletion_accepted: {} {}/{}",
                    resource.kind, resource.namespace, resource.name
                );
            }
        }

        return Ok(ExitCode::SUCCESS);
    }

    if dry_run {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let loaded_client = match runtime.block_on(kply_k8s::load_discovery_client_with_info()) {
            Ok(loaded_client) => loaded_client,
            Err(error) => return render_discovery_error(&error, cli.json),
        };
        let namespace = match namespace {
            Some(namespace) => namespace.to_owned(),
            None => loaded_client.default_namespace.clone(),
        };
        let deletion_candidate_resources =
            match runtime.block_on(kply_k8s::list_session_cleanup_resources(
                loaded_client.client,
                &namespace,
                session_id.as_str(),
            )) {
                Ok(deletion_candidate_resources) => deletion_candidate_resources,
                Err(error) => {
                    let error = DiscoveryError::from_kubernetes_api_error(
                        "list session cleanup resources",
                        &error,
                    );
                    return render_discovery_error(&error, cli.json);
                }
            };

        if cli.json {
            let value = serde_json::json!({
                "session_id": session_id.as_str(),
                "namespace": namespace,
                "status": "planned",
                "mutation": "not_applied",
                "apply": false,
                "dry_run": true,
                "deletion_candidate_resources": deletion_candidate_resources,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        } else if !cli.quiet {
            println!("kply session cleanup {}", session_id.as_str());
            println!("namespace: {namespace}");
            println!("status: planned");
            println!("mutation: not_applied");
            println!("apply: false");
            println!("dry_run: true");
            println!(
                "deletion_candidate_resources: {}",
                deletion_candidate_resources.len()
            );
            for resource in &deletion_candidate_resources {
                println!(
                    "  deletion_candidate: {} {}/{}",
                    resource.kind, resource.namespace, resource.name
                );
            }
        }

        return Ok(ExitCode::SUCCESS);
    }

    if cli.json {
        let value = serde_json::json!({
            "session_id": session_id.as_str(),
            "status": "planned",
            "mutation": "not_applied",
            "cleanup": "not_implemented",
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        println!("kply session cleanup {}", session_id.as_str());
        println!("status: planned");
        println!("mutation: not_applied");
        println!("cleanup: not_implemented");
    }

    Ok(ExitCode::SUCCESS)
}

/// Render a deterministic dry-run route plan for one sandbox session.
fn render_route_plan(cli: &Cli, session: &str, namespace: Option<&str>) -> Result<ExitCode> {
    let session_id = match SessionId::new(session) {
        Ok(session_id) => session_id,
        Err(error) => return render_route_plan_error(&error.to_string(), cli.json),
    };
    let route_plan = match route_plan_from_session(session_id.as_str(), namespace) {
        Ok(route_plan) => route_plan,
        Err(error) => return render_route_plan_error(&error.to_string(), cli.json),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&route_plan)?);
    } else if !cli.quiet {
        println!("kply route plan {}", route_plan.session_id);
        println!("status: {}", route_plan.status);
        println!("mutation: {}", route_plan.mutation);
        println!("apply: {}", route_plan.apply);
        println!("route_kind: {}", route_plan.route_kind);
        match &route_plan.planned_resource {
            Some(resource) => {
                println!(
                    "planned_resource: {}/{}/{}",
                    resource.namespace, resource.kind, resource.name
                );
            }
            None => println!("planned_resource: <namespace required>"),
        }
        println!(
            "cleanup_selector: {}",
            route_plan.cleanup_selector.match_labels.len()
        );
        for (key, value) in &route_plan.cleanup_selector.match_labels {
            println!("  label: {key}={value}");
        }
        println!(
            "unsupported_routes: {}",
            route_plan.unsupported_routes.len()
        );
        for route in &route_plan.unsupported_routes {
            println!(
                "  unsupported_route: {}:{} ({}) action={}",
                route.strategy, route.feature, route.reason, route.action
            );
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Render a guarded route apply placeholder without mutating Kubernetes.
fn render_route_apply(
    cli: &Cli,
    session: &str,
    namespace: Option<&str>,
    confirm_route_mutation: bool,
) -> Result<ExitCode> {
    let session_id = match SessionId::new(session) {
        Ok(session_id) => session_id,
        Err(error) => return render_route_apply_error(&error.to_string(), cli.json),
    };
    if !confirm_route_mutation {
        return render_route_apply_error(
            "route apply requires --confirm-route-mutation before route mutation can run",
            cli.json,
        );
    }

    let output = RouteApplyOutput {
        session_id: session_id.as_str().to_owned(),
        namespace: namespace.map(ToOwned::to_owned),
        status: "not_implemented",
        mutation: "not_applied",
        apply: false,
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if !cli.quiet {
        println!("kply route apply {}", output.session_id);
        println!(
            "namespace: {}",
            output.namespace.as_deref().unwrap_or("<default>")
        );
        println!("status: {}", output.status);
        println!("mutation: {}", output.mutation);
        println!("apply: {}", output.apply);
    }

    Ok(ExitCode::SUCCESS)
}

/// Render a non-mutating route cleanup plan for one sandbox session.
fn render_route_cleanup(cli: &Cli, session: &str, namespace: Option<&str>) -> Result<ExitCode> {
    let session_id = match SessionId::new(session) {
        Ok(session_id) => session_id,
        Err(error) => return render_route_cleanup_error(&error.to_string(), cli.json),
    };
    let route_cleanup = match route_cleanup_from_session(session_id.as_str(), namespace) {
        Ok(route_cleanup) => route_cleanup,
        Err(error) => return render_route_cleanup_error(&error.to_string(), cli.json),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&route_cleanup)?);
    } else if !cli.quiet {
        println!("kply route cleanup {}", route_cleanup.session_id);
        println!("status: {}", route_cleanup.status);
        println!("mutation: {}", route_cleanup.mutation);
        println!("cleanup: {}", route_cleanup.cleanup);
        println!("route_kind: {}", route_cleanup.route_kind);
        match &route_cleanup.cleanup_target {
            Some(target) => {
                println!(
                    "cleanup_target: {}/{}/{}",
                    target.namespace, target.kind, target.name
                );
            }
            None => println!("cleanup_target: <namespace required>"),
        }
        println!(
            "cleanup_selector: {}",
            route_cleanup.cleanup_selector.match_labels.len()
        );
        for (key, value) in &route_cleanup.cleanup_selector.match_labels {
            println!("  label: {key}={value}");
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Convert Kubernetes cleanup failures into CLI cleanup apply failures.
fn session_cleanup_error_from_cleanup_error(
    error: kply_k8s::CleanupError,
) -> SessionCleanupApplyError {
    SessionCleanupApplyError {
        error: MutationError::from_kubernetes_api_error(
            "delete sandbox session resources",
            error.source(),
        ),
        deletion_accepted_resources: error.deletion_accepted_resources,
        pending_resources: error.pending_resources,
    }
}

/// List sandbox sessions through the Kubernetes adapter.
async fn list_sessions_in_namespace(
    namespace: &str,
) -> std::result::Result<Vec<SessionSummary>, DiscoveryError> {
    let client = kply_k8s::load_discovery_client().await?;

    kply_k8s::list_sessions(client, namespace)
        .await
        .map_err(|error| DiscoveryError::from_kubernetes_api_error("list sessions", &error))
}

/// Get one sandbox session through the Kubernetes adapter.
async fn get_session_in_namespace(
    namespace: &str,
    session_id: &str,
) -> std::result::Result<SessionSummary, DiscoveryError> {
    let client = kply_k8s::load_discovery_client().await?;
    let session = kply_k8s::get_session(client, namespace, session_id)
        .await
        .map_err(|error| {
            DiscoveryError::from_kubernetes_api_error("read session status", &error)
        })?;
    session.ok_or_else(|| DiscoveryError {
        code: kply_k8s::DiscoveryErrorCode::MissingWorkload,
        message: format!("session {session_id} was not found in namespace {namespace}"),
    })
}

/// Build the current non-mutating check report from discovered session metadata.
fn check_run_report_from_session(session: &SessionSummary) -> CheckRunReport {
    let status = session_state_check_status(session.status.as_deref());
    let target = format!(
        "{}/{}/{}",
        session.namespace, session.workload_kind, session.workload_name
    );
    let check = CheckRunItem {
        name: "session_state",
        target,
        status,
        evidence: serde_json::json!({
            "observed_status": session.status,
            "expected_status": "active",
            "workload_kind": session.workload_kind,
            "workload_name": session.workload_name,
        }),
    };

    CheckRunReport {
        session_id: session.id.clone(),
        namespace: session.namespace.clone(),
        status: check.status,
        checks: vec![check],
    }
}

/// Build the current report availability response from discovered session metadata.
fn report_show_unavailable_from_session(session: &SessionSummary) -> ReportShowUnavailable {
    ReportShowUnavailable {
        session_id: session.id.clone(),
        namespace: session.namespace.clone(),
        session_status: session
            .status
            .clone()
            .unwrap_or_else(|| "unknown".to_owned()),
        report: "not_available",
        reason: "session_report_persistence_not_implemented",
    }
}

/// Render a deterministic text report availability response.
fn render_report_unavailable_text(report: &ReportShowUnavailable) -> String {
    format!(
        "kply report show {}\nnamespace: {}\nsession_status: {}\nreport: {}\nreason: {}\n",
        report.session_id, report.namespace, report.session_status, report.report, report.reason
    )
}

/// Render a deterministic JSON report availability response.
fn render_report_unavailable_json(report: &ReportShowUnavailable) -> serde_json::Result<String> {
    serde_json::to_string_pretty(report)
}

/// Render a deterministic Markdown report availability response.
fn render_report_unavailable_markdown(report: &ReportShowUnavailable) -> String {
    use std::fmt::Write as _;

    let mut output = String::new();
    writeln!(output, "# Kply Session Report").expect("writing report markdown should not fail");
    writeln!(output).expect("writing report markdown should not fail");
    writeln!(output, "- Session: `{}`", report.session_id)
        .expect("writing report markdown should not fail");
    writeln!(output, "- Namespace: `{}`", report.namespace)
        .expect("writing report markdown should not fail");
    writeln!(output, "- Session status: `{}`", report.session_status)
        .expect("writing report markdown should not fail");
    writeln!(output, "- Report: `{}`", report.report)
        .expect("writing report markdown should not fail");
    writeln!(output, "- Reason: `{}`", report.reason)
        .expect("writing report markdown should not fail");

    output
}

/// Return the check status for discovered session lifecycle metadata.
fn session_state_check_status(status: Option<&str>) -> CheckResultStatus {
    match status {
        Some("active") => CheckResultStatus::Passed,
        Some(_) => CheckResultStatus::Failed,
        None => CheckResultStatus::Warning,
    }
}

/// Render a deterministic agent-readable JSON check report.
fn render_check_run_json_report(report: &CheckRunReport) -> serde_json::Result<String> {
    let json_report = CheckRunJsonReport {
        session_id: &report.session_id,
        namespace: &report.namespace,
        status: report.status,
        summary: CheckRunStatusCounts::from_checks(&report.checks),
        checks: &report.checks,
    };

    serde_json::to_string_pretty(&json_report)
}

/// Render a deterministic human-readable check report.
fn render_check_run_text_report(report: &CheckRunReport) -> String {
    use std::fmt::Write as _;

    let counts = CheckRunStatusCounts::from_checks(&report.checks);
    let mut output = String::new();

    writeln!(output, "kply check run {}", report.session_id)
        .expect("writing check report to a string should not fail");
    writeln!(output, "namespace: {}", report.namespace)
        .expect("writing check report to a string should not fail");
    writeln!(output, "status: {}", report.status)
        .expect("writing check report to a string should not fail");
    writeln!(
        output,
        "summary: passed={} failed={} warning={} skipped={}",
        counts.passed, counts.failed, counts.warning, counts.skipped
    )
    .expect("writing check report to a string should not fail");
    writeln!(output, "checks: {}", report.checks.len())
        .expect("writing check report to a string should not fail");

    for check in &report.checks {
        writeln!(output, "  check: {}", check.name)
            .expect("writing check report to a string should not fail");
        writeln!(output, "    target: {}", check.target)
            .expect("writing check report to a string should not fail");
        writeln!(output, "    status: {}", check.status)
            .expect("writing check report to a string should not fail");
        if let Some(evidence) = render_check_evidence(&check.evidence) {
            writeln!(output, "    evidence: {evidence}")
                .expect("writing check report to a string should not fail");
        }
    }

    output
}

/// Render compact key-value evidence for text check reports.
fn render_check_evidence(evidence: &serde_json::Value) -> Option<String> {
    match evidence {
        serde_json::Value::Object(fields) if fields.is_empty() => None,
        serde_json::Value::Object(fields) => {
            let mut rendered_fields = fields
                .iter()
                .map(|(key, value)| format!("{key}={}", render_evidence_value(value)))
                .collect::<Vec<_>>();
            rendered_fields.sort_unstable();
            Some(rendered_fields.join(" "))
        }
        serde_json::Value::Null => None,
        value => Some(render_evidence_value(value)),
    }
}

/// Render one evidence value without pretty-printing nested JSON.
fn render_evidence_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_owned(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::to_string(value).expect("serializing evidence JSON should not fail")
        }
    }
}

/// Result of applying session resources to Kubernetes.
#[derive(Debug, Serialize)]
struct SessionCreateApplyResult {
    created_resources: Vec<SessionManifestSummary>,
    pending_resources: Vec<SessionManifestSummary>,
    readiness: SessionReadinessSummary,
    state: SessionStateRecordSummary,
}

/// Readiness status observed before `session create --apply` succeeds.
#[derive(Debug, Serialize)]
struct SessionReadinessSummary {
    resource: SessionManifestSummary,
    phase: DeploymentRolloutPhase,
}

/// Session state recorded in Kubernetes resource metadata.
#[derive(Debug, Serialize)]
struct SessionStateRecordSummary {
    status: SessionStatus,
    resources: Vec<SessionManifestSummary>,
}

/// Machine-readable report emitted by `kply check run`.
#[derive(Debug, Serialize)]
struct CheckRunReport {
    session_id: String,
    namespace: String,
    status: CheckResultStatus,
    checks: Vec<CheckRunItem>,
}

/// JSON report emitted by `kply check run --json`.
#[derive(Debug, Serialize)]
struct CheckRunJsonReport<'a> {
    session_id: &'a str,
    namespace: &'a str,
    status: CheckResultStatus,
    summary: CheckRunStatusCounts,
    checks: &'a [CheckRunItem],
}

/// One check result emitted by `kply check run`.
#[derive(Debug, Serialize)]
struct CheckRunItem {
    name: &'static str,
    target: String,
    status: CheckResultStatus,
    evidence: serde_json::Value,
}

/// Status totals for check reports.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
struct CheckRunStatusCounts {
    passed: usize,
    failed: usize,
    warning: usize,
    skipped: usize,
}

/// Placeholder report availability emitted by `kply report show`.
#[derive(Debug, Serialize)]
struct ReportShowUnavailable {
    session_id: String,
    namespace: String,
    session_status: String,
    report: &'static str,
    reason: &'static str,
}

/// Dry-run route plan emitted by `kply route plan`.
#[derive(Debug, Serialize)]
struct RoutePlanOutput {
    session_id: String,
    status: &'static str,
    mutation: &'static str,
    apply: bool,
    route_kind: &'static str,
    planned_resource: Option<SessionManifestSummary>,
    cleanup_target: Option<GatewayHttpRouteCleanupTarget>,
    cleanup_selector: GatewayRouteCleanupSelector,
    unsupported_routes: Vec<UnsupportedRouteOutput>,
}

/// Unsupported route detail emitted by `kply route plan`.
#[derive(Debug, Serialize)]
struct UnsupportedRouteOutput {
    strategy: &'static str,
    feature: &'static str,
    reason: &'static str,
    action: &'static str,
}

/// Guarded route apply output emitted before route mutation is implemented.
#[derive(Debug, Serialize)]
struct RouteApplyOutput {
    session_id: String,
    namespace: Option<String>,
    status: &'static str,
    mutation: &'static str,
    apply: bool,
}

/// Dry-run route cleanup emitted by `kply route cleanup`.
#[derive(Debug, Serialize)]
struct RouteCleanupOutput {
    session_id: String,
    status: &'static str,
    mutation: &'static str,
    cleanup: bool,
    route_kind: &'static str,
    cleanup_target: Option<GatewayHttpRouteCleanupTarget>,
    cleanup_selector: GatewayRouteCleanupSelector,
}

impl CheckRunStatusCounts {
    /// Count check result statuses in declaration order.
    fn from_checks(checks: &[CheckRunItem]) -> Self {
        let mut counts = Self::default();
        for check in checks {
            match check.status {
                CheckResultStatus::Passed => counts.passed += 1,
                CheckResultStatus::Failed => counts.failed += 1,
                CheckResultStatus::Warning => counts.warning += 1,
                CheckResultStatus::Skipped => counts.skipped += 1,
                _ => counts.warning += 1,
            }
        }
        counts
    }
}

/// Error raised after one or more state metadata writes may have completed.
#[derive(Debug)]
struct SessionStateRecordError {
    error: MutationError,
    recorded_resources: Vec<SessionManifestSummary>,
}

/// Error raised while cleaning up session resources.
#[derive(Debug)]
struct SessionCleanupApplyError {
    error: MutationError,
    deletion_accepted_resources: Vec<ResourceDeletionSummary>,
    pending_resources: Vec<ResourceDeletionSummary>,
}

/// Error raised while applying session resources to Kubernetes.
#[derive(Debug)]
enum SessionCreateApplyError {
    /// Configured policy does not allow session resource apply.
    Policy(String),
    /// Generated session manifests could not be converted for apply.
    Manifest(SessionManifestBuildError),
    /// Kubernetes rejected or could not execute the mutation.
    Mutation(MutationError),
    /// Kubernetes rejected a later mutation after earlier resources were created.
    PartialMutation {
        /// Error raised by the failed mutation.
        error: MutationError,
        /// Resources already created before the failure.
        created_resources: Vec<SessionManifestSummary>,
        /// Resources not created because the failure stopped apply.
        pending_resources: Vec<SessionManifestSummary>,
        /// Resources already annotated with session state before the failure.
        recorded_resources: Vec<SessionManifestSummary>,
    },
}

/// Apply generated sandbox resources through injectable Kubernetes boundaries.
fn apply_session_resources(
    plan: &SessionPlan,
    create_deployment: impl FnOnce(
        &str,
        &Deployment,
    ) -> std::result::Result<DeploymentSummary, MutationError>,
    create_service: impl FnOnce(&str, &Service) -> std::result::Result<ServiceSummary, MutationError>,
    wait_deployment_ready: impl FnOnce(
        &str,
        &str,
    ) -> std::result::Result<DeploymentSummary, MutationError>,
    mut record_state: impl FnMut(
        Vec<SessionManifestSummary>,
        SessionStatus,
    ) -> std::result::Result<
        Vec<SessionManifestSummary>,
        SessionStateRecordError,
    >,
) -> std::result::Result<SessionCreateApplyResult, SessionCreateApplyError> {
    if !plan.policy().allows(SessionOperation::Prepare) {
        return Err(SessionCreateApplyError::Policy(
            "policy does not allow session creation".to_owned(),
        ));
    }

    let deployment =
        session_deployment_manifest(plan).map_err(SessionCreateApplyError::Manifest)?;
    let service = session_service_manifest(plan).map_err(SessionCreateApplyError::Manifest)?;
    let manifests = session_manifest_summaries(plan).map_err(SessionCreateApplyError::Manifest)?;
    let Some(deployment_manifest) = manifests
        .iter()
        .find(|manifest| manifest.kind == "Deployment")
    else {
        return Err(SessionCreateApplyError::Manifest(
            SessionManifestBuildError::Summary("deployment manifest missing from session"),
        ));
    };
    let Some(service_manifest) = manifests.iter().find(|manifest| manifest.kind == "Service")
    else {
        return Err(SessionCreateApplyError::Manifest(
            SessionManifestBuildError::Summary("service manifest missing from session"),
        ));
    };
    let deployment_namespace = deployment_manifest.namespace.clone();
    let service_namespace = service_manifest.namespace.clone();
    let pending_after_deployment = manifests
        .iter()
        .filter(|manifest| manifest.kind != "Deployment")
        .cloned()
        .collect::<Vec<_>>();
    let pending_resources = manifests
        .into_iter()
        .filter(|manifest| manifest.kind != "Deployment" && manifest.kind != "Service")
        .collect::<Vec<_>>();
    let created_deployment = create_deployment(&deployment_namespace, &deployment)
        .map_err(SessionCreateApplyError::Mutation)?;
    let created_deployment_resource = SessionManifestSummary {
        kind: "Deployment".to_owned(),
        namespace: created_deployment.namespace,
        name: created_deployment.name,
    };
    let created_service = create_service(&service_namespace, &service).map_err(|error| {
        SessionCreateApplyError::PartialMutation {
            error,
            created_resources: vec![created_deployment_resource.clone()],
            pending_resources: pending_after_deployment,
            recorded_resources: Vec::new(),
        }
    })?;
    let created_service_resource = SessionManifestSummary {
        kind: "Service".to_owned(),
        namespace: created_service.namespace,
        name: created_service.name,
    };
    let created_resources = vec![
        created_deployment_resource.clone(),
        created_service_resource.clone(),
    ];
    let preparing_resources = record_state(created_resources.clone(), SessionStatus::Preparing)
        .map_err(|error| SessionCreateApplyError::PartialMutation {
            error: error.error,
            created_resources: created_resources.clone(),
            pending_resources: pending_resources.clone(),
            recorded_resources: error.recorded_resources,
        })?;

    let ready_deployment = wait_deployment_ready(
        &created_deployment_resource.namespace,
        &created_deployment_resource.name,
    )
    .map_err(|error| SessionCreateApplyError::PartialMutation {
        error,
        created_resources: vec![
            created_deployment_resource.clone(),
            created_service_resource.clone(),
        ],
        pending_resources: pending_resources.clone(),
        recorded_resources: preparing_resources.clone(),
    })?;

    let state_resources =
        record_state(created_resources.clone(), SessionStatus::Active).map_err(|error| {
            let recorded_resources = if error.recorded_resources.is_empty() {
                preparing_resources.clone()
            } else {
                error.recorded_resources
            };

            SessionCreateApplyError::PartialMutation {
                error: error.error,
                created_resources: created_resources.clone(),
                pending_resources: pending_resources.clone(),
                recorded_resources,
            }
        })?;

    Ok(SessionCreateApplyResult {
        created_resources: vec![
            created_deployment_resource.clone(),
            created_service_resource,
        ],
        pending_resources,
        readiness: SessionReadinessSummary {
            resource: created_deployment_resource,
            phase: ready_deployment.rollout.phase,
        },
        state: SessionStateRecordSummary {
            status: SessionStatus::Active,
            resources: state_resources,
        },
    })
}

/// Create the generated sandbox Deployment through the Kubernetes adapter.
async fn create_session_deployment(
    namespace: &str,
    deployment: &Deployment,
) -> std::result::Result<DeploymentSummary, MutationError> {
    let client = kply_k8s::load_kube_client().await?;
    kply_k8s::create_deployment(client, namespace, deployment)
        .await
        .map_err(|error| {
            MutationError::from_kubernetes_api_error("create sandbox Deployment", &error)
        })
}

/// Create the generated sandbox Service through the Kubernetes adapter.
async fn create_session_service(
    namespace: &str,
    service: &Service,
) -> std::result::Result<ServiceSummary, MutationError> {
    let client = kply_k8s::load_kube_client().await?;
    kply_k8s::create_service(client, namespace, service)
        .await
        .map_err(|error| MutationError::from_kubernetes_api_error("create sandbox Service", &error))
}

/// Wait until the generated sandbox Deployment reaches a complete rollout.
async fn wait_for_session_deployment_readiness(
    namespace: &str,
    name: &str,
) -> std::result::Result<DeploymentSummary, MutationError> {
    const READINESS_TIMEOUT: Duration = Duration::from_secs(30);
    const READINESS_INTERVAL: Duration = Duration::from_secs(1);

    let client = kply_k8s::load_kube_client().await?;
    let deadline = Instant::now() + READINESS_TIMEOUT;

    loop {
        let observed = kply_k8s::get_deployment(client.clone(), namespace, name)
            .await
            .map_err(|error| {
                MutationError::from_kubernetes_api_error(
                    "read sandbox Deployment readiness",
                    &error,
                )
            })?;
        if observed.rollout.phase == DeploymentRolloutPhase::Complete {
            return Ok(observed);
        }
        if Instant::now() >= deadline {
            return Err(MutationError {
                code: MutationErrorCode::KubernetesApi,
                message: format!(
                    "sandbox Deployment {namespace}/{name} did not become ready within {}s; current phase is {}",
                    READINESS_TIMEOUT.as_secs(),
                    observed.rollout.phase.as_str()
                ),
            });
        }

        tokio::time::sleep(READINESS_INTERVAL).await;
    }
}

/// Record current session state in Kubernetes resource metadata.
async fn record_session_state_metadata(
    resources: Vec<SessionManifestSummary>,
    status: SessionStatus,
) -> std::result::Result<Vec<SessionManifestSummary>, SessionStateRecordError> {
    let client = kply_k8s::load_kube_client()
        .await
        .map_err(|error| SessionStateRecordError {
            error,
            recorded_resources: Vec::new(),
        })?;
    let annotations = session_state_annotations(status);
    let mut recorded_resources = Vec::new();

    for resource in resources {
        match resource.kind.as_str() {
            "Deployment" => {
                kply_k8s::patch_deployment_annotations(
                    client.clone(),
                    &resource.namespace,
                    &resource.name,
                    &annotations,
                )
                .await
                .map_err(|error| {
                    let error = MutationError::from_kubernetes_api_error(
                        "record sandbox Deployment session state",
                        &error,
                    );
                    SessionStateRecordError {
                        error,
                        recorded_resources: recorded_resources.clone(),
                    }
                })?;
                recorded_resources.push(resource);
            }
            "Service" => {
                kply_k8s::patch_service_annotations(
                    client.clone(),
                    &resource.namespace,
                    &resource.name,
                    &annotations,
                )
                .await
                .map_err(|error| {
                    let error = MutationError::from_kubernetes_api_error(
                        "record sandbox Service session state",
                        &error,
                    );
                    SessionStateRecordError {
                        error,
                        recorded_resources: recorded_resources.clone(),
                    }
                })?;
                recorded_resources.push(resource);
            }
            _ => {}
        }
    }

    Ok(recorded_resources)
}

/// Build stable session state annotations for cluster metadata.
fn session_state_annotations(status: SessionStatus) -> BTreeMap<String, String> {
    BTreeMap::from([(
        SESSION_STATUS_ANNOTATION.to_owned(),
        status.as_str().to_owned(),
    )])
}

/// Render a deterministic dry-run list of generated sandbox manifests.
fn render_session_manifests(
    cli: &Cli,
    app_name: &str,
    wants_yaml: bool,
    image: Option<&str>,
    namespace: Option<&str>,
    time_to_live: Option<&str>,
    route_strategy: Option<&str>,
) -> Result<ExitCode> {
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

    let plan = match session_plan_from_config_with_policies(
        app,
        config.policies(),
        image,
        namespace,
        time_to_live,
        route_strategy,
    ) {
        Ok(plan) => plan,
        Err(SessionPlanBuildError::Config(message)) => {
            return render_session_plan_config_error(&message, cli.json);
        }
        Err(SessionPlanBuildError::Policy(message)) => {
            return render_session_plan_policy_error(&message, cli.json);
        }
        Err(SessionPlanBuildError::Usage(message)) => {
            return render_session_plan_error(&message, cli.json);
        }
    };
    if wants_yaml {
        let manifests = match session_manifest_values(&plan) {
            Ok(manifests) => manifests,
            Err(error) => return render_session_manifest_error(&error, cli.json),
        };
        print!("{}", render_yaml_documents(&manifests)?);
    } else if cli.json {
        let manifests = match session_manifest_documents(&plan) {
            Ok(manifests) => manifests,
            Err(error) => return render_session_manifest_error(&error, cli.json),
        };
        let value = serde_json::json!({
            "app": app_name,
            "session_id": plan.id(),
            "status": "generated",
            "manifests": manifests
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !cli.quiet {
        let manifests = match session_manifest_summaries(&plan) {
            Ok(manifests) => manifests,
            Err(error) => return render_session_manifest_error(&error, cli.json),
        };
        println!("kply session manifests {app_name}");
        println!("session_id: {}", plan.id());
        println!("manifests: {}", manifests.len());
        for manifest in manifests {
            println!(
                "  manifest: {} {}/{}",
                manifest.kind, manifest.namespace, manifest.name
            );
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Render a deterministic dry-run session plan.
fn render_session_plan(
    cli: &Cli,
    app_name: &str,
    image: Option<&str>,
    namespace: Option<&str>,
    time_to_live: Option<&str>,
    route_strategy: Option<&str>,
) -> Result<ExitCode> {
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

    let plan = match session_plan_from_config_with_policies(
        app,
        config.policies(),
        image,
        namespace,
        time_to_live,
        route_strategy,
    ) {
        Ok(plan) => plan,
        Err(SessionPlanBuildError::Config(message)) => {
            return render_session_plan_config_error(&message, cli.json);
        }
        Err(SessionPlanBuildError::Policy(message)) => {
            return render_session_plan_policy_error(&message, cli.json);
        }
        Err(SessionPlanBuildError::Usage(message)) => {
            return render_session_plan_error(&message, cli.json);
        }
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else if !cli.quiet {
        println!("kply session plan {app_name}");
        println!("id: {}", plan.id());
        println!("name: {}", plan.name());
        println!("workload: {}", plan.workload());
        println!("image: {}", plan.image());
        println!("planned_resources: {}", plan.planned_resources().len());
        for resource in plan.planned_resources() {
            println!("  resource: {resource}");
        }
        println!("planned_labels: {}", plan.planned_labels().len());
        for label in plan.planned_labels() {
            println!("  label: {label}");
        }
        println!("planned_annotations: {}", plan.planned_annotations().len());
        for annotation in plan.planned_annotations() {
            println!("  annotation: {annotation}");
        }
        println!("planned_checks: {}", plan.planned_checks().len());
        for check in plan.planned_checks() {
            println!("  check: {check}");
        }
        println!(
            "planned_cleanup_steps: {}",
            plan.planned_cleanup_steps().len()
        );
        for step in plan.planned_cleanup_steps() {
            println!("  cleanup: {step}");
        }
        println!(
            "required_permissions: {}",
            plan.required_permissions().len()
        );
        for permission in plan.required_permissions() {
            println!("  permission: {permission}");
        }
        println!(
            "unsupported_feature_warnings: {}",
            plan.unsupported_feature_warnings().len()
        );
        for warning in plan.unsupported_feature_warnings() {
            println!("  unsupported: {warning}");
        }
        println!("risk_notes: {}", plan.risk_notes().len());
        for note in plan.risk_notes() {
            println!("  risk: {note}");
        }
        println!(
            "route_selector: {}",
            plan.route_selector()
                .map_or("<none>".to_owned(), ToString::to_string)
        );
        println!(
            "policy_operations: {}",
            plan.policy().allowed_operations().len()
        );
        println!("status: {}", plan.status());
        if let Some(time_to_live) = plan.time_to_live() {
            println!("ttl: {time_to_live}");
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Stable manifest identifier rendered by `kply session manifests`.
#[derive(Clone, Debug, Serialize)]
struct SessionManifestSummary {
    kind: String,
    namespace: String,
    name: String,
}

/// Serialized manifest document rendered for agent-oriented JSON output.
#[derive(Debug, Serialize)]
struct SessionManifestDocument {
    kind: String,
    namespace: String,
    name: String,
    object: serde_json::Value,
}

/// Error produced while building a session plan from config and CLI input.
#[derive(Debug)]
enum SessionPlanBuildError {
    /// Configuration-derived data could not be converted into the core model.
    Config(String),
    /// Configured policy boundaries reject the requested session plan.
    Policy(String),
    /// User-provided CLI input was invalid for session planning.
    Usage(String),
}

/// Build stable manifest summaries from generated session resources.
fn session_manifest_summaries(
    plan: &SessionPlan,
) -> std::result::Result<Vec<SessionManifestSummary>, SessionManifestBuildError> {
    let _deployment = sandbox_deployment_manifest(plan)?;
    let _service = sandbox_service_manifest(plan)?;
    let mut manifests = vec![
        planned_manifest_summary(plan, SANDBOX_WORKLOAD_KIND, "Deployment")?,
        planned_manifest_summary(plan, "Service", "Service")?,
    ];

    if plan.route_selector().is_some() {
        let _route = sandbox_route_placeholder_manifest(plan)?;
        manifests.push(planned_manifest_summary(plan, "HTTPRoute", "ConfigMap")?);
    }

    Ok(manifests)
}

/// Build serialized Kubernetes manifest values from a session plan.
fn session_manifest_values(
    plan: &SessionPlan,
) -> std::result::Result<Vec<serde_json::Value>, SessionManifestBuildError> {
    let deployment = sandbox_deployment_manifest(plan)?;
    let service = sandbox_service_manifest(plan)?;
    let mut manifests = vec![
        serde_json::to_value(deployment).map_err(SessionManifestBuildError::Serialize)?,
        serde_json::to_value(service).map_err(SessionManifestBuildError::Serialize)?,
    ];

    if plan.route_selector().is_some() {
        let route = sandbox_route_placeholder_manifest(plan)?;
        manifests.push(serde_json::to_value(route).map_err(SessionManifestBuildError::Serialize)?);
    }

    Ok(manifests)
}

/// Build the typed Kubernetes Deployment object used by `session create --apply`.
fn session_deployment_manifest(
    plan: &SessionPlan,
) -> std::result::Result<Deployment, SessionManifestBuildError> {
    let manifest = sandbox_deployment_manifest(plan)?;
    let value = serde_json::to_value(manifest).map_err(SessionManifestBuildError::Serialize)?;
    serde_json::from_value(value).map_err(SessionManifestBuildError::Serialize)
}

/// Build the typed Kubernetes Service object used by `session create --apply`.
fn session_service_manifest(
    plan: &SessionPlan,
) -> std::result::Result<Service, SessionManifestBuildError> {
    let manifest = sandbox_service_manifest(plan)?;
    let value = serde_json::to_value(manifest).map_err(SessionManifestBuildError::Serialize)?;
    serde_json::from_value(value).map_err(SessionManifestBuildError::Serialize)
}

/// Pair generated manifest identities with full Kubernetes object bodies.
/// Summary and value helpers must generate the same resource sequence.
fn session_manifest_documents(
    plan: &SessionPlan,
) -> std::result::Result<Vec<SessionManifestDocument>, SessionManifestBuildError> {
    let summaries = session_manifest_summaries(plan)?;
    let values = session_manifest_values(plan)?;
    debug_assert_eq!(
        summaries.len(),
        values.len(),
        "session manifest summaries and values must describe the same resources",
    );

    Ok(summaries
        .into_iter()
        .zip(values)
        .map(|(summary, object)| SessionManifestDocument {
            kind: summary.kind,
            namespace: summary.namespace,
            name: summary.name,
            object,
        })
        .collect())
}

/// Render manifests as a Kubernetes-style multi-document YAML stream.
fn render_yaml_documents(manifests: &[serde_json::Value]) -> Result<String> {
    let mut output = String::new();
    for manifest in manifests {
        output.push_str("---\n");
        output.push_str(&serde_norway::to_string(manifest)?);
    }

    Ok(output)
}

/// Error produced while deriving the session manifest output summary.
#[derive(Debug)]
enum SessionManifestBuildError {
    /// Core manifest generation rejected the session plan.
    Manifest(SandboxManifestError),
    /// Manifest serialization failed before rendering output.
    Serialize(serde_json::Error),
    /// CLI summary extraction could not find an expected planned resource.
    Summary(&'static str),
}

impl fmt::Display for SessionManifestBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(error) => write!(formatter, "{error}"),
            Self::Serialize(error) => write!(formatter, "{error}"),
            Self::Summary(message) => formatter.write_str(message),
        }
    }
}

impl From<SandboxManifestError> for SessionManifestBuildError {
    fn from(error: SandboxManifestError) -> Self {
        Self::Manifest(error)
    }
}

/// Return the generated manifest identity for one planned resource kind.
fn planned_manifest_summary(
    plan: &SessionPlan,
    planned_kind: &str,
    manifest_kind: &str,
) -> std::result::Result<SessionManifestSummary, SessionManifestBuildError> {
    let resource = plan
        .planned_resources()
        .iter()
        .find(|resource| resource.kind() == planned_kind)
        .ok_or(SessionManifestBuildError::Summary(
            "planned manifest resource missing",
        ))?;

    Ok(SessionManifestSummary {
        kind: manifest_kind.to_owned(),
        namespace: resource.namespace().to_owned(),
        name: resource.name().to_owned(),
    })
}

/// Build a session plan from static app configuration and CLI overrides.
#[cfg(test)]
fn session_plan_from_config(
    app: &AppConfig,
    image: Option<&str>,
    namespace: Option<&str>,
    time_to_live: Option<&str>,
    route_strategy: Option<&str>,
) -> std::result::Result<SessionPlan, SessionPlanBuildError> {
    session_plan_from_config_with_policies(
        app,
        &PolicyConfigs::default(),
        image,
        namespace,
        time_to_live,
        route_strategy,
    )
}

/// Build a session plan from app config, policy config, and CLI overrides.
fn session_plan_from_config_with_policies(
    app: &AppConfig,
    policies: &PolicyConfigs,
    image: Option<&str>,
    namespace: Option<&str>,
    time_to_live: Option<&str>,
    route_strategy: Option<&str>,
) -> std::result::Result<SessionPlan, SessionPlanBuildError> {
    let session_id = SessionId::new(session_token(app.name(), "plan")).map_err(|error| {
        SessionPlanBuildError::Config(format!("invalid generated session id: {error}"))
    })?;
    let session_name = SessionName::new(session_token(app.name(), "session")).map_err(|error| {
        SessionPlanBuildError::Config(format!("invalid generated session name: {error}"))
    })?;
    let namespace = namespace.unwrap_or_else(|| app.namespace());
    let image = match image {
        Some(image) => ImageRef::new(image).map_err(|error| {
            SessionPlanBuildError::Usage(format!("invalid session plan image: {error}"))
        })?,
        None => {
            let image = app.default_image().ok_or_else(|| {
                SessionPlanBuildError::Usage(format!(
                    "app `{}` has no image; pass --image",
                    app.name()
                ))
            })?;
            ImageRef::new(image).map_err(|error| {
                SessionPlanBuildError::Config(format!("invalid configured image: {error}"))
            })?
        }
    };

    let workload =
        WorkloadRef::new(namespace, app.workload_kind(), app.workload()).map_err(|error| {
            let message = format!(
                "invalid configured workload `{}/{}`: {error}",
                app.workload_kind(),
                app.workload()
            );
            if namespace == app.namespace() {
                SessionPlanBuildError::Config(message)
            } else {
                SessionPlanBuildError::Usage(message)
            }
        })?;
    let time_to_live = time_to_live
        .map(TimeToLive::new)
        .transpose()
        .map_err(|error| SessionPlanBuildError::Usage(error.to_string()))?;
    let route_strategy = resolve_session_route_strategy(app, route_strategy);
    let route_selector = match route_strategy {
        "header" => Some(
            RouteSelector::header(SESSION_HEADER_NAME, session_id.as_str()).map_err(|error| {
                SessionPlanBuildError::Config(format!("invalid session route selector: {error}"))
            })?,
        ),
        "host" => {
            let hostname = format!("{}.{}.kply.local", session_id.as_str(), namespace);
            Some(RouteSelector::host(hostname).map_err(|error| {
                SessionPlanBuildError::Config(format!("invalid session route selector: {error}"))
            })?)
        }
        // Preview routing is represented by Service-targeted checks, not a request selector.
        ROUTE_STRATEGY_PREVIEW | ROUTE_STRATEGY_PREVIEW_SERVICE => None,
        // No routing keeps the sandbox reachable only through its direct Service.
        ROUTE_STRATEGY_NONE => None,
        value => {
            return Err(SessionPlanBuildError::Usage(format!(
                "unsupported route strategy `{value}`; expected {}",
                supported_route_strategies()
            )));
        }
    };
    let policy_decision = evaluate_session_planning_policies(
        policies,
        namespace,
        app.workload_kind(),
        &image,
        time_to_live.as_ref(),
        route_strategy,
    )?;
    let planned_resources = planned_session_resources(
        namespace,
        SANDBOX_WORKLOAD_KIND,
        session_id.as_str(),
        route_strategy,
    )
    .map_err(|error| {
        SessionPlanBuildError::Config(format!("invalid planned kubernetes resource: {error}"))
    })?;
    let planned_labels =
        planned_session_labels(app.name(), session_id.as_str(), session_name.as_str()).map_err(
            |error| {
                SessionPlanBuildError::Config(format!("invalid planned label metadata: {error}"))
            },
        )?;
    let planned_annotations = planned_session_annotations(&workload, &image, route_strategy)
        .map_err(|error| {
            SessionPlanBuildError::Config(format!("invalid planned annotation metadata: {error}"))
        })?;
    let planned_checks = planned_session_checks(
        namespace,
        &workload,
        &image,
        route_strategy,
        session_id.as_str(),
    )
    .map_err(|error| {
        SessionPlanBuildError::Config(format!("invalid planned check metadata: {error}"))
    })?;
    let planned_cleanup_steps = planned_session_cleanup_steps(
        namespace,
        SANDBOX_WORKLOAD_KIND,
        session_id.as_str(),
        route_strategy,
    )
    .map_err(|error| {
        SessionPlanBuildError::Config(format!("invalid planned cleanup step metadata: {error}"))
    })?;
    let required_permissions = required_session_permissions(SANDBOX_WORKLOAD_KIND, route_strategy)
        .map_err(|error| {
            SessionPlanBuildError::Config(format!("invalid required permission metadata: {error}"))
        })?;
    let unsupported_feature_warnings = unsupported_session_feature_warnings(route_strategy)
        .map_err(|error| {
            SessionPlanBuildError::Config(format!(
                "invalid unsupported feature warning metadata: {error}"
            ))
        })?;
    let risk_notes = if policy_decision.database_risk_warnings_enabled {
        planned_session_risk_notes(app).map_err(|error| {
            SessionPlanBuildError::Config(format!("invalid risk note metadata: {error}"))
        })?
    } else {
        Vec::new()
    };

    let mut plan = SessionPlan::new(
        session_id,
        session_name,
        workload,
        image,
        policy_decision.session_policy,
    );
    plan = plan.with_planned_resources(planned_resources);
    plan = plan.with_planned_labels(planned_labels).map_err(|error| {
        SessionPlanBuildError::Config(format!("invalid planned label metadata: {error}"))
    })?;
    plan = plan.with_planned_annotations(planned_annotations);
    plan = plan.with_planned_checks(planned_checks);
    plan = plan.with_planned_cleanup_steps(planned_cleanup_steps);
    plan = plan.with_required_permissions(required_permissions);
    plan = plan.with_unsupported_feature_warnings(unsupported_feature_warnings);
    plan = plan.with_risk_notes(risk_notes);
    if let Some(route_selector) = route_selector {
        plan = plan.with_route_selector(route_selector);
    }
    if let Some(time_to_live) = time_to_live {
        plan = plan.with_time_to_live(time_to_live);
    }

    Ok(plan)
}

/// Policy decision returned by planning policy evaluation.
#[derive(Debug)]
struct SessionPlanningPolicyDecision {
    session_policy: SessionPolicy,
    database_risk_warnings_enabled: bool,
}

/// Evaluate configured policy entries against one requested session plan.
fn evaluate_session_planning_policies(
    policies: &PolicyConfigs,
    namespace: &str,
    workload_kind: &str,
    image: &ImageRef,
    time_to_live: Option<&TimeToLive>,
    route_strategy: &str,
) -> std::result::Result<SessionPlanningPolicyDecision, SessionPlanBuildError> {
    let enabled_policies = policies
        .entries()
        .iter()
        .filter(|policy| policy.enabled())
        .collect::<Vec<_>>();

    if enabled_policies.is_empty() {
        return Ok(SessionPlanningPolicyDecision {
            session_policy: SessionPolicy::sandbox(),
            database_risk_warnings_enabled: true,
        });
    }

    let mut denials = Vec::new();
    for policy in enabled_policies {
        match evaluate_session_planning_policy(
            policy,
            namespace,
            workload_kind,
            image,
            time_to_live,
            route_strategy,
        ) {
            Ok(decision) => return Ok(decision),
            Err(reason) => denials.push(format!("policy `{}` {reason}", policy.name())),
        }
    }

    Err(SessionPlanBuildError::Policy(format!(
        "no enabled policy allows this session plan: {}",
        denials.join("; ")
    )))
}

/// Evaluate one policy entry against one requested session plan.
fn evaluate_session_planning_policy(
    policy: &PolicyConfig,
    namespace: &str,
    workload_kind: &str,
    image: &ImageRef,
    time_to_live: Option<&TimeToLive>,
    route_strategy: &str,
) -> std::result::Result<SessionPlanningPolicyDecision, String> {
    if !policy.allowed_namespaces().is_empty()
        && !policy
            .allowed_namespaces()
            .iter()
            .any(|allowed| allowed == namespace)
    {
        return Err(format!("does not allow namespace `{namespace}`"));
    }

    if !policy.allowed_workload_kinds().is_empty()
        && !policy
            .allowed_workload_kinds()
            .iter()
            .any(|allowed| allowed == workload_kind)
    {
        return Err(format!("does not allow workload kind `{workload_kind}`"));
    }

    if !policy.allowed_image_registries().is_empty() {
        let registry = image_registry_host(image.as_str());
        if !policy
            .allowed_image_registries()
            .iter()
            .any(|allowed| allowed == registry)
        {
            return Err(format!("does not allow image registry `{registry}`"));
        }
    }

    if !policy.allowed_route_strategies().is_empty() {
        let Some(strategy) = policy_route_strategy(route_strategy) else {
            return Err(format!("does not allow route strategy `{route_strategy}`"));
        };
        if !policy.allowed_route_strategies().contains(&strategy) {
            return Err(format!("does not allow route strategy `{route_strategy}`"));
        }
    }

    if let (Some(max_session_ttl), Some(time_to_live)) = (policy.max_session_ttl(), time_to_live)
        && compact_duration_seconds(time_to_live.as_str())
            > compact_duration_seconds(max_session_ttl)
    {
        return Err(format!(
            "does not allow ttl `{}` above max_session_ttl `{max_session_ttl}`",
            time_to_live.as_str()
        ));
    }

    if route_strategy_creates_route_object(route_strategy)
        && matches!(
            policy.mutation_mode(),
            Some(MutationModePolicy::ReadOnly | MutationModePolicy::SandboxOnly)
        )
    {
        return Err(format!(
            "does not allow route mutation for route strategy `{route_strategy}`"
        ));
    }

    Ok(SessionPlanningPolicyDecision {
        session_policy: session_policy_for_mutation_mode(policy.mutation_mode())?,
        database_risk_warnings_enabled: policy.database_risk_warnings()
            != Some(DatabaseRiskWarningPolicy::Disabled),
    })
}

/// Return the session operations represented by a mutation mode policy.
fn session_policy_for_mutation_mode(
    mutation_mode: Option<MutationModePolicy>,
) -> std::result::Result<SessionPolicy, String> {
    match mutation_mode {
        Some(MutationModePolicy::ReadOnly) => {
            SessionPolicy::new([SessionOperation::Inspect, SessionOperation::Plan])
                .map_err(|error| format!("invalid read-only session policy: {error}"))
        }
        Some(MutationModePolicy::SandboxOnly) => SessionPolicy::new([
            SessionOperation::Inspect,
            SessionOperation::Plan,
            SessionOperation::Prepare,
            SessionOperation::Verify,
            SessionOperation::Cleanup,
        ])
        .map_err(|error| format!("invalid sandbox-only session policy: {error}")),
        Some(MutationModePolicy::RouteMutation) | None => Ok(SessionPolicy::sandbox()),
        Some(_) => Ok({
            // `MutationModePolicy` is non-exhaustive outside `kply-config`.
            // Unknown future modes keep the current sandbox-safe operation set
            // until this CLI deliberately adopts their semantics.
            SessionPolicy::sandbox()
        }),
    }
}

/// Convert a CLI route strategy spelling to the policy route strategy domain.
fn policy_route_strategy(route_strategy: &str) -> Option<RouteStrategy> {
    match route_strategy {
        "header" => Some(RouteStrategy::Header),
        "host" => Some(RouteStrategy::Host),
        ROUTE_STRATEGY_PREVIEW | ROUTE_STRATEGY_PREVIEW_SERVICE => Some(RouteStrategy::Preview),
        // `none` deliberately has no config-level route strategy variant. A
        // policy with an explicit route strategy allowlist rejects it instead
        // of treating "no route mutation" as one of the allowed route types.
        ROUTE_STRATEGY_NONE => None,
        _ => None,
    }
}

/// Return the explicit registry host for an image reference.
fn image_registry_host(image: &str) -> &str {
    let first_component = image.split('/').next().unwrap_or(image);
    let has_registry_port = first_component
        .rsplit_once(':')
        .is_some_and(|(_, port)| !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()));
    if first_component == "localhost" || first_component.contains('.') || has_registry_port {
        first_component
    } else {
        "docker.io"
    }
}

/// Convert a validated compact duration to seconds for policy comparisons.
fn compact_duration_seconds(value: &str) -> u128 {
    let (digits, unit) = value.split_at(value.len().saturating_sub(1));
    let value = digits.parse::<u128>().unwrap_or(u128::MAX);
    match unit {
        "s" => value,
        "m" => value.saturating_mul(60),
        "h" => value.saturating_mul(60 * 60),
        "d" => value.saturating_mul(24 * 60 * 60),
        _ => u128::MAX,
    }
}

/// Build Kubernetes resource identities created by one planned sandbox session.
fn planned_session_resources(
    namespace: &str,
    workload_kind: &str,
    session_id: &str,
    route_strategy: &str,
) -> std::result::Result<Vec<KubernetesResourceRef>, String> {
    let mut resources = vec![
        (
            workload_kind,
            planned_resource_token(session_id, "workload"),
        ),
        ("Service", planned_resource_token(session_id, "service")),
    ];
    if route_strategy_creates_route_object(route_strategy) {
        resources.push(("HTTPRoute", planned_resource_token(session_id, "route")));
    }

    resources
        .into_iter()
        .map(|(kind, name)| {
            KubernetesResourceRef::new(namespace, kind, name).map_err(|error| error.to_string())
        })
        .collect()
}

/// Build ownership labels shared by all sandbox session resources.
fn planned_session_labels(
    app_name: &str,
    session_id: &str,
    session_name: &str,
) -> std::result::Result<Vec<MetadataEntry>, String> {
    [
        ("kply.dev/app", app_name),
        ("kply.dev/managed-by", "kply"),
        ("kply.dev/session-id", session_id),
        ("kply.dev/session-name", session_name),
    ]
    .into_iter()
    .map(|(key, value)| MetadataEntry::new_label(key, value).map_err(|error| error.to_string()))
    .collect()
}

/// Build audit and routing annotations shared by all sandbox session resources.
fn planned_session_annotations(
    workload: &WorkloadRef,
    image: &ImageRef,
    route_strategy: &str,
) -> std::result::Result<Vec<MetadataEntry>, String> {
    let workload = workload.to_string();
    let image = image.to_string();
    [
        ("kply.dev/image", image.as_str()),
        ("kply.dev/route-strategy", route_strategy),
        ("kply.dev/workload", workload.as_str()),
    ]
    .into_iter()
    .map(|(key, value)| MetadataEntry::new(key, value).map_err(|error| error.to_string()))
    .collect()
}

/// Build checks expected for one planned sandbox session.
fn planned_session_checks(
    namespace: &str,
    workload: &WorkloadRef,
    image: &ImageRef,
    route_strategy: &str,
    session_id: &str,
) -> std::result::Result<Vec<PlannedCheck>, String> {
    let workload = workload.to_string();
    let image = image.to_string();
    let service = ServiceRef::new(namespace, planned_resource_token(session_id, "service"))
        .map_err(|error| error.to_string())?
        .to_string();
    let route_ready_target = route_strategy_has_route_check(route_strategy).then_some(
        if route_strategy_uses_preview_service(route_strategy) {
            service.as_str()
        } else {
            route_strategy
        },
    );
    let mut checks = vec![
        ("image_pull", image.as_str()),
        ("service_endpoints", service.as_str()),
        ("workload_ready", workload.as_str()),
    ];
    if let Some(route_ready_target) = route_ready_target {
        checks.insert(1, ("route_ready", route_ready_target));
    }
    checks
        .into_iter()
        .map(|(name, target)| PlannedCheck::new(name, target).map_err(|error| error.to_string()))
        .collect()
}

/// Build cleanup steps for resources created by one planned sandbox session.
fn planned_session_cleanup_steps(
    namespace: &str,
    workload_kind: &str,
    session_id: &str,
    route_strategy: &str,
) -> std::result::Result<Vec<PlannedCleanupStep>, String> {
    let workload = KubernetesResourceRef::new(
        namespace,
        workload_kind,
        planned_resource_token(session_id, "workload"),
    )
    .map_err(|error| error.to_string())?
    .to_string();
    let service = KubernetesResourceRef::new(
        namespace,
        "Service",
        planned_resource_token(session_id, "service"),
    )
    .map_err(|error| error.to_string())?
    .to_string();
    let mut steps = vec![
        ("delete_service", service.as_str()),
        ("delete_workload", workload.as_str()),
    ];
    let route;
    if route_strategy_creates_route_object(route_strategy) {
        route = KubernetesResourceRef::new(
            namespace,
            "HTTPRoute",
            planned_resource_token(session_id, "route"),
        )
        .map_err(|error| error.to_string())?
        .to_string();
        steps.insert(0, ("delete_route", route.as_str()));
    }

    steps
        .into_iter()
        .map(|(action, target)| {
            PlannedCleanupStep::new(action, target).map_err(|error| error.to_string())
        })
        .collect()
}

/// Build Kubernetes permissions required to create and clean up a session.
fn required_session_permissions(
    workload_kind: &str,
    route_strategy: &str,
) -> std::result::Result<Vec<RequiredPermission>, String> {
    let workload_resource = workload_permission_resource(workload_kind)?;
    let mut permission_inputs = vec![
        (
            "apps",
            workload_resource.as_str(),
            vec!["create", "delete", "get", "patch"],
        ),
        ("", "pods", vec!["get", "list", "watch"]),
        ("", "services", vec!["create", "delete", "get", "patch"]),
    ];
    if route_strategy_creates_route_object(route_strategy) {
        permission_inputs.push((
            "gateway.networking.k8s.io",
            "httproutes",
            vec!["create", "delete", "get"],
        ));
    }

    let mut permissions = permission_inputs
        .into_iter()
        .map(|(api_group, resource, verbs)| {
            RequiredPermission::new(api_group, resource, verbs).map_err(|error| error.to_string())
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    permissions.sort_unstable();
    permissions.dedup();
    Ok(permissions)
}

/// Build unsupported feature warnings for the requested route strategy.
fn unsupported_session_feature_warnings(
    route_strategy: &str,
) -> std::result::Result<Vec<UnsupportedFeatureWarning>, String> {
    let warning_inputs = match route_strategy {
        ROUTE_STRATEGY_PREVIEW | ROUTE_STRATEGY_PREVIEW_SERVICE => vec![(
            UNSUPPORTED_FEATURE_EDGE_ROUTE_VALIDATION,
            UNSUPPORTED_REASON_PREVIEW_SKIPS_EDGE_ROUTE_VALIDATION,
        )],
        ROUTE_STRATEGY_NONE => vec![(
            UNSUPPORTED_FEATURE_EDGE_ROUTE_VALIDATION,
            UNSUPPORTED_REASON_NONE_SKIPS_ROUTE_VALIDATION,
        )],
        _ => Vec::new(),
    };

    warning_inputs
        .into_iter()
        .map(|(feature, reason)| {
            UnsupportedFeatureWarning::new(feature, reason).map_err(|error| error.to_string())
        })
        .collect()
}

/// Build risk notes for app shapes that need human review before release decisions.
fn planned_session_risk_notes(app: &AppConfig) -> std::result::Result<Vec<RiskNote>, String> {
    let target = database_like_app_target(app);
    let notes = match target {
        Some(target) => vec![
            RiskNote::new(
                RISK_CATEGORY_DATABASE,
                RISK_SEVERITY_WARNING,
                target,
                RISK_REASON_DATABASE_REFERENCE_REQUIRES_MANUAL_REVIEW,
            )
            .map_err(|error| error.to_string())?,
        ],
        None => Vec::new(),
    };

    Ok(notes)
}

fn database_like_app_target(app: &AppConfig) -> Option<String> {
    let candidates = [
        ("app", app.name()),
        ("workload", app.workload()),
        ("service", app.service()),
        ("image", app.default_image().unwrap_or_default()),
    ];

    candidates.into_iter().find_map(|(field, value)| {
        contains_database_token(value).then(|| format!("{field}:{value}"))
    })
}

fn contains_database_token(value: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| {
            matches!(
                token.to_ascii_lowercase().as_str(),
                "db" | "database"
                    | "mysql"
                    | "mariadb"
                    | "postgres"
                    | "postgresql"
                    | "mongo"
                    | "mongodb"
                    | "redis"
            )
        })
}

fn workload_permission_resource(workload_kind: &str) -> std::result::Result<String, String> {
    match workload_kind {
        "DaemonSet" => Ok("daemonsets".to_owned()),
        "Deployment" => Ok("deployments".to_owned()),
        "ReplicaSet" => Ok("replicasets".to_owned()),
        "StatefulSet" => Ok("statefulsets".to_owned()),
        value => Err(format!(
            "unsupported workload kind `{value}` for required permission planning; expected {}",
            supported_workload_kinds()
        )),
    }
}

fn supported_workload_kinds() -> String {
    ["DaemonSet", "Deployment", "ReplicaSet", "StatefulSet"].join(", ")
}

fn planned_resource_token(session_id: &str, suffix: &str) -> String {
    unique_token(session_id, suffix)
}

fn supported_route_strategies() -> String {
    [
        ROUTE_STRATEGY_AUTO,
        ROUTE_STRATEGY_NONE,
        ROUTE_STRATEGY_PREVIEW_SERVICE,
    ]
    .into_iter()
    .chain(SUPPORTED_ROUTE_STRATEGIES.iter().map(RouteStrategy::as_str))
    .collect::<Vec<_>>()
    .join(", ")
}

/// Resolve CLI route strategy input to a concrete configured strategy.
fn resolve_session_route_strategy<'a>(
    app: &'a AppConfig,
    route_strategy: Option<&'a str>,
) -> &'a str {
    match route_strategy {
        Some(ROUTE_STRATEGY_AUTO) | None => app.route_strategy().as_str(),
        Some(route_strategy) => route_strategy,
    }
}

/// Return whether a strategy should plan a Kubernetes route object.
fn route_strategy_creates_route_object(route_strategy: &str) -> bool {
    !matches!(
        route_strategy,
        ROUTE_STRATEGY_PREVIEW | ROUTE_STRATEGY_PREVIEW_SERVICE | ROUTE_STRATEGY_NONE
    )
}

/// Return whether a strategy has a route readiness check.
fn route_strategy_has_route_check(route_strategy: &str) -> bool {
    route_strategy != ROUTE_STRATEGY_NONE
}

/// Return whether route readiness targets the sandbox Service directly.
fn route_strategy_uses_preview_service(route_strategy: &str) -> bool {
    matches!(
        route_strategy,
        ROUTE_STRATEGY_PREVIEW | ROUTE_STRATEGY_PREVIEW_SERVICE
    )
}

/// Build a dry-run route plan from a session id and optional namespace.
fn route_plan_from_session(
    session_id: &str,
    namespace: Option<&str>,
) -> std::result::Result<RoutePlanOutput, String> {
    let labels = route_plan_ownership_labels(session_id)?;
    let cleanup_selector =
        gateway_route_cleanup_selector(&labels).map_err(|error| error.to_string())?;
    let (planned_resource, cleanup_target, unsupported_routes) = match namespace {
        Some(namespace) => {
            let route = KubernetesResourceRef::new(
                namespace,
                "HTTPRoute",
                planned_resource_token(session_id, "route"),
            )
            .map_err(|error| format!("invalid planned route resource: {error}"))?;
            let cleanup_target = gateway_http_route_cleanup_target(&route, &labels)
                .map_err(|error| error.to_string())?;
            (
                Some(SessionManifestSummary {
                    kind: "HTTPRoute".to_owned(),
                    namespace: route.namespace().to_owned(),
                    name: route.name().to_owned(),
                }),
                Some(cleanup_target),
                Vec::new(),
            )
        }
        None => (
            None,
            None,
            vec![UnsupportedRouteOutput {
                strategy: "gateway_api",
                feature: "temporary_http_route",
                reason: "namespace_required",
                action: "rerun_with_namespace",
            }],
        ),
    };

    Ok(RoutePlanOutput {
        session_id: session_id.to_owned(),
        status: "planned",
        mutation: "not_applied",
        apply: false,
        route_kind: "HTTPRoute",
        planned_resource,
        cleanup_target,
        cleanup_selector,
        unsupported_routes,
    })
}

/// Build a dry-run route cleanup plan from a session id and optional namespace.
fn route_cleanup_from_session(
    session_id: &str,
    namespace: Option<&str>,
) -> std::result::Result<RouteCleanupOutput, String> {
    let route_plan = route_plan_from_session(session_id, namespace)?;

    Ok(RouteCleanupOutput {
        session_id: route_plan.session_id,
        status: "planned",
        mutation: "not_applied",
        cleanup: false,
        route_kind: route_plan.route_kind,
        cleanup_target: route_plan.cleanup_target,
        cleanup_selector: route_plan.cleanup_selector,
    })
}

/// Build ownership labels required for route planning.
fn route_plan_ownership_labels(
    session_id: &str,
) -> std::result::Result<Vec<MetadataEntry>, String> {
    [
        ("kply.dev/app", "unknown"),
        ("kply.dev/managed-by", "kply"),
        ("kply.dev/session-id", session_id),
        ("kply.dev/session-name", session_id),
    ]
    .into_iter()
    .map(|(key, value)| {
        MetadataEntry::new_label(key, value)
            .map_err(|error| format!("invalid route ownership label `{key}`: {error}"))
    })
    .collect()
}

/// Derive a stable Kubernetes-compatible session token from an app name.
fn session_token(app_name: &str, suffix: &str) -> String {
    unique_token(app_name, suffix)
}

fn unique_token(value: &str, suffix: &str) -> String {
    let token = normalized_token_prefix(value);
    if token.len() + suffix.len() < 63 {
        return append_token_suffix(token, suffix, None);
    }

    append_token_suffix(token, suffix, Some(stable_token_hash(value)))
}

fn normalized_token_prefix(value: &str) -> String {
    let mut token = String::with_capacity(value.len());
    let mut previous_was_separator = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            token.push(character);
            previous_was_separator = false;
        } else if !previous_was_separator && !token.is_empty() {
            token.push('-');
            previous_was_separator = true;
        }
    }

    while token.ends_with('-') {
        token.pop();
    }

    if token.is_empty() {
        token.push_str("session");
    }

    token
}

fn append_token_suffix(mut token: String, suffix: &str, hash: Option<String>) -> String {
    let hash_len = hash.as_ref().map_or(0, String::len);
    let separators = if hash.is_some() { 2 } else { 1 };
    let max_prefix_len = 63usize.saturating_sub(suffix.len() + hash_len + separators);
    if token.len() > max_prefix_len {
        token.truncate(max_prefix_len);
        while token.ends_with('-') {
            token.pop();
        }
    }

    if token.is_empty() {
        token.push_str("session");
    }

    token.push('-');
    if let Some(hash) = hash {
        token.push_str(&hash);
        token.push('-');
    }
    token.push_str(suffix);

    token
}

/// Compute a deterministic non-cryptographic 8-hex-digit token hash.
///
/// This uses a 32-bit FNV-1 style multiply-then-XOR fold for stable name
/// derivation when long prefixes must be truncated. It is not for security.
fn stable_token_hash(value: &str) -> String {
    let hash = value.bytes().fold(0x811c9dc5u32, |hash, byte| {
        hash.wrapping_mul(0x01000193) ^ u32::from(byte)
    });
    format!("{hash:08x}")
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

/// Render one configured application graph.
fn render_app_graph(cli: &Cli, app_name: &str) -> Result<ExitCode> {
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

    let graph = match app_graph_from_config(app) {
        Ok(graph) => graph,
        Err(message) => return render_app_graph_config_error(&message),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&graph)?);
    } else if !cli.quiet {
        render_app_graph_text(app.name(), &graph);
    }

    Ok(ExitCode::SUCCESS)
}

/// Render a concise human-readable app graph summary.
fn render_app_graph_text(app_name: &str, graph: &AppGraph) {
    println!("kply app graph {app_name}");
    println!("workload: {}", graph.workload());
    println!("owned_pods: {}", graph.owned_pods().len());
    println!("selecting_services: {}", graph.selecting_services().len());
    for service in graph.selecting_services() {
        println!("  service: {service}");
    }
    println!("service_routes: {}", graph.service_routes().len());
    println!("probe_facts: {}", graph.probe_facts().len());
    println!("image_facts: {}", graph.image_facts().len());
    println!("resource_facts: {}", graph.resource_facts().len());
    println!("config_references: {}", graph.config_references().len());
    println!("secret_references: {}", graph.secret_references().len());
    println!(
        "relationship_confidences: {}",
        graph.relationship_confidences().len()
    );
    println!("warnings: {}", graph.warnings().len());
}

/// Build a provisional app graph from static app configuration.
fn app_graph_from_config(app: &AppConfig) -> Result<AppGraph, String> {
    let workload = WorkloadRef::new(app.namespace(), app.workload_kind(), app.workload()).map_err(
        |error| {
            format!(
                "invalid configured workload `{}/{}`: {error}",
                app.workload_kind(),
                app.workload()
            )
        },
    )?;
    let service = ServiceRef::new(app.namespace(), app.service())
        .map_err(|error| format!("invalid configured service `{}`: {error}", app.service()))?;
    let service_relationship = GraphRelationship::WorkloadServiceSelection {
        service: service.clone(),
    };

    Ok(AppGraph::new(workload)
        .with_selecting_services([service])
        .with_relationship_confidences([RelationshipConfidence::new(
            service_relationship,
            ConfidenceLevel::High,
        )]))
}

/// Render an app graph config-to-domain conversion error.
fn render_app_graph_config_error(message: &str) -> Result<ExitCode> {
    let value = serde_json::json!({
        "error": {
            "code": "config",
            "exit_code": EXIT_BLOCKING,
            "message": message
        }
    });
    eprintln!("{}", serde_json::to_string_pretty(&value)?);
    Ok(exit_code(EXIT_BLOCKING))
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

/// Render session plan construction errors.
fn render_session_plan_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "session_plan",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: session plan\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render session plan config-to-domain conversion errors.
fn render_session_plan_config_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "config",
                "exit_code": EXIT_BLOCKING,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: config\n\n{message}");
    }

    Ok(exit_code(EXIT_BLOCKING))
}

/// Render configured policy rejection errors.
fn render_session_plan_policy_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "policy",
                "exit_code": EXIT_BLOCKING,
                "message": message,
                "policy_violation": {
                    "reason": "policy_denied",
                    "violations": [
                        {
                            "message": message
                        }
                    ]
                }
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: policy\n\n{message}");
    }

    Ok(exit_code(EXIT_BLOCKING))
}

/// Render session status input errors.
fn render_session_status_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "session_status",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: session status\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render report show input errors.
fn render_report_show_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "report_show",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: report show\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render report export input errors.
fn render_report_export_error(message: &str) -> Result<ExitCode> {
    let value = serde_json::json!({
        "error": {
            "code": "report_export",
            "exit_code": EXIT_USAGE,
            "message": message
        }
    });
    eprintln!("{}", serde_json::to_string_pretty(&value)?);

    Ok(exit_code(EXIT_USAGE))
}

/// Render check run input errors.
fn render_check_run_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "check_run",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: check run\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render route plan input errors.
fn render_route_plan_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "route_plan",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: route plan\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render route apply input errors.
fn render_route_apply_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "route_apply",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: route apply\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render route cleanup input errors.
fn render_route_cleanup_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "route_cleanup",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: route cleanup\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render session cleanup input errors.
fn render_session_cleanup_error(message: &str, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "session_cleanup",
                "exit_code": EXIT_USAGE,
                "message": message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: session cleanup\n\n{message}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render session cleanup apply errors while mutating Kubernetes resources.
fn render_session_cleanup_apply_error(
    error: &SessionCleanupApplyError,
    wants_json: bool,
) -> Result<ExitCode> {
    let exit_code_value = match error.error.code {
        MutationErrorCode::ForbiddenAccess | MutationErrorCode::KubernetesApi => EXIT_BLOCKING,
        _ => EXIT_USAGE,
    };

    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "session_cleanup_apply",
                "exit_code": exit_code_value,
                "kubernetes_code": error.error.code,
                "message": error.error.message,
                "deletion_accepted_resources": error.deletion_accepted_resources,
                "pending_resources": error.pending_resources
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!(
            "kply error: session cleanup apply\n\n{}",
            error.error.message
        );
        if !error.deletion_accepted_resources.is_empty() || !error.pending_resources.is_empty() {
            eprintln!(
                "\ndeletion_accepted_resources: {}",
                error.deletion_accepted_resources.len()
            );
            for resource in &error.deletion_accepted_resources {
                eprintln!(
                    "  deletion_accepted: {} {}/{}",
                    resource.kind, resource.namespace, resource.name
                );
            }
            eprintln!("pending_resources: {}", error.pending_resources.len());
            for resource in &error.pending_resources {
                eprintln!(
                    "  pending: {} {}/{}",
                    resource.kind, resource.namespace, resource.name
                );
            }
        }
    }

    Ok(exit_code(exit_code_value))
}

/// Render session create apply errors while mutating Kubernetes resources.
fn render_session_create_apply_error(error: &MutationError, wants_json: bool) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": error.code.as_str(),
                "exit_code": EXIT_USAGE,
                "message": error.message,
                "apply_stage": EXPERIMENTAL_APPLY_STAGE
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: {}\n\n{}", error.code.as_str(), error.message);
        eprintln!("apply_stage: {EXPERIMENTAL_APPLY_STAGE}");
    }

    Ok(exit_code(EXIT_USAGE))
}

/// Render session create errors after one or more resources were already created.
fn render_session_create_partial_apply_error(
    error: &MutationError,
    created_resources: &[SessionManifestSummary],
    pending_resources: &[SessionManifestSummary],
    recorded_resources: &[SessionManifestSummary],
    wants_json: bool,
) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": error.code.as_str(),
                "exit_code": EXIT_BLOCKING,
                "message": error.message,
                "mutation": "partially_applied",
                "apply_stage": EXPERIMENTAL_APPLY_STAGE,
                "created_resources": created_resources,
                "pending_resources": pending_resources,
                "recorded_resources": recorded_resources,
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: {}\n\n{}", error.code.as_str(), error.message);
        eprintln!("mutation: partially_applied");
        eprintln!("apply_stage: {EXPERIMENTAL_APPLY_STAGE}");
        eprintln!("created_resources: {}", created_resources.len());
        for resource in created_resources {
            eprintln!(
                "  created: {} {}/{}",
                resource.kind, resource.namespace, resource.name
            );
        }
        eprintln!("pending_resources: {}", pending_resources.len());
        for resource in pending_resources {
            eprintln!(
                "  pending: {} {}/{}",
                resource.kind, resource.namespace, resource.name
            );
        }
        eprintln!("recorded_resources: {}", recorded_resources.len());
        for resource in recorded_resources {
            eprintln!(
                "  recorded: {} {}/{}",
                resource.kind, resource.namespace, resource.name
            );
        }
    }

    Ok(exit_code(EXIT_BLOCKING))
}

/// Render read-only Kubernetes discovery errors.
fn render_discovery_error(error: &DiscoveryError, wants_json: bool) -> Result<ExitCode> {
    let exit_code_value = match error.code {
        kply_k8s::DiscoveryErrorCode::ForbiddenAccess
        | kply_k8s::DiscoveryErrorCode::KubernetesApi => EXIT_BLOCKING,
        _ => EXIT_USAGE,
    };

    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": error.code.as_str(),
                "exit_code": exit_code_value,
                "message": error.message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: {}\n\n{}", error.code.as_str(), error.message);
    }

    Ok(exit_code(exit_code_value))
}

/// Render session manifest generation errors.
fn render_session_manifest_error(
    error: &SessionManifestBuildError,
    wants_json: bool,
) -> Result<ExitCode> {
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": "session_manifests",
                "exit_code": EXIT_BLOCKING,
                "message": error.to_string()
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: session manifests\n\n{error}");
    }

    Ok(exit_code(EXIT_BLOCKING))
}

/// Render read-only cluster facts resolved from kubeconfig.
fn render_cluster_info(cli: &Cli) -> Result<ExitCode> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
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

/// Validate configured policy boundaries.
fn render_policy_check(cli: &Cli) -> Result<ExitCode> {
    let config = match resolved_config(cli) {
        Ok(config) => config,
        Err(error) => return render_config_load_error(&error, cli.json),
    };

    match config.validate() {
        Ok(()) => {
            let policies = config.policies().entries();
            let enabled_policies = policies.iter().filter(|policy| policy.enabled()).count();
            if cli.json {
                let value = serde_json::json!({
                    "status": "valid",
                    "policies": policies.len(),
                    "enabled_policies": enabled_policies,
                    "errors": []
                });
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else if !cli.quiet {
                println!("kply policy check");
                println!("Policy config is valid.");
                println!("Policies: {}", policies.len());
                println!("Enabled policies: {enabled_policies}");
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
fn render_kubeconfig_error(error: &KubeconfigError, wants_json: bool) -> Result<ExitCode> {
    let error = kply_k8s::DiscoveryError::from_kubeconfig_error_redacted(error);
    if wants_json {
        let value = serde_json::json!({
            "error": {
                "code": error.code.as_str(),
                "exit_code": EXIT_USAGE,
                "message": error.message
            }
        });
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        eprintln!("kply error: {}\n\n{}", error.code.as_str(), error.message);
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

#[cfg(test)]
mod tests {
    use super::{
        CheckRunItem, CheckRunReport, CheckRunStatusCounts, InitDiscoveredApp,
        ReportShowUnavailable, SessionCreateApplyError, SessionPlanBuildError,
        SessionStateRecordError, apply_session_resources, check_run_report_from_session,
        compact_duration_seconds, deduplicate_app_names, discover_namespace_apps,
        evaluate_session_planning_policies, image_registry_host, init_config_from_apps,
        init_report_text, planned_resource_token, planned_session_annotations,
        planned_session_checks, planned_session_cleanup_steps, planned_session_labels,
        planned_session_resources, planned_session_risk_notes, policy_route_strategy,
        render_check_evidence, render_check_run_json_report, render_check_run_text_report,
        render_init_config_yaml, render_report_unavailable_json,
        render_report_unavailable_markdown, render_report_unavailable_text,
        report_show_unavailable_from_session, required_session_permissions,
        resolve_session_route_strategy, route_cleanup_from_session,
        route_strategy_creates_route_object, route_strategy_has_route_check,
        route_strategy_uses_preview_service, selector_matches_labels, session_plan_from_config,
        session_plan_from_config_with_policies, session_policy_for_mutation_mode,
        session_state_annotations, session_state_check_status, session_token,
        unsupported_session_feature_warnings, workload_permission_resource,
    };
    use kply_config::{
        AppConfig, DatabaseRiskWarningPolicy, MutationModePolicy, PolicyConfig, PolicyConfigs,
        RouteStrategy,
    };
    use kply_core::{
        CheckResultStatus, ImageRef, SessionOperation, SessionPlan, SessionStatus, TimeToLive,
        WorkloadRef,
    };
    use kply_k8s::{
        DeploymentRolloutPhase, DeploymentRolloutSummary, DeploymentSummary, LabelSelectorEntry,
        MutationError, MutationErrorCode, ServicePortSummary, ServiceSummary, SessionSummary,
    };
    use std::collections::BTreeMap;

    #[test]
    fn matches_service_selector_against_deployment_template_labels() {
        let selector = vec![label("app", "checkout"), label("tier", "api")];
        let labels = vec![
            label("app", "checkout"),
            label("track", "stable"),
            label("tier", "api"),
        ];

        assert!(selector_matches_labels(&selector, &labels));
        assert!(!selector_matches_labels(
            &[label("app", "checkout"), label("tier", "worker")],
            &labels
        ));
    }

    #[test]
    fn discovers_namespace_apps_from_deterministic_service_selectors() {
        let deployments = vec![
            deployment_summary("shop", "checkout-api", &[label("app", "checkout")]),
            deployment_summary("shop", "catalog-api", &[label("app", "catalog")]),
        ];
        let services = vec![
            service_summary("shop", "checkout-http", &[label("app", "checkout")], &[80]),
            service_summary("shop", "catalog-http", &[label("app", "catalog")], &[8080]),
        ];

        let discovery = discover_namespace_apps("shop", &deployments, &services);

        assert_eq!(
            discovery.apps,
            vec![
                InitDiscoveredApp {
                    name: "checkout-api".to_owned(),
                    namespace: "shop".to_owned(),
                    workload: "checkout-api".to_owned(),
                    workload_kind: "Deployment".to_owned(),
                    service: "checkout-http".to_owned(),
                    route_strategy: "preview".to_owned(),
                    ports: vec![80],
                },
                InitDiscoveredApp {
                    name: "catalog-api".to_owned(),
                    namespace: "shop".to_owned(),
                    workload: "catalog-api".to_owned(),
                    workload_kind: "Deployment".to_owned(),
                    service: "catalog-http".to_owned(),
                    route_strategy: "preview".to_owned(),
                    ports: vec![8080],
                },
            ]
        );
        assert!(discovery.skipped_services.is_empty());
    }

    #[test]
    fn skips_ambiguous_and_selectorless_services() {
        let deployments = vec![
            deployment_summary("shop", "checkout-blue", &[label("app", "checkout")]),
            deployment_summary("shop", "checkout-green", &[label("app", "checkout")]),
        ];
        let services = vec![
            service_summary("shop", "checkout-http", &[label("app", "checkout")], &[80]),
            service_summary("shop", "headless", &[], &[80]),
        ];

        let discovery = discover_namespace_apps("shop", &deployments, &services);

        assert!(discovery.apps.is_empty());
        assert_eq!(discovery.skipped_services.len(), 2);
        assert_eq!(discovery.skipped_services[0].reason, "ambiguous_selector");
        assert_eq!(discovery.skipped_services[1].reason, "missing_selector");
    }

    #[test]
    fn deduplicates_app_names_across_namespaces() {
        let mut apps = vec![
            discovered_app("web", "staging", "web", "web-http"),
            discovered_app("web", "prod", "web", "web-http"),
            discovered_app("api", "prod", "api", "api-http"),
        ];

        deduplicate_app_names(&mut apps);

        assert_eq!(apps[0].name, "staging-web");
        assert_eq!(apps[1].name, "prod-web");
        assert_eq!(apps[2].name, "api");
    }

    #[test]
    fn renders_init_config_yaml_from_discovered_apps() {
        let apps = vec![discovered_app(
            "checkout-api",
            "shop",
            "checkout-api",
            "checkout-http",
        )];
        let config = init_config_from_apps(&apps);
        config.validate().expect("generated config should validate");
        let yaml = render_init_config_yaml(&apps).expect("config should serialize");

        insta::assert_snapshot!("init_config_yaml", yaml);
    }

    #[test]
    fn renders_empty_init_report_text() {
        let report = init_report(Vec::new(), Vec::new());

        insta::assert_snapshot!("init_report_empty_text", init_report_text(&report, false));
    }

    #[test]
    fn renders_multiple_app_init_report_text() {
        let report = init_report(
            vec![
                discovered_app("checkout-api", "shop", "checkout-api", "checkout-http"),
                discovered_app("catalog-api", "shop", "catalog-api", "catalog-http"),
            ],
            Vec::new(),
        );

        insta::assert_snapshot!(
            "init_report_multiple_apps_text",
            init_report_text(&report, false)
        );
    }

    #[test]
    fn renders_init_report_json_without_terminal_decoration() {
        let report = init_report(
            vec![discovered_app(
                "checkout-api",
                "shop",
                "checkout-api",
                "checkout-http",
            )],
            Vec::new(),
        );
        let value = serde_json::to_value(&report).expect("report should serialize");

        insta::assert_json_snapshot!("init_report_json", value);
    }

    #[test]
    fn renders_ambiguous_selector_init_report_text() {
        let report = init_report(
            Vec::new(),
            vec![super::InitSkippedService {
                namespace: "shop".to_owned(),
                service: "checkout-http".to_owned(),
                reason: "ambiguous_selector".to_owned(),
                matched_workloads: vec!["checkout-blue".to_owned(), "checkout-green".to_owned()],
            }],
        );

        insta::assert_snapshot!(
            "init_report_ambiguous_selector_text",
            init_report_text(&report, false)
        );
    }

    #[test]
    fn preserves_session_token_suffix_for_long_app_names() {
        let app_name = "checkout-api-with-a-very-long-stable-application-name-for-tests";

        let plan_token = session_token(app_name, "plan");
        let session_token = session_token(app_name, "session");

        assert!(plan_token.ends_with("-plan"));
        assert!(session_token.ends_with("-session"));
        assert_ne!(plan_token, session_token);
        assert!(plan_token.len() <= 63);
        assert!(session_token.len() <= 63);
    }

    #[test]
    fn session_tokens_preserve_long_app_uniqueness() {
        let shared_prefix = "a".repeat(58);
        let first_app = format!("{shared_prefix}1111");
        let second_app = format!("{shared_prefix}2222");

        let first_token = session_token(&first_app, "plan");
        let second_token = session_token(&second_app, "plan");

        assert_ne!(first_token, second_token);
        assert!(first_token.ends_with("-plan"));
        assert!(second_token.ends_with("-plan"));
        assert!(first_token.len() <= 63);
        assert!(second_token.len() <= 63);
    }

    #[test]
    fn classifies_active_session_state_check_as_passed() {
        assert_eq!(
            session_state_check_status(Some("active")),
            CheckResultStatus::Passed
        );
    }

    #[test]
    fn classifies_missing_session_state_check_as_warning() {
        assert_eq!(session_state_check_status(None), CheckResultStatus::Warning);
    }

    #[test]
    fn classifies_non_active_session_state_check_as_failed() {
        assert_eq!(
            session_state_check_status(Some("preparing")),
            CheckResultStatus::Failed
        );
    }

    #[test]
    fn builds_check_run_report_from_session_metadata() {
        let report = check_run_report_from_session(&SessionSummary {
            id: "checkout-plan".to_owned(),
            name: Some("checkout-session".to_owned()),
            namespace: "shop".to_owned(),
            app: Some("checkout".to_owned()),
            status: Some("active".to_owned()),
            workload_kind: "Deployment".to_owned(),
            workload_name: "checkout-plan-workload".to_owned(),
        });

        assert_eq!(report.session_id, "checkout-plan");
        assert_eq!(report.namespace, "shop");
        assert_eq!(report.status, CheckResultStatus::Passed);
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].name, "session_state");
        assert_eq!(
            report.checks[0].target,
            "shop/Deployment/checkout-plan-workload"
        );
        assert_eq!(report.checks[0].status, CheckResultStatus::Passed);
    }

    #[test]
    fn builds_report_show_unavailable_from_session_metadata() {
        let report = report_show_unavailable_from_session(&SessionSummary {
            id: "checkout-plan".to_owned(),
            name: Some("checkout-session".to_owned()),
            namespace: "shop".to_owned(),
            app: Some("checkout".to_owned()),
            status: Some("active".to_owned()),
            workload_kind: "Deployment".to_owned(),
            workload_name: "checkout-plan-workload".to_owned(),
        });

        assert_eq!(report.session_id, "checkout-plan");
        assert_eq!(report.namespace, "shop");
        assert_eq!(report.session_status, "active");
        assert_eq!(report.report, "not_available");
        assert_eq!(report.reason, "session_report_persistence_not_implemented");
    }

    #[test]
    fn builds_report_show_unavailable_with_unknown_status_when_missing() {
        let report = report_show_unavailable_from_session(&SessionSummary {
            id: "checkout-plan-unknown".to_owned(),
            name: None,
            namespace: "shop".to_owned(),
            app: None,
            status: None,
            workload_kind: "Deployment".to_owned(),
            workload_name: "checkout-plan-workload".to_owned(),
        });

        assert_eq!(report.session_id, "checkout-plan-unknown");
        assert_eq!(report.namespace, "shop");
        assert_eq!(report.session_status, "unknown");
        assert_eq!(report.report, "not_available");
        assert_eq!(report.reason, "session_report_persistence_not_implemented");
    }

    #[test]
    fn renders_report_unavailable_markdown() {
        let report = ReportShowUnavailable {
            session_id: "checkout-plan".to_owned(),
            namespace: "shop".to_owned(),
            session_status: "active".to_owned(),
            report: "not_available",
            reason: "session_report_persistence_not_implemented",
        };

        insta::assert_snapshot!(
            "report_unavailable_markdown",
            render_report_unavailable_markdown(&report)
        );
    }

    #[test]
    fn renders_report_unavailable_text() {
        let report = ReportShowUnavailable {
            session_id: "checkout-plan".to_owned(),
            namespace: "shop".to_owned(),
            session_status: "active".to_owned(),
            report: "not_available",
            reason: "session_report_persistence_not_implemented",
        };

        insta::assert_snapshot!(
            "report_unavailable_text",
            render_report_unavailable_text(&report)
        );
    }

    #[test]
    fn renders_report_unavailable_json() {
        let report = ReportShowUnavailable {
            session_id: "checkout-plan".to_owned(),
            namespace: "shop".to_owned(),
            session_status: "active".to_owned(),
            report: "not_available",
            reason: "session_report_persistence_not_implemented",
        };
        let output = render_report_unavailable_json(&report).expect("report should serialize");
        let value: serde_json::Value = serde_json::from_str(&output).expect("report JSON");

        insta::assert_json_snapshot!("report_unavailable_json", value);
    }

    #[test]
    fn renders_check_run_text_report_with_summary_and_evidence() {
        let report = CheckRunReport {
            session_id: "checkout-plan".to_owned(),
            namespace: "shop".to_owned(),
            status: CheckResultStatus::Failed,
            checks: vec![
                CheckRunItem {
                    name: "session_state",
                    target: "shop/Deployment/checkout-plan-workload".to_owned(),
                    status: CheckResultStatus::Passed,
                    evidence: serde_json::json!({
                        "observed_status": "active",
                        "expected_status": "active",
                    }),
                },
                CheckRunItem {
                    name: "smoke_http",
                    target: "http://checkout-plan.shop.svc.cluster.local/healthz".to_owned(),
                    status: CheckResultStatus::Failed,
                    evidence: serde_json::json!({
                        "status_code": 503,
                        "expected_status_code": 200,
                    }),
                },
            ],
        };

        insta::assert_snapshot!(
            "check_run_text_report",
            render_check_run_text_report(&report)
        );
    }

    #[test]
    fn renders_check_run_json_report_with_summary_and_evidence() {
        let report = CheckRunReport {
            session_id: "checkout-plan".to_owned(),
            namespace: "shop".to_owned(),
            status: CheckResultStatus::Failed,
            checks: vec![
                CheckRunItem {
                    name: "session_state",
                    target: "shop/Deployment/checkout-plan-workload".to_owned(),
                    status: CheckResultStatus::Passed,
                    evidence: serde_json::json!({
                        "observed_status": "active",
                        "expected_status": "active",
                    }),
                },
                CheckRunItem {
                    name: "smoke_http",
                    target: "http://checkout-plan.shop.svc.cluster.local/healthz".to_owned(),
                    status: CheckResultStatus::Failed,
                    evidence: serde_json::json!({
                        "status_code": 503,
                        "expected_status_code": 200,
                    }),
                },
            ],
        };
        let output = render_check_run_json_report(&report).expect("report should serialize");
        let value: serde_json::Value = serde_json::from_str(&output).expect("report JSON");

        insta::assert_json_snapshot!("check_run_json_report", value);
    }

    #[test]
    fn renders_check_run_text_report_without_empty_evidence() {
        let report = CheckRunReport {
            session_id: "checkout-plan".to_owned(),
            namespace: "shop".to_owned(),
            status: CheckResultStatus::Warning,
            checks: vec![CheckRunItem {
                name: "session_state",
                target: "shop/Deployment/checkout-plan-workload".to_owned(),
                status: CheckResultStatus::Warning,
                evidence: serde_json::json!({}),
            }],
        };

        insta::assert_snapshot!(
            "check_run_text_report_without_empty_evidence",
            render_check_run_text_report(&report)
        );
    }

    #[test]
    fn counts_check_result_statuses_for_text_reports() {
        let checks = [
            CheckRunItem {
                name: "passed",
                target: "target".to_owned(),
                status: CheckResultStatus::Passed,
                evidence: serde_json::json!({}),
            },
            CheckRunItem {
                name: "failed",
                target: "target".to_owned(),
                status: CheckResultStatus::Failed,
                evidence: serde_json::json!({}),
            },
            CheckRunItem {
                name: "warning",
                target: "target".to_owned(),
                status: CheckResultStatus::Warning,
                evidence: serde_json::json!({}),
            },
            CheckRunItem {
                name: "skipped",
                target: "target".to_owned(),
                status: CheckResultStatus::Skipped,
                evidence: serde_json::json!({}),
            },
        ];

        assert_eq!(
            CheckRunStatusCounts::from_checks(&checks),
            CheckRunStatusCounts {
                passed: 1,
                failed: 1,
                warning: 1,
                skipped: 1,
            }
        );
    }

    #[test]
    fn renders_scalar_and_nested_check_evidence() {
        let evidence = serde_json::json!({
            "attempts": 2,
            "healthy": false,
            "messages": ["ready", "degraded"],
        });

        assert_eq!(
            render_check_evidence(&evidence).as_deref(),
            Some("attempts=2 healthy=false messages=[\"ready\",\"degraded\"]")
        );
        assert_eq!(render_check_evidence(&serde_json::Value::Null), None);
    }

    #[test]
    fn normalizes_session_token_prefixes() {
        assert_eq!(session_token("", "plan"), "session-plan");
        assert_eq!(session_token("---", "plan"), "session-plan");
        assert_eq!(session_token("MyApp", "plan"), "myapp-plan");
        assert_eq!(session_token("my__app", "plan"), "my-app-plan");
    }

    #[test]
    fn applies_session_deployment_and_service_through_injected_boundaries() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("ghcr.io/acme/checkout:next".to_owned()),
            RouteStrategy::Header,
        );
        let plan = session_plan_from_config(&app, None, None, None, None)
            .unwrap_or_else(|_| panic!("session plan should be created"));
        let mut recorded_statuses = Vec::new();

        let applied = apply_session_resources(
            &plan,
            |namespace, deployment| {
                assert_eq!(namespace, "shop");
                assert_eq!(
                    deployment.metadata.name.as_deref(),
                    Some("checkout-plan-workload")
                );
                assert_eq!(deployment.metadata.namespace.as_deref(), Some("shop"));

                Ok(DeploymentSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-workload".to_owned(),
                    replicas: Some(1),
                    available_replicas: None,
                    ready_replicas: None,
                    updated_replicas: None,
                    images: vec!["ghcr.io/acme/checkout:next".to_owned()],
                    pod_template_labels: Vec::new(),
                    probes: Vec::new(),
                    resources: Vec::new(),
                    rollout: DeploymentRolloutSummary {
                        phase: DeploymentRolloutPhase::Unknown,
                        generation: None,
                        observed_generation: None,
                        desired_replicas: Some(1),
                        ready_replicas: None,
                        available_replicas: None,
                        updated_replicas: None,
                        unavailable_replicas: None,
                        conditions: Vec::new(),
                    },
                })
            },
            |namespace, service| {
                assert_eq!(namespace, "shop");
                assert_eq!(
                    service.metadata.name.as_deref(),
                    Some("checkout-plan-service")
                );
                assert_eq!(service.metadata.namespace.as_deref(), Some("shop"));

                Ok(ServiceSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-service".to_owned(),
                    service_type: Some("ClusterIP".to_owned()),
                    selector: Vec::new(),
                    ports: Vec::new(),
                })
            },
            |namespace, name| {
                assert_eq!(namespace, "shop");
                assert_eq!(name, "checkout-plan-workload");

                Ok(DeploymentSummary {
                    namespace: namespace.to_owned(),
                    name: name.to_owned(),
                    replicas: Some(1),
                    available_replicas: Some(1),
                    ready_replicas: Some(1),
                    updated_replicas: Some(1),
                    images: vec!["ghcr.io/acme/checkout:next".to_owned()],
                    pod_template_labels: Vec::new(),
                    probes: Vec::new(),
                    resources: Vec::new(),
                    rollout: DeploymentRolloutSummary {
                        phase: DeploymentRolloutPhase::Complete,
                        generation: Some(1),
                        observed_generation: Some(1),
                        desired_replicas: Some(1),
                        ready_replicas: Some(1),
                        available_replicas: Some(1),
                        updated_replicas: Some(1),
                        unavailable_replicas: None,
                        conditions: Vec::new(),
                    },
                })
            },
            |resources, status| {
                recorded_statuses.push(status);
                assert_eq!(resources.len(), 2);
                assert_eq!(resources[0].kind, "Deployment");
                assert_eq!(resources[1].kind, "Service");

                Ok(resources)
            },
        )
        .expect("session resource apply should succeed");

        assert_eq!(applied.created_resources.len(), 2);
        assert_eq!(applied.created_resources[0].kind, "Deployment");
        assert_eq!(applied.created_resources[0].namespace, "shop");
        assert_eq!(applied.created_resources[0].name, "checkout-plan-workload");
        assert_eq!(applied.created_resources[1].kind, "Service");
        assert_eq!(applied.created_resources[1].namespace, "shop");
        assert_eq!(applied.created_resources[1].name, "checkout-plan-service");
        assert_eq!(applied.pending_resources.len(), 1);
        assert!(
            applied
                .pending_resources
                .iter()
                .any(|resource| resource.kind == "ConfigMap")
        );
        assert_eq!(applied.readiness.resource.kind, "Deployment");
        assert_eq!(applied.readiness.resource.name, "checkout-plan-workload");
        assert_eq!(applied.readiness.phase, DeploymentRolloutPhase::Complete);
        assert_eq!(applied.state.status, SessionStatus::Active);
        assert_eq!(applied.state.resources.len(), 2);
        assert_eq!(
            recorded_statuses,
            [SessionStatus::Preparing, SessionStatus::Active]
        );
    }

    #[test]
    fn rejects_session_resource_apply_when_policy_forbids_prepare() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("ghcr.io/acme/checkout:next".to_owned()),
            RouteStrategy::Header,
        );
        let policies = PolicyConfigs::new(vec![
            PolicyConfig::new("read-only")
                .with_allowed_namespaces(["shop"])
                .with_allowed_workload_kinds(["Deployment"])
                .with_allowed_image_registries(["ghcr.io"])
                .with_mutation_mode(MutationModePolicy::ReadOnly),
        ]);
        let plan =
            session_plan_from_config_with_policies(&app, &policies, None, None, None, Some("none"))
                .unwrap_or_else(|_| panic!("read-only session plan should be created"));

        let error = apply_session_resources(
            &plan,
            |_, _| panic!("deployment creation must not run for read-only policies"),
            |_, _| panic!("service creation must not run for read-only policies"),
            |_, _| panic!("readiness wait must not run for read-only policies"),
            |_, _| panic!("state recording must not run for read-only policies"),
        )
        .expect_err("read-only policy should block resource apply");

        match error {
            SessionCreateApplyError::Policy(message) => {
                assert_eq!(message, "policy does not allow session creation");
            }
            _ => panic!("expected policy apply denial"),
        }
    }

    #[test]
    fn reports_partial_apply_when_service_creation_fails() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("ghcr.io/acme/checkout:next".to_owned()),
            RouteStrategy::Header,
        );
        let plan = session_plan_from_config(&app, None, None, None, None)
            .unwrap_or_else(|_| panic!("session plan should be created"));

        let error = apply_session_resources(
            &plan,
            |namespace, _deployment| {
                Ok(DeploymentSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-workload".to_owned(),
                    replicas: Some(1),
                    available_replicas: None,
                    ready_replicas: None,
                    updated_replicas: None,
                    images: Vec::new(),
                    pod_template_labels: Vec::new(),
                    probes: Vec::new(),
                    resources: Vec::new(),
                    rollout: DeploymentRolloutSummary {
                        phase: DeploymentRolloutPhase::Unknown,
                        generation: None,
                        observed_generation: None,
                        desired_replicas: Some(1),
                        ready_replicas: None,
                        available_replicas: None,
                        updated_replicas: None,
                        unavailable_replicas: None,
                        conditions: Vec::new(),
                    },
                })
            },
            |_namespace, _service| {
                Err(MutationError {
                    code: MutationErrorCode::KubernetesApi,
                    message: "create sandbox Service failed".to_owned(),
                })
            },
            |_namespace, _name| panic!("readiness must not run after Service creation fails"),
            |_resources, _status| panic!("state must not record after Service creation fails"),
        )
        .expect_err("service failure after Deployment create should be partial");

        let SessionCreateApplyError::PartialMutation {
            created_resources,
            pending_resources,
            ..
        } = error
        else {
            panic!("expected a partial mutation error");
        };

        assert_eq!(created_resources.len(), 1);
        assert_eq!(created_resources[0].kind, "Deployment");
        assert_eq!(created_resources[0].name, "checkout-plan-workload");
        assert_eq!(pending_resources.len(), 2);
        assert!(
            pending_resources
                .iter()
                .any(|resource| resource.kind == "Service")
        );
        assert!(
            pending_resources
                .iter()
                .any(|resource| resource.kind == "ConfigMap")
        );
    }

    #[test]
    fn reports_partial_apply_when_readiness_wait_fails() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("ghcr.io/acme/checkout:next".to_owned()),
            RouteStrategy::Header,
        );
        let plan = session_plan_from_config(&app, None, None, None, None)
            .unwrap_or_else(|_| panic!("session plan should be created"));

        let error = apply_session_resources(
            &plan,
            |namespace, _deployment| {
                Ok(DeploymentSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-workload".to_owned(),
                    replicas: Some(1),
                    available_replicas: None,
                    ready_replicas: None,
                    updated_replicas: None,
                    images: Vec::new(),
                    pod_template_labels: Vec::new(),
                    probes: Vec::new(),
                    resources: Vec::new(),
                    rollout: DeploymentRolloutSummary {
                        phase: DeploymentRolloutPhase::Unknown,
                        generation: None,
                        observed_generation: None,
                        desired_replicas: Some(1),
                        ready_replicas: None,
                        available_replicas: None,
                        updated_replicas: None,
                        unavailable_replicas: None,
                        conditions: Vec::new(),
                    },
                })
            },
            |namespace, _service| {
                Ok(ServiceSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-service".to_owned(),
                    service_type: Some("ClusterIP".to_owned()),
                    selector: Vec::new(),
                    ports: Vec::new(),
                })
            },
            |_namespace, _name| {
                Err(MutationError {
                    code: MutationErrorCode::KubernetesApi,
                    message: "sandbox Deployment did not become ready".to_owned(),
                })
            },
            |resources, status| {
                assert_eq!(status, SessionStatus::Preparing);
                Ok(resources)
            },
        )
        .expect_err("readiness failure after resources create should be partial");

        let SessionCreateApplyError::PartialMutation {
            created_resources,
            pending_resources,
            recorded_resources,
            ..
        } = error
        else {
            panic!("expected a partial mutation error");
        };

        assert_eq!(created_resources.len(), 2);
        assert!(
            created_resources
                .iter()
                .any(|resource| resource.kind == "Deployment")
        );
        assert!(
            created_resources
                .iter()
                .any(|resource| resource.kind == "Service")
        );
        assert_eq!(pending_resources.len(), 1);
        assert_eq!(pending_resources[0].kind, "ConfigMap");
        assert_eq!(recorded_resources.len(), 2);
        assert!(
            recorded_resources
                .iter()
                .any(|resource| resource.kind == "Deployment")
        );
        assert!(
            recorded_resources
                .iter()
                .any(|resource| resource.kind == "Service")
        );
    }

    #[test]
    fn reports_partial_apply_when_state_recording_fails() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("ghcr.io/acme/checkout:next".to_owned()),
            RouteStrategy::Header,
        );
        let plan = session_plan_from_config(&app, None, None, None, None)
            .unwrap_or_else(|_| panic!("session plan should be created"));

        let error = apply_session_resources(
            &plan,
            |namespace, _deployment| {
                Ok(DeploymentSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-workload".to_owned(),
                    replicas: Some(1),
                    available_replicas: None,
                    ready_replicas: None,
                    updated_replicas: None,
                    images: Vec::new(),
                    pod_template_labels: Vec::new(),
                    probes: Vec::new(),
                    resources: Vec::new(),
                    rollout: DeploymentRolloutSummary {
                        phase: DeploymentRolloutPhase::Unknown,
                        generation: None,
                        observed_generation: None,
                        desired_replicas: Some(1),
                        ready_replicas: None,
                        available_replicas: None,
                        updated_replicas: None,
                        unavailable_replicas: None,
                        conditions: Vec::new(),
                    },
                })
            },
            |namespace, _service| {
                Ok(ServiceSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-service".to_owned(),
                    service_type: Some("ClusterIP".to_owned()),
                    selector: Vec::new(),
                    ports: Vec::new(),
                })
            },
            |namespace, name| {
                Ok(DeploymentSummary {
                    namespace: namespace.to_owned(),
                    name: name.to_owned(),
                    replicas: Some(1),
                    available_replicas: Some(1),
                    ready_replicas: Some(1),
                    updated_replicas: Some(1),
                    images: Vec::new(),
                    pod_template_labels: Vec::new(),
                    probes: Vec::new(),
                    resources: Vec::new(),
                    rollout: DeploymentRolloutSummary {
                        phase: DeploymentRolloutPhase::Complete,
                        generation: Some(1),
                        observed_generation: Some(1),
                        desired_replicas: Some(1),
                        ready_replicas: Some(1),
                        available_replicas: Some(1),
                        updated_replicas: Some(1),
                        unavailable_replicas: None,
                        conditions: Vec::new(),
                    },
                })
            },
            |resources, status| {
                if status == SessionStatus::Preparing {
                    Ok(resources)
                } else {
                    assert_eq!(status, SessionStatus::Active);
                    Err(SessionStateRecordError {
                        error: MutationError {
                            code: MutationErrorCode::KubernetesApi,
                            message: "record session state failed".to_owned(),
                        },
                        recorded_resources: Vec::new(),
                    })
                }
            },
        )
        .expect_err("state recording failure after resources create should be partial");

        let SessionCreateApplyError::PartialMutation {
            created_resources,
            pending_resources,
            recorded_resources,
            ..
        } = error
        else {
            panic!("expected a partial mutation error");
        };

        assert_eq!(created_resources.len(), 2);
        assert!(
            created_resources
                .iter()
                .any(|resource| resource.kind == "Deployment")
        );
        assert!(
            created_resources
                .iter()
                .any(|resource| resource.kind == "Service")
        );
        assert_eq!(pending_resources.len(), 1);
        assert_eq!(pending_resources[0].kind, "ConfigMap");
        assert_eq!(recorded_resources.len(), 2);
    }

    #[test]
    fn reports_recorded_resources_when_state_recording_partially_fails() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("ghcr.io/acme/checkout:next".to_owned()),
            RouteStrategy::Header,
        );
        let plan = session_plan_from_config(&app, None, None, None, None)
            .unwrap_or_else(|_| panic!("session plan should be created"));

        let error = apply_session_resources(
            &plan,
            |namespace, _deployment| {
                Ok(DeploymentSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-workload".to_owned(),
                    replicas: Some(1),
                    available_replicas: None,
                    ready_replicas: None,
                    updated_replicas: None,
                    images: Vec::new(),
                    pod_template_labels: Vec::new(),
                    probes: Vec::new(),
                    resources: Vec::new(),
                    rollout: DeploymentRolloutSummary {
                        phase: DeploymentRolloutPhase::Unknown,
                        generation: None,
                        observed_generation: None,
                        desired_replicas: Some(1),
                        ready_replicas: None,
                        available_replicas: None,
                        updated_replicas: None,
                        unavailable_replicas: None,
                        conditions: Vec::new(),
                    },
                })
            },
            |namespace, _service| {
                Ok(ServiceSummary {
                    namespace: namespace.to_owned(),
                    name: "checkout-plan-service".to_owned(),
                    service_type: Some("ClusterIP".to_owned()),
                    selector: Vec::new(),
                    ports: Vec::new(),
                })
            },
            |namespace, name| {
                Ok(DeploymentSummary {
                    namespace: namespace.to_owned(),
                    name: name.to_owned(),
                    replicas: Some(1),
                    available_replicas: Some(1),
                    ready_replicas: Some(1),
                    updated_replicas: Some(1),
                    images: Vec::new(),
                    pod_template_labels: Vec::new(),
                    probes: Vec::new(),
                    resources: Vec::new(),
                    rollout: DeploymentRolloutSummary {
                        phase: DeploymentRolloutPhase::Complete,
                        generation: Some(1),
                        observed_generation: Some(1),
                        desired_replicas: Some(1),
                        ready_replicas: Some(1),
                        available_replicas: Some(1),
                        updated_replicas: Some(1),
                        unavailable_replicas: None,
                        conditions: Vec::new(),
                    },
                })
            },
            |resources, status| {
                if status == SessionStatus::Preparing {
                    Ok(resources)
                } else {
                    Err(SessionStateRecordError {
                        error: MutationError {
                            code: MutationErrorCode::KubernetesApi,
                            message: "record Service state failed".to_owned(),
                        },
                        recorded_resources: vec![resources[0].clone()],
                    })
                }
            },
        )
        .expect_err("partial state recording should be auditable");

        let SessionCreateApplyError::PartialMutation {
            recorded_resources, ..
        } = error
        else {
            panic!("expected a partial mutation error");
        };

        assert_eq!(recorded_resources.len(), 1);
        assert_eq!(recorded_resources[0].kind, "Deployment");
        assert_eq!(recorded_resources[0].name, "checkout-plan-workload");
    }

    #[test]
    fn builds_session_state_annotations() {
        let annotations = session_state_annotations(SessionStatus::Active);

        assert_eq!(
            annotations
                .get("kply.dev/session-status")
                .map(String::as_str),
            Some("active")
        );
    }

    #[test]
    fn builds_planned_session_resources() {
        let resources = planned_session_resources("ns", "Workload", "sess", "header")
            .expect("planned resources");

        assert_eq!(resources.len(), 3);
        assert_eq!(resources[0].namespace(), "ns");
        assert_eq!(resources[0].kind(), "Workload");
        assert_eq!(
            resources[0].name(),
            &planned_resource_token("sess", "workload")
        );
        assert_eq!(resources[1].namespace(), "ns");
        assert_eq!(resources[1].kind(), "Service");
        assert_eq!(
            resources[1].name(),
            &planned_resource_token("sess", "service")
        );
        assert_eq!(resources[2].namespace(), "ns");
        assert_eq!(resources[2].kind(), "HTTPRoute");
        assert_eq!(
            resources[2].name(),
            &planned_resource_token("sess", "route")
        );

        let preview_resources = planned_session_resources("ns", "Workload", "sess", "preview")
            .expect("preview planned resources");
        assert_eq!(preview_resources.len(), 2);
        assert_eq!(preview_resources[0].kind(), "Workload");
        assert_eq!(preview_resources[1].kind(), "Service");

        let preview_service_resources =
            planned_session_resources("ns", "Workload", "sess", "preview-service")
                .expect("preview service resources");
        assert_eq!(preview_service_resources.len(), 2);
        assert_eq!(preview_service_resources[0].kind(), "Workload");
        assert_eq!(preview_service_resources[1].kind(), "Service");

        let none_resources =
            planned_session_resources("ns", "Workload", "sess", "none").expect("none resources");
        assert_eq!(none_resources.len(), 2);
        assert_eq!(none_resources[0].kind(), "Workload");
        assert_eq!(none_resources[1].kind(), "Service");
    }

    #[test]
    fn planned_session_resources_return_validation_errors() {
        let error = planned_session_resources("Bad_Namespace", "Workload", "sess", "header")
            .expect_err("invalid namespace should fail");

        assert!(error.contains("namespace"));
    }

    #[test]
    fn builds_planned_session_labels() {
        let labels =
            planned_session_labels("myapp", "session-123", "my-session").expect("planned labels");

        assert_eq!(labels.len(), 4);
        assert_eq!(labels[0].key(), "kply.dev/app");
        assert_eq!(labels[0].value(), "myapp");
        assert_eq!(labels[1].key(), "kply.dev/managed-by");
        assert_eq!(labels[1].value(), "kply");
        assert_eq!(labels[2].key(), "kply.dev/session-id");
        assert_eq!(labels[2].value(), "session-123");
        assert_eq!(labels[3].key(), "kply.dev/session-name");
        assert_eq!(labels[3].value(), "my-session");
    }

    #[test]
    fn planned_session_labels_return_validation_errors() {
        let error = planned_session_labels("my app", "session-123", "my-session")
            .expect_err("invalid label value should fail");

        assert!(error.contains("metadata value"));
    }

    #[test]
    fn builds_planned_session_annotations() {
        let workload = WorkloadRef::new("ns", "Deployment", "name").expect("workload");
        let image = ImageRef::new("myimage:v1").expect("image");
        let annotations =
            planned_session_annotations(&workload, &image, "header").expect("annotations");

        assert_eq!(annotations.len(), 3);
        assert_eq!(annotations[0].key(), "kply.dev/image");
        assert_eq!(annotations[0].value(), "myimage:v1");
        assert_eq!(annotations[1].key(), "kply.dev/route-strategy");
        assert_eq!(annotations[1].value(), "header");
        assert_eq!(annotations[2].key(), "kply.dev/workload");
        assert_eq!(annotations[2].value(), "ns/Deployment/name");
    }

    #[test]
    fn planned_session_annotations_return_validation_errors() {
        let workload = WorkloadRef::new("ns", "Deployment", "name").expect("workload");
        let image = ImageRef::new("myimage:v1").expect("image");
        let error = planned_session_annotations(&workload, &image, "bad strategy")
            .expect_err("invalid annotation value should fail");

        assert!(error.contains("metadata value"));
    }

    #[test]
    fn builds_planned_session_checks() {
        let workload = WorkloadRef::new("ns", "Deployment", "name").expect("workload");
        let image = ImageRef::new("myimage:v1").expect("image");
        let checks =
            planned_session_checks("ns", &workload, &image, "header", "sess").expect("checks");

        assert_eq!(checks.len(), 4);
        assert_eq!(checks[0].name(), "image_pull");
        assert_eq!(checks[0].target(), "myimage:v1");
        assert_eq!(checks[1].name(), "route_ready");
        assert_eq!(checks[1].target(), "header");
        assert_eq!(checks[2].name(), "service_endpoints");
        assert_eq!(
            checks[2].target(),
            &format!("ns/{}", planned_resource_token("sess", "service"))
        );
        assert_eq!(checks[3].name(), "workload_ready");
        assert_eq!(checks[3].target(), "ns/Deployment/name");

        let preview_checks =
            planned_session_checks("ns", &workload, &image, "preview", "sess").expect("checks");
        assert_eq!(
            preview_checks[1].target(),
            &format!("ns/{}", planned_resource_token("sess", "service"))
        );

        let preview_service_checks =
            planned_session_checks("ns", &workload, &image, "preview-service", "sess")
                .expect("checks");
        assert_eq!(
            preview_service_checks[1].target(),
            &format!("ns/{}", planned_resource_token("sess", "service"))
        );

        let none_checks =
            planned_session_checks("ns", &workload, &image, "none", "sess").expect("checks");
        assert_eq!(none_checks.len(), 3);
        assert_eq!(none_checks[0].name(), "image_pull");
        assert_eq!(none_checks[1].name(), "service_endpoints");
        assert_eq!(none_checks[2].name(), "workload_ready");
    }

    #[test]
    fn planned_session_checks_return_validation_errors() {
        let workload = WorkloadRef::new("ns", "Deployment", "name").expect("workload");
        let image = ImageRef::new("myimage:v1").expect("image");
        let error = planned_session_checks("Bad_Namespace", &workload, &image, "header", "sess")
            .expect_err("invalid service ref should fail");

        assert!(error.contains("namespace"));
    }

    #[test]
    fn builds_planned_session_cleanup_steps() {
        let steps = planned_session_cleanup_steps("ns", "Deployment", "sess", "header")
            .expect("planned cleanup steps");

        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].action(), "delete_route");
        assert_eq!(
            steps[0].target(),
            &format!("ns/HTTPRoute/{}", planned_resource_token("sess", "route"))
        );
        assert_eq!(steps[1].action(), "delete_service");
        assert_eq!(
            steps[1].target(),
            &format!("ns/Service/{}", planned_resource_token("sess", "service"))
        );
        assert_eq!(steps[2].action(), "delete_workload");
        assert_eq!(
            steps[2].target(),
            &format!(
                "ns/Deployment/{}",
                planned_resource_token("sess", "workload")
            )
        );

        let preview_steps = planned_session_cleanup_steps("ns", "Deployment", "sess", "preview")
            .expect("preview cleanup steps");
        assert_eq!(preview_steps.len(), 2);
        assert_eq!(preview_steps[0].action(), "delete_service");
        assert_eq!(preview_steps[1].action(), "delete_workload");

        let preview_service_steps =
            planned_session_cleanup_steps("ns", "Deployment", "sess", "preview-service")
                .expect("preview service cleanup steps");
        assert_eq!(preview_service_steps.len(), 2);
        assert_eq!(preview_service_steps[0].action(), "delete_service");
        assert_eq!(preview_service_steps[1].action(), "delete_workload");

        let none_steps = planned_session_cleanup_steps("ns", "Deployment", "sess", "none")
            .expect("none cleanup steps");
        assert_eq!(none_steps.len(), 2);
        assert_eq!(none_steps[0].action(), "delete_service");
        assert_eq!(none_steps[1].action(), "delete_workload");
    }

    #[test]
    fn builds_route_cleanup_for_session_route_object() {
        let cleanup = route_cleanup_from_session("checkout-plan", Some("shop"))
            .expect("route cleanup should be planned");

        assert_eq!(cleanup.session_id, "checkout-plan");
        assert_eq!(cleanup.status, "planned");
        assert_eq!(cleanup.mutation, "not_applied");
        assert!(!cleanup.cleanup);
        assert_eq!(cleanup.route_kind, "HTTPRoute");
        let target = cleanup
            .cleanup_target
            .expect("namespaced cleanup should include a route target");
        assert_eq!(target.api_version, "gateway.networking.k8s.io/v1");
        assert_eq!(target.kind, "HTTPRoute");
        assert_eq!(target.namespace, "shop");
        assert_eq!(target.name, "checkout-plan-route");
        assert_eq!(
            target.selector.match_labels,
            BTreeMap::from([
                ("kply.dev/managed-by".to_owned(), "kply".to_owned()),
                ("kply.dev/session-id".to_owned(), "checkout-plan".to_owned()),
            ])
        );
        assert_eq!(cleanup.cleanup_selector, target.selector);
    }

    #[test]
    fn builds_route_cleanup_selector_without_namespace_target() {
        let cleanup = route_cleanup_from_session("checkout-plan", None)
            .expect("route cleanup should be planned without namespace");

        assert_eq!(cleanup.session_id, "checkout-plan");
        assert_eq!(cleanup.route_kind, "HTTPRoute");
        assert!(cleanup.cleanup_target.is_none());
        assert_eq!(
            cleanup.cleanup_selector.match_labels,
            BTreeMap::from([
                ("kply.dev/managed-by".to_owned(), "kply".to_owned()),
                ("kply.dev/session-id".to_owned(), "checkout-plan".to_owned()),
            ])
        );
    }

    #[test]
    fn planned_session_cleanup_steps_return_validation_errors() {
        let error = planned_session_cleanup_steps("Bad_Namespace", "Deployment", "sess", "header")
            .expect_err("invalid cleanup resource should fail");

        assert!(error.contains("namespace"));
    }

    #[test]
    fn builds_required_session_permissions() {
        let permissions =
            required_session_permissions("Deployment", "header").expect("required permissions");

        assert_eq!(permissions.len(), 4);
        assert_eq!(permissions[0].api_group(), "");
        assert_eq!(permissions[0].resource(), "pods");
        assert_eq!(permissions[0].verbs(), ["get", "list", "watch"]);
        assert_eq!(permissions[1].api_group(), "");
        assert_eq!(permissions[1].resource(), "services");
        assert_eq!(permissions[1].verbs(), ["create", "delete", "get", "patch"]);
        assert_eq!(permissions[2].api_group(), "apps");
        assert_eq!(permissions[2].resource(), "deployments");
        assert_eq!(permissions[2].verbs(), ["create", "delete", "get", "patch"]);
        assert_eq!(permissions[3].api_group(), "gateway.networking.k8s.io");
        assert_eq!(permissions[3].resource(), "httproutes");
        assert_eq!(permissions[3].verbs(), ["create", "delete", "get"]);

        let preview_permissions = required_session_permissions("Deployment", "preview")
            .expect("preview required permissions");
        assert_eq!(preview_permissions.len(), 3);
        assert!(
            preview_permissions
                .iter()
                .all(|permission| permission.resource() != "httproutes")
        );

        let preview_service_permissions =
            required_session_permissions("Deployment", "preview-service")
                .expect("preview service permissions");
        assert_eq!(preview_service_permissions.len(), 3);
        assert!(
            preview_service_permissions
                .iter()
                .all(|permission| permission.resource() != "httproutes")
        );

        let none_permissions =
            required_session_permissions("Deployment", "none").expect("none permissions");
        assert_eq!(none_permissions.len(), 3);
        assert!(
            none_permissions
                .iter()
                .all(|permission| permission.resource() != "httproutes")
        );
    }

    #[test]
    fn required_session_permissions_return_validation_errors() {
        let error = required_session_permissions("Bad_Workload", "header")
            .expect_err("invalid workload resource should fail");

        assert!(error.contains("unsupported workload kind"));
    }

    #[test]
    fn maps_known_workload_kinds_to_permission_resources() {
        assert_eq!(
            workload_permission_resource("DaemonSet").expect("workload resource"),
            "daemonsets"
        );
        assert_eq!(
            workload_permission_resource("Deployment").expect("workload resource"),
            "deployments"
        );
        assert_eq!(
            workload_permission_resource("ReplicaSet").expect("workload resource"),
            "replicasets"
        );
        assert_eq!(
            workload_permission_resource("StatefulSet").expect("workload resource"),
            "statefulsets"
        );
        assert!(
            workload_permission_resource("Widget")
                .expect_err("unknown workload kind should fail")
                .contains("unsupported workload kind")
        );
    }

    #[test]
    fn builds_unsupported_session_feature_warnings() {
        let header_warnings =
            unsupported_session_feature_warnings("header").expect("unsupported warnings");
        let preview_warnings =
            unsupported_session_feature_warnings("preview").expect("unsupported warnings");
        let preview_service_warnings =
            unsupported_session_feature_warnings("preview-service").expect("unsupported warnings");
        let none_warnings =
            unsupported_session_feature_warnings("none").expect("unsupported warnings");

        assert!(header_warnings.is_empty());
        assert_eq!(preview_warnings.len(), 1);
        assert_eq!(preview_warnings[0].feature(), "edge_route_validation");
        assert_eq!(
            preview_warnings[0].reason(),
            "preview_service_skips_edge_route_validation"
        );
        assert_eq!(preview_service_warnings, preview_warnings);
        assert_eq!(none_warnings.len(), 1);
        assert_eq!(none_warnings[0].feature(), "edge_route_validation");
        assert_eq!(
            none_warnings[0].reason(),
            "route_strategy_none_skips_route_validation"
        );
    }

    #[test]
    fn resolves_auto_route_strategy_to_configured_strategy() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("registry.example.com/shop/checkout:test".to_owned()),
            RouteStrategy::Preview,
        );

        assert_eq!(resolve_session_route_strategy(&app, None), "preview");
        assert_eq!(
            resolve_session_route_strategy(&app, Some("auto")),
            "preview"
        );
        assert_eq!(resolve_session_route_strategy(&app, Some("host")), "host");
        assert_eq!(resolve_session_route_strategy(&app, Some("none")), "none");
        assert_eq!(
            resolve_session_route_strategy(&app, Some("preview-service")),
            "preview-service"
        );
        assert!(!route_strategy_creates_route_object("none"));
        assert!(!route_strategy_has_route_check("none"));
        assert!(!route_strategy_creates_route_object("preview-service"));
        assert!(route_strategy_has_route_check("preview-service"));
        assert!(route_strategy_uses_preview_service("preview-service"));
    }

    #[test]
    fn maps_cli_route_strategies_to_policy_route_strategies() {
        assert_eq!(policy_route_strategy("header"), Some(RouteStrategy::Header));
        assert_eq!(policy_route_strategy("host"), Some(RouteStrategy::Host));
        assert_eq!(
            policy_route_strategy("preview"),
            Some(RouteStrategy::Preview)
        );
        assert_eq!(
            policy_route_strategy("preview-service"),
            Some(RouteStrategy::Preview)
        );
        assert_eq!(policy_route_strategy("none"), None);
        assert_eq!(policy_route_strategy("unknown"), None);
    }

    #[test]
    fn extracts_image_registry_hosts_for_policy_checks() {
        assert_eq!(image_registry_host("ghcr.io/acme/checkout:next"), "ghcr.io");
        assert_eq!(
            image_registry_host("localhost:5000/acme/checkout:next"),
            "localhost:5000"
        );
        assert_eq!(
            image_registry_host("localhost/acme/checkout:next"),
            "localhost"
        );
        assert_eq!(image_registry_host("nginx:latest"), "docker.io");
        assert_eq!(image_registry_host("myimage:v1"), "docker.io");
        assert_eq!(image_registry_host("busybox"), "docker.io");
    }

    #[test]
    fn converts_compact_policy_durations_to_seconds() {
        assert_eq!(compact_duration_seconds("30s"), 30);
        assert_eq!(compact_duration_seconds("2m"), 120);
        assert_eq!(compact_duration_seconds("3h"), 10_800);
        assert_eq!(compact_duration_seconds("1d"), 86_400);
    }

    #[test]
    fn derives_session_policy_from_mutation_mode() {
        let read_only = session_policy_for_mutation_mode(Some(MutationModePolicy::ReadOnly))
            .expect("read-only policy should build");
        assert_eq!(read_only.allowed_operations().len(), 2);
        assert!(read_only.allows(SessionOperation::Inspect));
        assert!(read_only.allows(SessionOperation::Plan));
        assert!(!read_only.allows(SessionOperation::Prepare));

        let sandbox_only = session_policy_for_mutation_mode(Some(MutationModePolicy::SandboxOnly))
            .expect("sandbox-only policy should build");
        assert_eq!(sandbox_only.allowed_operations().len(), 5);
        assert!(sandbox_only.allows(SessionOperation::Prepare));
        assert!(!sandbox_only.allows(SessionOperation::Route));

        let route_mutation =
            session_policy_for_mutation_mode(Some(MutationModePolicy::RouteMutation))
                .expect("route-mutation policy should build");
        assert_eq!(route_mutation.allowed_operations().len(), 6);
        assert!(route_mutation.allows(SessionOperation::Route));
    }

    #[test]
    fn evaluates_session_planning_policy_matches_and_denials() {
        let image = ImageRef::new("ghcr.io/acme/checkout:next").expect("image");
        let time_to_live = TimeToLive::new("30m").expect("ttl");
        let policies = PolicyConfigs::new(vec![
            PolicyConfig::new("disabled")
                .with_enabled(false)
                .with_allowed_namespaces(["warehouse"]),
            PolicyConfig::new("sandbox-defaults")
                .with_allowed_namespaces(["shop"])
                .with_allowed_workload_kinds(["Deployment"])
                .with_allowed_image_registries(["ghcr.io"])
                .with_allowed_route_strategies([RouteStrategy::Preview])
                .with_max_session_ttl("30m")
                .with_mutation_mode(MutationModePolicy::SandboxOnly)
                .with_database_risk_warnings(DatabaseRiskWarningPolicy::Disabled),
        ]);

        let decision = evaluate_session_planning_policies(
            &policies,
            "shop",
            "Deployment",
            &image,
            Some(&time_to_live),
            "preview",
        )
        .expect("matching policy should allow planning");

        assert!(!decision.database_risk_warnings_enabled);
        assert_eq!(decision.session_policy.allowed_operations().len(), 5);

        let denied = evaluate_session_planning_policies(
            &policies,
            "warehouse",
            "Deployment",
            &image,
            Some(&time_to_live),
            "preview",
        )
        .expect_err("no enabled policy should allow warehouse");

        match denied {
            SessionPlanBuildError::Policy(message) => {
                assert!(message.contains("does not allow namespace `warehouse`"));
            }
            _ => panic!("expected policy denial"),
        }
    }

    #[test]
    fn evaluates_session_planning_policy_allowed_and_denied_actions() {
        let image = ImageRef::new("ghcr.io/acme/checkout:next").expect("image");
        let time_to_live = TimeToLive::new("30m").expect("ttl");
        let long_time_to_live = TimeToLive::new("45m").expect("long ttl");

        let route_mutation_policies = PolicyConfigs::new(vec![
            PolicyConfig::new("route-mutation")
                .with_allowed_namespaces(["shop"])
                .with_allowed_workload_kinds(["Deployment"])
                .with_allowed_image_registries(["ghcr.io"])
                .with_allowed_route_strategies([RouteStrategy::Header])
                .with_max_session_ttl("30m")
                .with_mutation_mode(MutationModePolicy::RouteMutation),
        ]);
        let route_mutation = evaluate_session_planning_policies(
            &route_mutation_policies,
            "shop",
            "Deployment",
            &image,
            Some(&time_to_live),
            "header",
        )
        .expect("route mutation policy should allow matching route action");
        assert!(
            route_mutation
                .session_policy
                .allows(SessionOperation::Route)
        );

        let denied_workload_kind = evaluate_session_planning_policies(
            &route_mutation_policies,
            "shop",
            "StatefulSet",
            &image,
            Some(&time_to_live),
            "header",
        )
        .expect_err("workload kind allowlist should deny mismatches");
        assert_policy_denial_contains(denied_workload_kind, "does not allow workload kind");

        let denied_registry = evaluate_session_planning_policies(
            &route_mutation_policies,
            "shop",
            "Deployment",
            &ImageRef::new("registry.example.com/acme/checkout:next").expect("image"),
            Some(&time_to_live),
            "header",
        )
        .expect_err("image registry allowlist should deny mismatches");
        assert_policy_denial_contains(denied_registry, "does not allow image registry");

        let denied_route_strategy = evaluate_session_planning_policies(
            &route_mutation_policies,
            "shop",
            "Deployment",
            &image,
            Some(&time_to_live),
            "host",
        )
        .expect_err("route strategy allowlist should deny mismatches");
        assert_policy_denial_contains(denied_route_strategy, "does not allow route strategy");

        let denied_time_to_live = evaluate_session_planning_policies(
            &route_mutation_policies,
            "shop",
            "Deployment",
            &image,
            Some(&long_time_to_live),
            "header",
        )
        .expect_err("max session ttl should deny longer sessions");
        assert_policy_denial_contains(denied_time_to_live, "above max_session_ttl");

        let sandbox_only_route_policies = PolicyConfigs::new(vec![
            PolicyConfig::new("sandbox-only")
                .with_allowed_namespaces(["shop"])
                .with_allowed_workload_kinds(["Deployment"])
                .with_allowed_image_registries(["ghcr.io"])
                .with_allowed_route_strategies([RouteStrategy::Header])
                .with_max_session_ttl("30m")
                .with_mutation_mode(MutationModePolicy::SandboxOnly),
        ]);
        let denied_route_mutation = evaluate_session_planning_policies(
            &sandbox_only_route_policies,
            "shop",
            "Deployment",
            &image,
            Some(&time_to_live),
            "header",
        )
        .expect_err("sandbox-only policy should deny route mutation");
        assert_policy_denial_contains(denied_route_mutation, "does not allow route mutation");
    }

    fn assert_policy_denial_contains(error: SessionPlanBuildError, expected: &str) {
        match error {
            SessionPlanBuildError::Policy(message) => {
                assert!(
                    message.contains(expected),
                    "policy denial `{message}` should contain `{expected}`"
                );
            }
            _ => panic!("expected policy denial"),
        }
    }

    #[test]
    fn selects_route_strategy_from_config_auto_and_explicit_overrides() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("registry.example.com/shop/checkout:test".to_owned()),
            RouteStrategy::Host,
        );

        let configured_plan = session_plan_from_config(&app, None, None, None, None)
            .unwrap_or_else(|_| panic!("configured route strategy should build a plan"));
        assert_eq!(planned_route_strategy(&configured_plan), "host");
        assert_eq!(
            configured_plan
                .route_selector()
                .expect("host strategy should set a route selector")
                .kind(),
            "host"
        );
        assert!(
            configured_plan
                .planned_resources()
                .iter()
                .any(|resource| resource.kind() == "HTTPRoute")
        );

        let auto_plan = session_plan_from_config(&app, None, None, None, Some("auto"))
            .unwrap_or_else(|_| panic!("auto route strategy should resolve to config"));
        assert_eq!(planned_route_strategy(&auto_plan), "host");
        assert_eq!(
            auto_plan
                .route_selector()
                .expect("auto host strategy should set a route selector")
                .kind(),
            "host"
        );

        let header_plan = session_plan_from_config(&app, None, None, None, Some("header"))
            .unwrap_or_else(|_| panic!("explicit header route strategy should build a plan"));
        assert_eq!(planned_route_strategy(&header_plan), "header");
        assert_eq!(
            header_plan
                .route_selector()
                .expect("header strategy should set a route selector")
                .kind(),
            "header"
        );

        let preview_service_plan =
            session_plan_from_config(&app, None, None, None, Some("preview-service"))
                .unwrap_or_else(|_| {
                    panic!("explicit preview-service route strategy should build a plan")
                });
        assert_eq!(
            planned_route_strategy(&preview_service_plan),
            "preview-service"
        );
        assert!(preview_service_plan.route_selector().is_none());
        assert!(
            !preview_service_plan
                .planned_resources()
                .iter()
                .any(|resource| resource.kind() == "HTTPRoute")
        );
        assert!(
            preview_service_plan
                .planned_checks()
                .iter()
                .any(|check| check.name() == "route_ready"
                    && check.target() == "shop/checkout-plan-service")
        );

        let none_plan = session_plan_from_config(&app, None, None, None, Some("none"))
            .unwrap_or_else(|_| panic!("explicit none route strategy should build a plan"));
        assert_eq!(planned_route_strategy(&none_plan), "none");
        assert!(none_plan.route_selector().is_none());
        assert!(
            !none_plan
                .planned_resources()
                .iter()
                .any(|resource| resource.kind() == "HTTPRoute")
        );
        assert!(
            !none_plan
                .planned_checks()
                .iter()
                .any(|check| check.name() == "route_ready")
        );
    }

    #[test]
    fn rejects_unknown_route_strategy_with_supported_strategy_list() {
        let app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("registry.example.com/shop/checkout:test".to_owned()),
            RouteStrategy::Header,
        );

        match session_plan_from_config(&app, None, None, None, Some("unknown")) {
            Err(SessionPlanBuildError::Usage(message)) => {
                assert_eq!(
                    message,
                    "unsupported route strategy `unknown`; expected auto, none, preview-service, header, host, preview"
                );
            }
            Err(SessionPlanBuildError::Config(_)) => panic!("expected a usage error"),
            Err(SessionPlanBuildError::Policy(_)) => panic!("expected a usage error"),
            Ok(_) => panic!("unknown route strategy should be rejected"),
        }
    }

    fn planned_route_strategy(plan: &SessionPlan) -> &str {
        plan.planned_annotations()
            .iter()
            .find(|annotation| annotation.key() == "kply.dev/route-strategy")
            .map(|annotation| annotation.value())
            .expect("route strategy annotation should be planned")
    }

    #[test]
    fn builds_planned_session_risk_notes_for_database_like_apps() {
        let checkout_app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("registry.example.com/shop/checkout:test".to_owned()),
            RouteStrategy::Header,
        );
        let database_app = AppConfig::new(
            "checkout-db",
            "shop",
            "checkout-postgres",
            "checkout-postgres",
            Some("postgres:16".to_owned()),
            RouteStrategy::Header,
        );

        let checkout_notes = planned_session_risk_notes(&checkout_app).expect("risk notes");
        let database_notes = planned_session_risk_notes(&database_app).expect("risk notes");

        assert!(checkout_notes.is_empty());
        assert_eq!(database_notes.len(), 1);
        assert_eq!(database_notes[0].category(), "database");
        assert_eq!(database_notes[0].severity(), "warning");
        assert_eq!(database_notes[0].target(), "app:checkout-db");
        assert_eq!(
            database_notes[0].reason(),
            "database_reference_requires_manual_review"
        );
    }

    #[test]
    fn builds_planned_session_risk_notes_for_database_like_workloads_services_and_images() {
        let workload_app = AppConfig::new(
            "checkout",
            "shop",
            "mysql-primary",
            "checkout-http",
            Some("registry.example.com/shop/checkout:test".to_owned()),
            RouteStrategy::Header,
        );
        let service_app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-postgres",
            Some("registry.example.com/shop/checkout:test".to_owned()),
            RouteStrategy::Header,
        );
        let image_app = AppConfig::new(
            "checkout",
            "shop",
            "checkout-api",
            "checkout-http",
            Some("postgres:16".to_owned()),
            RouteStrategy::Header,
        );

        assert_database_risk_note(&workload_app, "workload:mysql-primary");
        assert_database_risk_note(&service_app, "service:checkout-postgres");
        assert_database_risk_note(&image_app, "image:postgres:16");
    }

    fn assert_database_risk_note(app: &AppConfig, target: &str) {
        let notes = planned_session_risk_notes(app).expect("risk notes");

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].category(), "database");
        assert_eq!(notes[0].severity(), "warning");
        assert_eq!(notes[0].target(), target);
        assert_eq!(
            notes[0].reason(),
            "database_reference_requires_manual_review"
        );
    }

    fn label(key: &str, value: &str) -> LabelSelectorEntry {
        LabelSelectorEntry {
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    fn deployment_summary(
        namespace: &str,
        name: &str,
        pod_template_labels: &[LabelSelectorEntry],
    ) -> DeploymentSummary {
        DeploymentSummary {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            replicas: Some(1),
            available_replicas: Some(1),
            ready_replicas: Some(1),
            updated_replicas: Some(1),
            images: Vec::new(),
            pod_template_labels: pod_template_labels.to_vec(),
            probes: Vec::new(),
            resources: Vec::new(),
            rollout: DeploymentRolloutSummary {
                phase: DeploymentRolloutPhase::Complete,
                generation: Some(1),
                observed_generation: Some(1),
                desired_replicas: Some(1),
                ready_replicas: Some(1),
                available_replicas: Some(1),
                updated_replicas: Some(1),
                unavailable_replicas: Some(0),
                conditions: Vec::new(),
            },
        }
    }

    fn service_summary(
        namespace: &str,
        name: &str,
        selector: &[LabelSelectorEntry],
        ports: &[i32],
    ) -> ServiceSummary {
        ServiceSummary {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            service_type: Some("ClusterIP".to_owned()),
            selector: selector.to_vec(),
            ports: ports
                .iter()
                .map(|port| ServicePortSummary {
                    name: None,
                    port: *port,
                    app_protocol: None,
                    protocol: Some("TCP".to_owned()),
                    target_port: Some(port.to_string()),
                })
                .collect(),
        }
    }

    fn discovered_app(
        name: &str,
        namespace: &str,
        workload: &str,
        service: &str,
    ) -> InitDiscoveredApp {
        InitDiscoveredApp {
            name: name.to_owned(),
            namespace: namespace.to_owned(),
            workload: workload.to_owned(),
            workload_kind: "Deployment".to_owned(),
            service: service.to_owned(),
            route_strategy: "preview".to_owned(),
            ports: vec![80],
        }
    }

    fn init_report(
        apps: Vec<InitDiscoveredApp>,
        skipped_services: Vec<super::InitSkippedService>,
    ) -> super::InitReport {
        super::InitReport {
            command: "init",
            source: "cluster",
            status: "generated",
            output_path: "kply.yaml".to_owned(),
            cluster: super::InitClusterReport {
                cluster_url: "https://127.0.0.1:6443/".to_owned(),
                default_namespace: "default".to_owned(),
                namespaces_scanned: 2,
            },
            apps,
            skipped_services,
        }
    }

    #[test]
    fn planned_resource_tokens_preserve_long_session_uniqueness() {
        let shared_prefix = "a".repeat(54);
        let first_app = format!("{shared_prefix}1111");
        let second_app = format!("{shared_prefix}2222");
        let first_session = session_token(&first_app, "plan");
        let second_session = session_token(&second_app, "plan");

        assert_ne!(first_session, second_session);
        for suffix in ["workload", "service", "route"] {
            let first_resource = planned_resource_token(&first_session, suffix);
            let second_resource = planned_resource_token(&second_session, suffix);

            assert_ne!(first_resource, second_resource);
            assert!(first_resource.ends_with(suffix));
            assert!(second_resource.ends_with(suffix));
            assert!(first_resource.len() <= 63);
            assert!(second_resource.len() <= 63);
        }
    }
}
