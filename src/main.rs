/*
Copyright (C) 2026 Undercat037
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, version 3 of the License


A modular configuration tool in the style of Gentoo
eselect for aura-emerge: select profile,
repositories and their keys,
the world file, caches, the package database lock
and Java environments - with Portage-like output.
*/

//! ae-eselect — modular configuration tool for aura-emerge
//!
//!   eselect <module> <action> [args]
//!
//! Modules: profile, repository, news, world, cache, db, java.
//! Man pages and shell completions are generated from this clap tree:
//!   eselect --gen-completions <shell>
//!   eselect --gen-manpage [DIR]

use std::io;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

mod help;
mod modules;
mod out;
mod paths;
mod util;

use modules::{cache, db, java, news, profile, repository, world};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColourMode {
    Yes,
    No,
    Auto,
}

#[derive(Parser)]
#[command(
    name = "eselect",
    version,
    about = "Configuration tool for aura-emerge",
    long_about = None,
    disable_help_subcommand = true,
    arg_required_else_help = false,
    propagate_version = true
)]
struct Cli {
    /// Make output shorter
    #[arg(long = "brief", global = true)]
    brief: bool,

    /// Enable or disable colour output
    #[arg(
        long = "colour",
        visible_alias = "color",
        value_name = "WHEN",
        default_value = "auto",
        global = true
    )]
    colour: ColourMode,

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
    #[arg(
        long = "gen-manpage",
        value_name = "DIR",
        hide = true,
        exclusive = true,
        num_args = 0..=1
    )]
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
    /// Clean package and aura-emerge caches
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Show, take or clear the alpm database lock
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
    /// Manage installed Java environments (built-in archlinux-java)
    Java {
        #[command(subcommand)]
        action: Option<JavaAction>,
    },
    /// Display a help message
    Help,
    /// Display a usage message
    Usage,
    /// Display version information
    Version,
}

#[derive(Subcommand)]
enum JavaAction {
    /// List Java environments, the default one is marked
    Status,
    /// Print the name of the default environment
    Get,
    /// Make an environment the default
    Set {
        /// Name from `status`, e.g. java-21-openjdk or java-8-openjdk/jre
        env: String,
    },
    /// Remove the default environment
    Unset,
    /// Repair the default link (pick the newest environment if it is broken)
    Fix,
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
    /// Print every package of a synced repository (plain, pipe-friendly)
    Packages {
        /// Repository name (a `<name>.db` in the pacman sync dir)
        name: String,
        /// Append the package version
        #[arg(short = 'v', long = "versions")]
        versions: bool,
        /// Prefix every line with `<repo>/`
        #[arg(short = 'r', long = "with-repo")]
        prefix: bool,
    },
    /// Add a repository by URL
    Add {
        name: String,
        /// Server URL ($repo / $arch are expanded by alpm)
        sync_uri: String,
        /// Higher is preferred
        #[arg(short = 'p', long, default_value_t = 50)]
        priority: i32,
        /// Signature level string, e.g. "Required DatabaseOptional"
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
    /// Add installed explicit packages that are missing from world
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
        /// Package cache (/var/cache/pacman/pkg)
        #[arg(long = "pkgcache", alias = "pacman")]
        pkgcache: bool,
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
        /// Remove even if emerge/alpm clients look to be running
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
}

fn main() {
    // Gentoo-style help screens (`eselect`, `eselect <module>`, ...) first.
    let mut argv: Vec<String> = std::env::args().collect();
    // `archlinux-java <action>` (symlink/hardlink to eselect) == `eselect java <action>`;
    // package install scripts call it by that name.
    let invoked = argv
        .first()
        .and_then(|a| std::path::Path::new(a).file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if invoked == "archlinux-java" {
        argv.insert(1, "java".to_string());
    }
    let args: Vec<String> = argv.iter().skip(1).cloned().collect();
    if let Some(code) = help::intercept(&args) {
        std::process::exit(code);
    }

    let cli = Cli::parse_from(&argv);
    apply_colour(cli.colour);
    out::set_brief(cli.brief);
    let code = match run(cli) {
        Ok(()) => 0,
        Err(e) => {
            out::write_error_msg(&format!("{:#}", e));
            1
        }
    };
    std::process::exit(code);
}

fn apply_colour(mode: ColourMode) {
    match mode {
        ColourMode::Yes => colored::control::set_override(true),
        ColourMode::No => colored::control::set_override(false),
        ColourMode::Auto => {}
    }
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
        help::print_main_help();
        return Ok(());
    };

    match module {
        Module::Help | Module::Usage => {
            help::print_main_help();
            Ok(())
        }
        Module::Version => {
            println!("eselect {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Module::Java { action } => match action.unwrap_or(JavaAction::Status) {
            JavaAction::Status => java::cmd_status(),
            JavaAction::Get => java::cmd_get(),
            JavaAction::Set { env } => java::cmd_set(&env),
            JavaAction::Unset => java::cmd_unset(),
            JavaAction::Fix => java::cmd_fix(),
        },
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
            RepositoryAction::Packages {
                name,
                versions,
                prefix,
            } => repository::cmd_packages(&name, versions, prefix),
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
                pkgcache,
                sources,
                packages,
                build,
                all,
                pretend,
                yes,
            } => cache::cmd_clean(&cache::Opts {
                pacman: pkgcache,
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
