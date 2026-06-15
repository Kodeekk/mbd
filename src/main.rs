use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

mod config;
mod daemon;

#[derive(Parser)]
#[command(name = "mbd", version, about = "M-Button Daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Run daemon in foreground
    Run {
        #[arg(short, long, help = "Shell command to run on M-Button press")]
        command: Option<String>,
        #[arg(short, long, help = "Path to script/executable to run directly")]
        script: Option<String>,
        #[arg(short, long)]
        mac: Option<String>,
    },
    /// Start daemon in background
    Start {
        #[arg(short, long, help = "Shell command (saves to config)")]
        command: Option<String>,
        #[arg(short, long, help = "Script path (saves to config)")]
        script: Option<String>,
    },
    /// Stop daemon
    Stop,
    /// Show daemon status
    Status,
}

fn state_dir() -> PathBuf {
    dirs::state_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("mbd")
}

fn pid_path() -> PathBuf {
    state_dir().join("mbd.pid")
}

fn proc_alive(pid: u32) -> bool {
    PathBuf::from(format!("/proc/{pid}")).exists()
}

fn read_pid() -> Option<u32> {
    let content = fs::read_to_string(pid_path()).ok()?;
    content.trim().parse().ok()
}

fn write_pid(pid: u32) -> anyhow::Result<()> {
    fs::create_dir_all(state_dir())?;
    fs::write(pid_path(), pid.to_string())?;
    Ok(())
}

fn rm_pid() {
    fs::remove_file(pid_path()).ok();
}

fn start() -> anyhow::Result<()> {
    if let Some(pid) = read_pid() {
        if proc_alive(pid) {
            println!("mbd: already running (PID {pid})");
            return Ok(());
        }
    }

    let exe = std::env::current_exe()?;
    let child = std::process::Command::new(&exe)
        .arg("run")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()?;

    write_pid(child.id())?;
    println!("mbd: started (PID {})", child.id());
    Ok(())
}

fn stop() -> anyhow::Result<()> {
    let pid = match read_pid() {
        Some(p) => p,
        None => {
            println!("mbd: not running");
            return Ok(());
        }
    };

    if !proc_alive(pid) {
        println!("mbd: not running");
        rm_pid();
        return Ok(());
    }

    let ok = std::process::Command::new("kill")
        .arg(pid.to_string())
        .status()?
        .success();

    if ok {
        println!("mbd: stopped");
    } else {
        eprintln!("mbd: failed to stop PID {pid}");
    }
    rm_pid();
    Ok(())
}

fn status() -> anyhow::Result<()> {
    match read_pid() {
        Some(pid) if proc_alive(pid) => println!("mbd: running (PID {pid})"),
        Some(_) => {
            println!("mbd: not running");
            rm_pid();
        }
        None => println!("mbd: not running"),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let run_default = Command::Run {
        command: None,
        script: None,
        mac: None,
    };
    match cli.command.unwrap_or(run_default) {
        Command::Run { command, script, mac } => {
            let mut cfg = config::Config::load().unwrap_or_default();
            let mut changed = false;

            if let Some(cmd) = command {
                cfg.command = cmd;
                cfg.mode = None;
                changed = true;
            } else if let Some(path) = script {
                cfg.command = path;
                cfg.mode = Some("script".into());
                changed = true;
            }
            if let Some(m) = mac {
                cfg.mac = Some(m);
            }
            if cli.verbose {
                cfg.verbose = Some(true);
            }

            if cfg.command.is_empty() {
                eprintln!("mbd: no command configured.
Set it in ~/.config/mbd/config.toml or pass --command/--script");
                std::process::exit(1);
            }

            if changed {
                cfg.save()?;
                println!("mbd: config saved ({})", config::Config::path().display());
            }

            let script_mode = cfg.mode.as_deref() == Some("script");
            let d = daemon::Daemon::new(
                cfg.command,
                cfg.verbose.unwrap_or(false),
                cfg.mac,
                script_mode,
            );
            d.run().await
        }
        Command::Start { command, script } => {
            if command.is_some() || script.is_some() {
                let mut cfg = config::Config::load().unwrap_or_default();
                if let Some(cmd) = command {
                    cfg.command = cmd;
                    cfg.mode = None;
                } else if let Some(path) = script {
                    cfg.command = path;
                    cfg.mode = Some("script".into());
                }
                cfg.save()?;
                println!("mbd: config saved ({})", config::Config::path().display());
            }
            start()
        }
        Command::Stop => stop(),
        Command::Status => status(),
    }
}
