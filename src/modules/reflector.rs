//! `eselect reflector` — rank the mirrors of repositories and rewrite their
//! mirrorlists, fastest first.
//!
//! Where a repository's mirrorlist lives depends on `FEATURES="repos"` in
//! make.conf, and only that source is read: with it `/etc/portage/repos.conf`,
//! without it `/etc/pacman.conf`. The other file is never consulted, so
//! repositories that are not active in the current mode are left alone. The candidate pool is
//! every `Server =` line of that file, active **or commented out**, so a
//! mirrorlist trimmed to one dead server can be revived. Mirrors are probed
//! with `curl` (a pacman dependency): the first 512 KiB of `<repo>.db`.
//!
//! The rewritten file keeps every URL: the fastest `--number` are active, the
//! rest stay as commented fallbacks (aura-emerge tries those when a sync
//! fails). The previous file is saved as `<file>.eselect-bak`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::modules::repository::features_has_repos;
use crate::out;
use crate::paths::REPOS_CONF;
use crate::util::fs::atomic_write;
use crate::util::root::require_root;

const PACMAN_CONF: &str = "/etc/pacman.conf";
/// Bytes of `<repo>.db` fetched per probe.
const PROBE_BYTES: u64 = 512 * 1024;

pub struct RankOpts {
    pub repos: Vec<String>,
    pub number: usize,
    pub timeout: u64,
    pub jobs: usize,
    pub protocol: String,
    pub active_only: bool,
    pub pretend: bool,
}

// ---------------------------------------------------------------- conf scan

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepoInc {
    repo: String,
    includes: Vec<PathBuf>,
}

/// `[repo]` sections with their `Include =` files. `Include` outside a repo
/// section (pacman.conf pulling in more conf files) goes to `nested`.
fn scan_text(text: &str, nested: &mut Vec<PathBuf>) -> Vec<RepoInc> {
    let mut out: Vec<RepoInc> = Vec::new();
    let mut cur: Option<usize> = None;
    for raw in text.lines() {
        // `#| [x]` (disabled eselect block) and plain comments both start with '#'.
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let name = name.trim();
            if name.eq_ignore_ascii_case("options") {
                cur = None;
            } else {
                out.push(RepoInc {
                    repo: name.to_string(),
                    includes: Vec::new(),
                });
                cur = Some(out.len() - 1);
            }
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if !k.trim().eq_ignore_ascii_case("include") {
            continue;
        }
        let v = v.trim();
        if v.is_empty() || v.contains('*') {
            continue;
        }
        match cur {
            Some(i) => out[i].includes.push(PathBuf::from(v)),
            None => nested.push(PathBuf::from(v)),
        }
    }
    out
}

/// A conf file, or a directory of `*.conf` (repos.conf), plus nested includes.
fn scan_source(path: &Path, depth: u8) -> Vec<RepoInc> {
    let mut files: Vec<PathBuf> = Vec::new();
    if path.is_dir() {
        if let Ok(rd) = fs::read_dir(path) {
            for e in rd.flatten() {
                let p = e.path();
                let hidden = e.file_name().to_string_lossy().starts_with('.');
                if !hidden && p.extension().and_then(|x| x.to_str()) == Some("conf") {
                    files.push(p);
                }
            }
        }
        files.sort();
    } else if path.is_file() {
        files.push(path.to_path_buf());
    }
    let mut out = Vec::new();
    for f in files {
        let Ok(text) = fs::read_to_string(&f) else {
            continue;
        };
        let mut nested = Vec::new();
        out.extend(scan_text(&text, &mut nested));
        if depth < 3 {
            for n in nested {
                out.extend(scan_source(&n, depth + 1));
            }
        }
    }
    out
}

type Sources = Vec<(&'static str, Vec<RepoInc>)>;

/// The one source that is in effect: repos.conf with FEATURES="repos",
/// pacman.conf without it. No fallback to the other file.
fn sources() -> Sources {
    if features_has_repos() {
        vec![("repos.conf", scan_source(Path::new(REPOS_CONF), 0))]
    } else {
        vec![("pacman.conf", scan_source(Path::new(PACMAN_CONF), 0))]
    }
}

fn source_label(srcs: &Sources) -> &'static str {
    srcs.first().map(|s| s.0).unwrap_or("configuration")
}

