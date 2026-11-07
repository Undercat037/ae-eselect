//! ae-eselect — modular configuration tool for aura-emerge
//!
//!   eselect <module> <action> [args]
//!
//! Modules: profile, repository, news, world, cache, db.
//! Man pages and shell completions are generated from this clap tree:
//!   eselect --gen-completions <shell>
//!   eselect --gen-manpage [DIR]

use std::io;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

mod modules;
mod paths;
mod util;

use modules::{cache, db, news, profile, repository, world};

#[derive(Parser)]
#[command(
    name = "eselect",
    version,
    about = "Configuration tool for aura-emerge / Cynosure",
    long_about = "Configuration tool for aura-emerge / Cynosure.\n\n\
        Gentoo-style: eselect <module> <action> [args]. Not part of emerge; \
        emerge ships only a minimal `eselect profile` shim.",
    after_help = "Files:\n  \
        /etc/portage/make.profile             selected profile (symlink)\n  \
        /etc/portage/repos.conf/eselect.conf  repositories managed by eselect\n  \
        /etc/portage/world                    world file\n  \
        /var/db/portage/profiles              profile tree\n  \
        /var/db/portage/repos/<name>/         repo templates and keys",
    arg_required_else_help = true,
    propagate_version = true
)]
struct Cli {
    /// Print shell completions to stdout
    #[arg(
        long = "gen-completions",
        value_enum,
        value_name = "SHELL",
        hide = true,
        exclusive = true
    )]
    gen_completions: Option<Shell>,

    /// Write man pages (one per module/action) into DIR; no DIR = main page on stdout
    #[arg(long = "gen-manpage", value_name = "DIR", hide = true, exclusive = true, num_args = 0..=1)]
    gen_manpage: Option<Option<PathBuf>>,

    #[command(subcommand)]
    module: Option<Module>,
}

#[derive(Subcommand)]
enum Module {
    /// Manage the /etc/portage/make.profile symlink
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// Manage repos.conf/eselect.conf and import repository keys
    Repository {
        #[command(subcommand)]
        action: RepositoryAction,
    },
    /// Read Arch news (delegates to `emerge --news`)
    News {
        #[command(subcommand)]
        action: Option<NewsAction>,
    },
    /// World file helpers
    World {
        #[command(subcommand)]
        action: WorldAction,
    },
    /// Clean pacman and aura-emerge caches
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Show, take or clear the pacman database lock
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
}

#[derive(Subcommand)]
enum ProfileAction {
    /// List available profiles
    List,
    /// Show the current profile
    Show,
    /// Select a profile by number or path
    Set {
        /// Number from `list`, or a path (absolute or relative to the profile tree)
        target: String,
    },
}

