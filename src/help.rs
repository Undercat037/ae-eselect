//! Gentoo-style help screens: `eselect`, `eselect help`, `eselect <module>`,
//! `eselect <module> help|usage|version`.
//!
//! These are handled before clap sees the command line. Everything else
//! (`eselect profile set --help`, man pages, completions) still comes from
//! the clap tree.

use crate::out::{write_error_msg, write_heading, write_kv_list_entry, write_kv_list_entry_at};

struct Action {
    name: &'static str,
    args: &'static str,
    desc: &'static str,
    opts: &'static [(&'static str, &'static str)],
}

struct Module {
    name: &'static str,
    desc: &'static str,
    actions: &'static [Action],
}

/// Alphabetical, like eselect's "Extra modules" list.
const MODULES: &[Module] = &[
    Module {
        name: "cache",
        desc: "Clean pacman and aura-emerge caches",
        actions: &[Action {
            name: "clean",
            args: "[<options>]",
            desc: "Delete cached files",
            opts: &[
                ("--pkgcache", "Package cache (/var/cache/pacman/pkg)"),
                ("--sources", "AE source cache of every user"),
                ("--packages", "AE built packages of every user"),
                ("--build", "AE build checkouts of every user"),
                ("--all", "All of the above"),
                ("-p, --pretend", "Only show what would be deleted"),
                ("-y, --yes", "Do not ask for confirmation"),
            ],
        }],
    },
    Module {
        name: "db",
        desc: "Show, take or clear the pacman database lock",
        actions: &[
            Action {
                name: "status",
                args: "",
                desc: "Show lock status",
                opts: &[],
            },
            Action {
                name: "lock",
                args: "",
                desc: "Create db.lck",
                opts: &[],
            },
            Action {
                name: "unlock",
                args: "[--force]",
                desc: "Remove db.lck",
                opts: &[(
                    "--force",
                    "Remove even if emerge/alpm clients look to be running",
                )],
            },
        ],
    },
    Module {
        name: "java",
        desc: "Manage installed Java environments (built-in archlinux-java)",
        actions: &[
            Action {
                name: "status",
                args: "",
                desc: "List Java environments, the default one is marked",
                opts: &[],
            },
            Action {
                name: "get",
                args: "",
                desc: "Print the name of the default environment",
                opts: &[],
            },
            Action {
                name: "set",
                args: "<env>",
                desc: "Make an environment the default",
                opts: &[(
                    "env",
                    "Name from 'status' (java-21-openjdk, java-8-openjdk/jre)",
                )],
            },
            Action {
                name: "unset",
                args: "",
                desc: "Remove the default environment",
                opts: &[],
            },
            Action {
                name: "fix",
                args: "",
                desc: "Repair the default link (newest environment if broken)",
                opts: &[],
            },
        ],
    },
    Module {
        name: "news",
        desc: "Read Arch news (delegates to `emerge --news`)",
        actions: &[
            Action {
                name: "list",
                args: "",
                desc: "List news items, newest first",
                opts: &[],
            },
            Action {
                name: "read",
                args: "<item>|all",
                desc: "Read item N, or mark everything read with `all`",
                opts: &[],
            },
        ],
    },
    Module {
        name: "profile",
        desc: "Manage the /etc/portage/make.profile symlink",
        actions: &[
            Action {
                name: "list",
                args: "",
                desc: "List available profile symlink targets",
                opts: &[],
            },
            Action {
                name: "show",
                args: "",
                desc: "Show the current make.profile symlink",
                opts: &[],
            },
            Action {
                name: "set",
                args: "<target>",
                desc: "Set a new profile symlink target",
                opts: &[("target", "Target name or number (from 'list' action)")],
            },
        ],
    },
    Module {
        name: "repository",
        desc: "Manage repos.conf/eselect.conf and import repository keys",
        actions: &[
            Action {
                name: "list",
                args: "[-i]",
                desc: "List configured and known repositories",
                opts: &[("-i, --installed", "Only repositories that are configured")],
            },
            Action {
                name: "enable",
                args: "<name>...",
                desc: "Enable a repository and import its keys",
                opts: &[],
            },
            Action {
                name: "disable",
                args: "<name>...",
                desc: "Disable a repository (the entry is kept)",
                opts: &[],
            },
            Action {
                name: "remove",
                args: "[-f] <name>...",
                desc: "Remove a repository entry from eselect.conf",
                opts: &[("-f, --force", "Do not ask for confirmation")],
            },
            Action {
                name: "packages",
                args: "[-v] [-r] <name>",
                desc: "Print every package of a synced repository",
                opts: &[
                    ("-v, --versions", "Append the package version"),
                    ("-r, --with-repo", "Prefix every line with <repo>/"),
                ],
            },
            Action {
                name: "add",
                args: "<name> <sync-uri>",
                desc: "Add a repository by URL",
                opts: &[
                    ("-p, --priority <N>", "Higher is preferred (default 50)"),
                    (
                        "--sig <level>",
                        "Signature level, e.g. \"Required DatabaseOptional\"",
                    ),
                ],
            },
        ],
    },
    Module {
        name: "world",
        desc: "World file helpers",
        actions: &[
            Action {
                name: "show",
                args: "",
                desc: "Print the world file",
                opts: &[],
            },
            Action {
                name: "regen",
                args: "[-p] [-a]",
                desc: "Add installed explicit packages missing from world",
                opts: &[
                    ("-p, --pretend", "Only show what would be added"),
                    ("-a, --ask", "Ask before writing"),
                ],
            },
        ],
    },
];