fn resolve(repo: &str, srcs: &Sources) -> Result<(PathBuf, &'static str)> {
    let mut defined = false;
    for (label, list) in srcs {
        for r in list.iter().filter(|r| r.repo == repo) {
            defined = true;
            if let Some(p) = r.includes.first() {
                return Ok((p.clone(), *label));
            }
        }
    }
    if defined {
        bail!(
            "[{}] has no Include mirrorlist (a single sync-uri / Server) — nothing to rank",
            repo
        );
    }
    let label = srcs.first().map(|s| s.0).unwrap_or("configuration");
    let note = if label == "repos.conf" {
        "FEATURES=\"repos\" is set, so pacman.conf is not consulted"
    } else {
        "FEATURES=\"repos\" is not set, so repos.conf is not consulted"
    };
    bail!(
        "repository '{}' is not defined in {} ({})",
        repo,
        label,
        note
    )
}

// ------------------------------------------------------------ mirror lists

/// `(url, active)` for every `Server =` line, commented ones included.
fn parse_mirrors(text: &str) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for line in text.lines() {
        let active = !line.trim_start().starts_with('#');
        let t = line.trim_start_matches(|c: char| c == '#' || c.is_whitespace());
        let Some((k, v)) = t.split_once('=') else {
            continue;
        };
        if !k.trim().eq_ignore_ascii_case("server") {
            continue;
        }
        let url = v.split('#').next().unwrap_or("").trim();
        if url.is_empty() || !url.contains("://") || url.contains(char::is_whitespace) {
            continue;
        }
        match out.iter_mut().find(|(u, _)| u == url) {
            Some(e) => e.1 |= active,
            None => out.push((url.to_string(), active)),
        }
    }
    out
}

fn expand(url: &str, repo: &str) -> String {
    let arch = std::env::consts::ARCH;
    url.replace("$arch_v3", &format!("{arch}_v3"))
        .replace("$arch_v4", &format!("{arch}_v4"))
        .replace("$repo", repo)
        .replace("$arch", arch)
}

fn protocol_ok(url: &str, protocol: &str) -> bool {
    match protocol {
        "https" => url.starts_with("https://"),
        "http" => url.starts_with("http://"),
        _ => url.starts_with("https://") || url.starts_with("http://"),
    }
}

// ----------------------------------------------------------------- probing

fn parse_curl(out: &str) -> Option<(u16, f64)> {
    let mut it = out.split_whitespace();
    let code = it.next()?.parse().ok()?;
    let secs = it.next()?.parse().ok()?;
    Some((code, secs))
}

fn curl_reason(code: Option<i32>, stderr: &str) -> String {
    match code {
        Some(28) => "timeout".to_string(),
        Some(6) => "DNS error".to_string(),
        Some(7) => "refused".to_string(),
        Some(35 | 51 | 58 | 60) => "TLS error".to_string(),
        _ => stderr
            .lines()
            .next()
            .unwrap_or("curl failed")
            .trim()
            .trim_start_matches("curl: ")
            .to_string(),
    }
}

/// Seconds to fetch the head of `<repo>.db` from this mirror.
fn probe(url_tpl: &str, repo: &str, timeout: u64) -> std::result::Result<f64, String> {
    let url = format!(
        "{}/{}.db",
        expand(url_tpl, repo).trim_end_matches('/'),
        repo
    );
    let t = timeout.to_string();
    let range = format!("0-{}", PROBE_BYTES - 1);
    let out = Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--max-redirs",
            "3",
            "--proto",
            "=http,https",
            "--proto-redir",
            "=http,https",
            "-o",
            "/dev/null",
            "-r",
            range.as_str(),
            "--max-time",
            t.as_str(),
            "-w",
            "%{http_code} %{time_total}",
            url.as_str(),
        ])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "curl not found".to_string()
            } else {
                e.to_string()
            }
        })?;
    if !out.status.success() {
        return Err(curl_reason(
            out.status.code(),
            &String::from_utf8_lossy(&out.stderr),
        ));
    }
    match parse_curl(&String::from_utf8_lossy(&out.stdout)) {
        Some((200 | 206, secs)) => Ok(secs),
        Some((code, _)) => Err(format!("HTTP {}", code)),
        None => Err("unreadable curl output".to_string()),
    }
}

