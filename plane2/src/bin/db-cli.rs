use clap::{Parser, Subcommand};
use colored::{self, Colorize};
use plane2::{
    database::{connect, node::NodeRow},
    names::DroneName,
    types::{ClusterId, NodeStatus},
    util::format_duration,
};

#[derive(Parser)]
struct Opts {
    #[clap(long)]
    db: String,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Events,
    ListDrones {
        #[clap(long)]
        all: bool,

        #[clap(long)]
        cluster: Option<ClusterId>,
    },
    ListBackends,
    TerminationCandidates {
        #[clap(long)]
        cluster: ClusterId,

        #[clap(long)]
        drone: DroneName,
    },
}

fn describe_drone_status(drone: &NodeRow) -> String {
    let age_string = format_duration(drone.status_age());
    match drone.last_status {
        NodeStatus::Available => format!("{} {} ago", "Available".green(), age_string.white()),
        NodeStatus::Terminated => format!("{} {} ago", "Terminated".black(), age_string.white()),
        NodeStatus::Starting => format!("{} {} ago", "Starting".yellow(), age_string.white()),
    }
}

async fn main_inner(opts: Opts) -> anyhow::Result<()> {
    let db = connect(&opts.db).await?;

    match opts.command {
        Command::Events => {
            let mut events = db.subscribe_all_events();

            while let Ok(event) = events.recv().await {
                println!(
                    "{} {} {} {} {}",
                    event.timestamp.to_string().white(),
                    event.id.to_string().red(),
                    event.key.unwrap_or_else(|| "<global>".to_string()).yellow(),
                    event.kind.magenta(),
                    serde_json::to_string(&event.payload)?.blue()
                );
            }
        }
        Command::ListDrones { all, cluster } => {
            let drones = db.node().list().await?;

            for drone in drones {
                if let Some(cluster) = &cluster {
                    if drone.cluster.as_ref() != Some(cluster) {
                        continue;
                    }
                }

                if !all && !drone.active() {
                    continue;
                }

                if drone.active() {
                    println!(
                        "{} to {} {} {} Plane={}@{} {}",
                        "Connected".green(),
                        drone
                            .controller
                            .as_ref()
                            .map(|d| d.to_string())
                            .unwrap_or_default()
                            .green(),
                        drone
                            .cluster
                            .as_ref()
                            .map(|d| d.to_string().purple())
                            .unwrap_or_default(),
                        drone.name.to_string().green(),
                        drone.plane_version.yellow(),
                        drone.plane_hash.yellow(),
                        describe_drone_status(&drone),
                    );
                } else if all {
                    println!(
                        "{} {} {} Plane={}@{} (was: {})",
                        "Disconnected".yellow(),
                        drone
                            .cluster
                            .as_ref()
                            .map(|d| d.to_string().purple())
                            .unwrap_or_default(),
                        drone.name.to_string().green(),
                        drone.plane_version.yellow(),
                        drone.plane_hash.yellow(),
                        describe_drone_status(&drone),
                    );
                }
            }
        }
        Command::ListBackends => {
            let backends = db.backend().list_backends().await?;

            for backend in backends {
                println!(
                    "{} {} {} {} {}",
                    backend.id.to_string().blue(),
                    backend.cluster.green(),
                    backend.last_status.to_string().yellow(),
                    backend.last_status_time.to_string().white(),
                    backend.drone_id.to_string().green(),
                );
            }
        }
        Command::TerminationCandidates { cluster, drone } => {
            let drone_id = db.node().get_id(&cluster, &drone).await?;

            if let Some(drone_id) = drone_id {
                let backends = db.backend().termination_candidates(drone_id).await?;

                for termination_candidate in backends {
                    if let Some(expiration_time) = termination_candidate.expiration_time {
                        if expiration_time > termination_candidate.as_of {
                            println!(
                                "{} is alive past expiration time {}",
                                termination_candidate.backend_id.to_string().blue(),
                                expiration_time.to_string().white(),
                            );
                            continue;
                        }
                    }

                    if let Some(allowed_idle_seconds) = termination_candidate.allowed_idle_seconds {
                        let overage = termination_candidate.as_of
                            - termination_candidate.last_keepalive
                            - chrono::Duration::seconds(allowed_idle_seconds.into());
                        if overage > chrono::Duration::zero() {
                            println!(
                                "{} is alive past allowed {} seconds past idle time {}",
                                termination_candidate.backend_id.to_string().blue(),
                                overage.num_seconds().to_string().white(),
                                allowed_idle_seconds.to_string().white(),
                            );
                            continue;
                        }
                    }

                    println!(
                        "{} is a candidate for termination ({:?})",
                        termination_candidate.backend_id.to_string().blue(),
                        termination_candidate,
                    );
                }
            } else {
                println!("No such drone: {} on {}", drone, cluster);
            }
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() {
    let opts = Opts::parse();

    if let Err(e) = main_inner(opts).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}