const BUILTIN: &[(&str, &str)] = &[
    ("help", "Display a help message"),
    ("usage", "Display a usage message"),
    ("version", "Display version information"),
];

fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn print_main_help() {
    println!("Usage: eselect <global options> <module name> <module options>");
    println!();
    write_heading("Global options:");
    write_kv_list_entry("--brief", "Make output shorter");
    write_kv_list_entry(
        "--colour=<yes|no|auto>",
        "Enable or disable colour output (default 'auto')",
    );
    println!();
    write_heading("Built-in modules:");
    for (n, d) in BUILTIN {
        write_kv_list_entry(n, d);
    }
    println!();
    write_heading("Extra modules:");
    for m in MODULES {
        write_kv_list_entry(m.name, m.desc);
    }
}

fn print_module_usage(m: &Module) {
    println!("Usage: eselect {} <action> <options>", m.name);
}

fn print_module_help(m: &Module) {
    println!("{}", m.desc);
    print_module_usage(m);
    println!();
    write_heading("Standard actions:");
    write_kv_list_entry("help", "Display help text");
    write_kv_list_entry("usage", "Display usage information");
    write_kv_list_entry("version", "Display version information");
    println!();
    write_heading("Extra actions:");
    for a in m.actions {
        let key = if a.args.is_empty() {
            a.name.to_string()
        } else {
            format!("{} {}", a.name, a.args)
        };
        write_kv_list_entry(&key, a.desc);
        for (k, d) in a.opts {
            write_kv_list_entry_at(4, k, d);
        }
    }
}

fn print_module_version(m: &Module) {
    println!("{}", m.desc);
    println!("Version {}", version());
    println!("Author: Undercat037");
}

/// Option parsing for the global flags that may precede the module.
/// Returns (index of first non-global argument, brief).
fn skip_globals(args: &[String]) -> (usize, bool) {
    let mut i = 0;
    let mut brief = false;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--brief" => brief = true,
            "--colour" | "--color" => {
                match args.get(i + 1).map(String::as_str) {
                    Some("yes") => colored::control::set_override(true),
                    Some("no") => colored::control::set_override(false),
                    _ => {}
                }
                i += 1;
            }
            _ if a.starts_with("--colour=") || a.starts_with("--color=") => {
                if let Some(v) = a
                    .strip_prefix("--colour=")
                    .or_else(|| a.strip_prefix("--color="))
                {
                    match v {
                        "yes" => colored::control::set_override(true),
                        "no" => colored::control::set_override(false),
                        _ => {}
                    }
                }
            }
            _ => break,
        }
        i += 1;
    }
    (i.min(args.len()), brief)
}

/// Handle the Gentoo-style screens. `Some(code)` = done, exit with `code`;
/// `None` = let clap parse the command line.
pub fn intercept(args: &[String]) -> Option<i32> {
    // `--colour no` (two words) is handled by clap later; for the help
    // screens only the `=` form and the plain flags matter.
    let (i, brief) = skip_globals(args);
    crate::out::set_brief(brief);

    let Some(first) = args.get(i).map(String::as_str) else {
        // nothing but global options
        if args
            .iter()
            .any(|a| a == "--gen-completions" || a == "--gen-manpage")
        {
            return None;
        }
        print_main_help();
        return Some(0);
    };

    match first {
        "-h" | "--help" => {
            print_main_help();
            return Some(0);
        }
        "-V" | "--version" | "version" => {
            println!("eselect {}", version());
            return Some(0);
        }
        "help" | "usage" => {
            print_main_help();
            return Some(0);
        }
        _ => {}
    }
    if first.starts_with('-') {
        return None; // --gen-*, unknown flags: clap decides
    }

    let Some(m) = MODULES.iter().find(|m| m.name == first) else {
        write_error_msg(&format!("Unknown module '{}'", first));
        return Some(1);
    };

    let action = args.get(i + 1).map(String::as_str);
    match action {
        None if m.name == "news" => None, // `eselect news` lists, as before
        None => {
            print_module_help(m);
            Some(0)
        }
        Some("help" | "-h" | "--help") => {
            print_module_help(m);
            Some(0)
        }
        Some("usage") => {
            print_module_usage(m);
            Some(0)
        }
        Some("version") => {
            print_module_version(m);
            Some(0)
        }
        Some(a) if !a.starts_with('-') && !m.actions.iter().any(|x| x.name == a) => {
            write_error_msg(&format!("Unknown action '{}'", a));
            Some(1)
        }
        _ => None,
    }
}