fn probe_all(
    cands: &[String],
    repo: &str,
    timeout: u64,
    jobs: usize,
) -> Vec<std::result::Result<f64, String>> {
    let results: Mutex<Vec<Option<std::result::Result<f64, String>>>> =
        Mutex::new(vec![None; cands.len()]);
    let next = AtomicUsize::new(0);
    let workers = jobs.clamp(1, 32).min(cands.len().max(1));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= cands.len() {
                        break;
                    }
                    let r = probe(&cands[i], repo, timeout);
                    if let Ok(mut g) = results.lock() {
                        g[i] = Some(r);
                    }
                }
            });
        }
    });
    results
        .into_inner()
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.unwrap_or_else(|| Err("not probed".to_string())))
        .collect()
}

// --------------------------------------------------------------- rendering

/// `YYYY-MM-DD` from unix seconds (civil-from-days).
fn ymd(secs: u64) -> String {
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

struct Outcome {
    /// reachable, fastest first
    ranked: Vec<(String, f64)>,
    /// unreachable: (url, reason)
    failed: Vec<(String, String)>,
    /// not probed (protocol filter / --active-only / non-http scheme)
    skipped: Vec<String>,
}

fn render(repos: &[String], o: &Outcome, number: usize, protocol: &str, date: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("## Generated by eselect reflector on {}\n", date));
    s.push_str(&format!("## repositories: {}\n", repos.join(" ")));
    s.push_str(&format!(
        "## fastest first; the top {} are active, the rest are commented fallbacks (protocol: {})\n\n",
        number, protocol
    ));
    for (url, secs) in o.ranked.iter().take(number) {
        s.push_str(&format!("Server = {} # {:.3}s\n", url, secs));
    }
    if o.ranked.len() > number {
        s.push_str("\n## Reachable, slower\n");
        for (url, secs) in o.ranked.iter().skip(number) {
            s.push_str(&format!("#Server = {} # {:.3}s\n", url, secs));
        }
    }
    if !o.failed.is_empty() {
        s.push_str("\n## Unreachable when probed\n");
        for (url, why) in &o.failed {
            s.push_str(&format!("#Server = {} # {}\n", url, why));
        }
    }
    if !o.skipped.is_empty() {
        s.push_str("\n## Not probed\n");
        for url in &o.skipped {
            s.push_str(&format!("#Server = {}\n", url));
        }
    }
    s
}

// -------------------------------------------------------------- operations

fn check_curl() -> Result<()> {
    match Command::new("curl").arg("--version").output() {
        Ok(_) => Ok(()),
        Err(_) => bail!("curl is required for probing mirrors (it is a pacman dependency)"),
    }
}

fn rank_file(path: &Path, repos: &[String], opts: &RankOpts) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let all = parse_mirrors(&text);
    if all.is_empty() {
        bail!("{} has no Server lines", path.display());
    }
    let mut cands: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for (url, active) in &all {
        if (opts.active_only && !active) || !protocol_ok(url, &opts.protocol) {
            skipped.push(url.clone());
        } else {
            cands.push(url.clone());
        }
    }
    let probe_repo = &repos[0];
    println!(
        "{} Ranking {} ({}; {} candidate(s), probing {}.db, timeout {}s)",
        ">>>".green().bold(),
        path.display(),
        repos.join(", "),
        cands.len(),
        probe_repo,
        opts.timeout
    );
    let results = probe_all(&cands, probe_repo, opts.timeout, opts.jobs);
    let mut ranked: Vec<(String, f64)> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    for (url, r) in cands.into_iter().zip(results) {
        match r {
            Ok(secs) => ranked.push((url, secs)),
            Err(why) => failed.push((url, why)),
        }
    }
    ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    if ranked.is_empty() {
        bail!(
            "no reachable mirror for {} — file left unchanged (is the network up?)",
            path.display()
        );
    }

    if out::is_brief() {
        for (url, _) in ranked.iter().take(opts.number) {
            println!("{}", url);
        }
    } else {
        for (i, (url, secs)) in ranked.iter().enumerate() {
            let mark = if i < opts.number {
                "*".blue().bold().to_string()
            } else {
                " ".to_string()
            };
            println!("  {} [{:>2}] {:>6.3}s  {}", mark, i + 1, secs, url);
        }
        for (url, why) in &failed {
            println!(
                "  {} {}  {}",
                "!".yellow().bold(),
                format!("{:<9}", why).dimmed(),
                url
            );
        }
    }

    if opts.pretend {
        println!(
            "{} (pretend) {} not modified",
            ">>>".yellow().bold(),
            path.display()
        );
        return Ok(());
    }
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        bail!("{} is a symlink — refusing to replace it", path.display());
    }
    let mut bak_name = path.file_name().unwrap_or_default().to_os_string();
    bak_name.push(".eselect-bak");
    let bak = path.with_file_name(bak_name);
    fs::copy(path, &bak).with_context(|| format!("backup to {}", bak.display()))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let body = render(
        repos,
        &Outcome {
            ranked,
            failed,
            skipped,
        },
        opts.number,
        &opts.protocol,
        &ymd(now),
    );
    atomic_write(path, &body).with_context(|| format!("write {}", path.display()))?;
    println!(
        "{} wrote {} (previous: {})",
        ">>>".green().bold(),
        path.display(),
        bak.display()
    );
    Ok(())
}