#[derive(Subcommand)]
enum RepositoryAction {
    /// List configured and known repositories
    List {
        /// Only repositories that are configured
        #[arg(short = 'i', long = "installed")]
        installed: bool,
    },
    /// Enable a repository (from a disabled entry or a template) and import its keys
    Enable {
        #[arg(required = true)]
        names: Vec<String>,
    },
    /// Disable a repository; the entry is kept so it can be re-enabled
    Disable {
        #[arg(required = true)]
        names: Vec<String>,
    },
    /// Remove a repository entry from eselect.conf
    Remove {
        #[arg(required = true)]
        names: Vec<String>,
        /// Do not ask for confirmation
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// Add a repository by URL
    Add {
        name: String,
        /// Server URL ($repo / $arch are expanded by alpm)
        sync_uri: String,
        /// Higher is preferred
        #[arg(short = 'p', long, default_value_t = 50)]
        priority: i32,
        /// Pacman SigLevel string, e.g. "Required DatabaseOptional"
        #[arg(long = "sig", value_name = "LEVEL")]
        sig: Option<String>,
    },
}

#[derive(Subcommand)]
enum NewsAction {
    /// List news, newest first
    List,
    /// Read item N, or mark everything read with `all`
    Read {
        /// Item number or "all"
        item: Option<String>,
    },
}

#[derive(Subcommand)]
enum WorldAction {
    /// Print the world file
    Show,
    /// Add pacman explicit packages that are missing from world
    Regen {
        /// Only show what would be added
        #[arg(short = 'p', long = "pretend")]
        pretend: bool,
        /// Ask before writing
        #[arg(short = 'a', long = "ask")]
        ask: bool,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Delete cached files
    Clean {
        /// /var/cache/pacman/pkg
        #[arg(long)]
        pacman: bool,
        /// AE source cache (~/.cache/aura-emerge/sources)
        #[arg(long)]
        sources: bool,
        /// AE built packages (~/.cache/aura-emerge/packages)
        #[arg(long)]
        packages: bool,
        /// AE build checkouts (~/.cache/aura-emerge/build)
        #[arg(long)]
        build: bool,
        /// All of the above
        #[arg(long)]
        all: bool,
        /// Only show what would be deleted
        #[arg(short = 'p', long = "pretend")]
        pretend: bool,
        /// Do not ask for confirmation
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum DbAction {
    /// Show lock status
    Status,
    /// Create db.lck
    Lock,
    /// Remove db.lck
    Unlock {
        /// Remove even if pacman/emerge looks to be running
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match run(cli) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("!!! {:#}", e);
            1
        }
    };
    std::process::exit(code);
}

fn run(cli: Cli) -> anyhow::Result<()> {
    if let Some(shell) = cli.gen_completions {
        let mut cmd = Cli::command();
        clap_complete::generate(shell, &mut cmd, "eselect", &mut io::stdout());
        return Ok(());
    }
    if let Some(dir) = cli.gen_manpage {
        let cmd = Cli::command();
        match dir {
            Some(d) => {
                std::fs::create_dir_all(&d)?;
                clap_mangen::generate_to(cmd, &d)?;
            }
            None => clap_mangen::Man::new(cmd).render(&mut io::stdout())?,
        }
        return Ok(());
    }
    let Some(module) = cli.module else {
        Cli::command().print_help()?;
        return Ok(());
    };
    match module {
        Module::Profile { action } => match action {
            ProfileAction::List => {
                profile::cmd_list();
                Ok(())
            }
            ProfileAction::Show => {
                profile::cmd_show();
                Ok(())
            }
            ProfileAction::Set { target } => profile::cmd_set(&target),
        },
        Module::Repository { action } => match action {
            RepositoryAction::List { installed } => repository::cmd_list(installed),
            RepositoryAction::Enable { names } => repository::cmd_enable(&names),
            RepositoryAction::Disable { names } => repository::cmd_disable(&names),
            RepositoryAction::Remove { names, force } => repository::cmd_remove(&names, force),
            RepositoryAction::Add {
                name,
                sync_uri,
                priority,
                sig,
            } => repository::cmd_add(&name, &sync_uri, priority, sig.as_deref()),
        },
        Module::News { action } => match action.unwrap_or(NewsAction::List) {
            NewsAction::List => news::cmd_list(),
            NewsAction::Read { item } => news::cmd_read(item.as_deref()),
        },
        Module::World { action } => match action {
            WorldAction::Show => world::cmd_show(),
            WorldAction::Regen { pretend, ask } => world::cmd_regen(pretend, ask),
        },
        Module::Cache { action } => match action {
            CacheAction::Clean {
                pacman,
                sources,
                packages,
                build,
                all,
                pretend,
                yes,
            } => cache::cmd_clean(&cache::Opts {
                pacman,
                sources,
                packages,
                build,
                all,
                pretend,
                yes,
            }),
        },
        Module::Db { action } => match action {
            DbAction::Status => {
                db::cmd_status();
                Ok(())
            }
            DbAction::Lock => db::cmd_lock(),
            DbAction::Unlock { force } => db::cmd_unlock(force),
        },
    }
}