pub fn cmd_rank(opts: RankOpts) -> Result<()> {
    if opts.repos.is_empty() {
        bail!("name at least one repository, e.g.: eselect reflector core blackarch");
    }
    if opts.number == 0 {
        bail!("--number must be at least 1");
    }
    if !(1..=60).contains(&opts.timeout) {
        bail!("--timeout must be 1..60 seconds");
    }
    if !matches!(opts.protocol.as_str(), "https" | "http" | "all") {
        bail!("--protocol must be https, http or all");
    }
    for r in &opts.repos {
        if r.is_empty()
            || !r
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            bail!("invalid repository name '{}'", r);
        }
    }
    check_curl()?;
    if !opts.pretend {
        require_root("eselect reflector")?;
    }

    let srcs = sources();
    // file -> repositories that use it (several repos share one mirrorlist)
    let mut groups: Vec<(PathBuf, Vec<String>)> = Vec::new();
    let mut problems = 0usize;
    for repo in &opts.repos {
        match resolve(repo, &srcs) {
            Ok((path, label)) => {
                if !path.is_file() {
                    out::write_error_msg(&format!(
                        "[{}] ({}) includes {} which does not exist",
                        repo,
                        label,
                        path.display()
                    ));
                    problems += 1;
                    continue;
                }
                match groups.iter_mut().find(|(p, _)| *p == path) {
                    Some((_, v)) => v.push(repo.clone()),
                    None => groups.push((path, vec![repo.clone()])),
                }
            }
            Err(e) => {
                out::write_error_msg(&e.to_string());
                problems += 1;
            }
        }
    }
    for (path, repos) in &groups {
        if let Err(e) = rank_file(path, repos, &opts) {
            out::write_error_msg(&format!("{}: {}", path.display(), e));
            problems += 1;
        }
    }
    if problems > 0 {
        bail!("{} problem(s); see messages above", problems);
    }
    Ok(())
}

/// Which mirrorlist each configured repository uses.
pub fn cmd_list() -> Result<()> {
    let srcs = sources();
    out::write_list_start(&format!(
        "Mirrorlists of configured repositories ({}):",
        source_label(&srcs)
    ));
    let mut seen: Vec<String> = Vec::new();
    let mut any = false;
    for (_, list) in &srcs {
        for r in list {
            if seen.contains(&r.repo) {
                continue;
            }
            seen.push(r.repo.clone());
            any = true;
            let detail = match r.includes.first() {
                None => "(sync-uri only, nothing to rank)".dimmed().to_string(),
                Some(p) => match fs::read_to_string(p) {
                    Ok(t) => {
                        let m = parse_mirrors(&t);
                        let act = m.iter().filter(|(_, a)| *a).count();
                        format!("{}  {} active / {} total", p.display(), act, m.len())
                    }
                    Err(_) => format!("{}  {}", p.display(), "(missing)".red()),
                },
            };
            println!("  {:<18} {}", r.repo.bold(), detail);
        }
    }
    if !any {
        out::write_none_found("(no repositories found)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrors_include_commented_and_dedupe() {
        let t = "## Germany\nServer = https://a.example/$repo/os/$arch\n#Server = https://b.example/$repo # note\n# Server = https://c.example/x\n#just prose\nServer = https://a.example/$repo/os/$arch\n";
        assert_eq!(
            parse_mirrors(t),
            vec![
                ("https://a.example/$repo/os/$arch".to_string(), true),
                ("https://b.example/$repo".to_string(), false),
                ("https://c.example/x".to_string(), false),
            ]
        );
    }

    #[test]
    fn scan_finds_includes_and_nested_confs() {
        let t = "[options]\nInclude = /etc/pacman.d/extra.conf\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n#| [off]\n#| include = /x\n[g14]\nServer = https://arch.asus-linux.org\n";
        let mut nested = Vec::new();
        let v = scan_text(t, &mut nested);
        assert_eq!(nested, vec![PathBuf::from("/etc/pacman.d/extra.conf")]);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].repo, "core");
        assert_eq!(
            v[0].includes,
            vec![PathBuf::from("/etc/pacman.d/mirrorlist")]
        );
        assert!(v[1].includes.is_empty());
    }

    #[test]
    fn resolve_follows_source_order_and_explains_failures() {
        let rc = vec![RepoInc {
            repo: "blackarch".into(),
            includes: vec![PathBuf::from("/etc/pacman.d/blackarch-mirrorlist")],
        }];
        let pc = vec![RepoInc {
            repo: "g14".into(),
            includes: vec![],
        }];
        let srcs: Sources = vec![("repos.conf", rc), ("pacman.conf", pc)];
        let (p, l) = resolve("blackarch", &srcs).unwrap();
        assert_eq!(p, PathBuf::from("/etc/pacman.d/blackarch-mirrorlist"));
        assert_eq!(l, "repos.conf");
        assert!(
            resolve("g14", &srcs)
                .unwrap_err()
                .to_string()
                .contains("nothing to rank")
        );
        assert!(
            resolve("nope", &srcs)
                .unwrap_err()
                .to_string()
                .contains("not defined")
        );
    }

    #[test]
    fn expand_handles_arch_variants() {
        let a = std::env::consts::ARCH;
        assert_eq!(
            expand("https://m/repo/$arch_v3/$repo", "cachyos-v3"),
            format!("https://m/repo/{a}_v3/cachyos-v3")
        );
        assert_eq!(
            expand("https://m/$repo/os/$arch", "core"),
            format!("https://m/core/os/{a}")
        );
    }

    #[test]
    fn curl_output_and_reasons() {
        assert_eq!(parse_curl("206 0.234"), Some((206, 0.234)));
        assert_eq!(parse_curl("garbage"), None);
        assert_eq!(curl_reason(Some(28), ""), "timeout");
        assert_eq!(curl_reason(Some(6), ""), "DNS error");
    }

    #[test]
    fn protocol_filter() {
        assert!(protocol_ok("https://a", "https"));
        assert!(!protocol_ok("http://a", "https"));
        assert!(protocol_ok("http://a", "all"));
        assert!(!protocol_ok("ftp://a", "all"));
    }

    #[test]
    fn dates() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(951_782_400), "2000-02-29");
        assert_eq!(ymd(951_868_800), "2000-03-01");
    }

    #[test]
    fn render_keeps_every_url() {
        let o = Outcome {
            ranked: vec![
                ("https://fast/x".into(), 0.1),
                ("https://slow/x".into(), 0.9),
            ],
            failed: vec![("https://dead/x".into(), "timeout".into())],
            skipped: vec!["http://plain/x".into()],
        };
        let s = render(&["core".into()], &o, 1, "https", "2026-11-16");
        assert!(s.contains("\nServer = https://fast/x # 0.100s\n"));
        assert!(s.contains("#Server = https://slow/x # 0.900s"));
        assert!(s.contains("#Server = https://dead/x # timeout"));
        assert!(s.contains("#Server = http://plain/x"));
        // exactly one active server
        assert_eq!(s.lines().filter(|l| l.starts_with("Server =")).count(), 1);
        // and it round-trips through our own parser
        let m = parse_mirrors(&s);
        assert_eq!(m.len(), 4);
        assert_eq!(m.iter().filter(|(_, a)| *a).count(), 1);
    }
}
