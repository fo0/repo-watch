//! Where the local tools live: absolute, real `.exe` files from the Windows install locations
//! and absolute `PATH` entries, never from the working directory or a Git work tree.
#[cfg(any(windows, test))]
use std::path::Path;
use std::path::PathBuf;

#[derive(Default)]
pub struct Programs {
    pub gh: Option<PathBuf>,
    pub gcm: Option<PathBuf>,
    pub git: Option<PathBuf>,
}
impl Programs {
    pub fn installed() -> Self {
        #[cfg(windows)]
        {
            windows_programs(
                |name| std::env::var_os(name),
                &std::env::current_dir().unwrap_or_default(),
            )
        }
        #[cfg(not(windows))]
        Self::default()
    }
}

#[cfg(any(windows, test))]
pub(super) fn windows_programs(
    get: impl Fn(&str) -> Option<std::ffi::OsString>,
    cwd: &Path,
) -> Programs {
    let roots: Vec<_> = ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(|name| get(name))
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .collect();
    let mut gh: Vec<_> = roots
        .iter()
        .map(|root| root.join("GitHub CLI/gh.exe"))
        .collect();
    let mut git: Vec<_> = roots
        .iter()
        .map(|root| root.join("Git/cmd/git.exe"))
        .collect();
    let mut gcm: Vec<_> = roots
        .iter()
        .flat_map(|root| {
            [
                root.join("Git Credential Manager/git-credential-manager.exe"),
                root.join("Git/mingw64/bin/git-credential-manager.exe"),
            ]
        })
        .collect();
    if let Some(local) = get("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        gh.extend([
            local.join("Programs/GitHub CLI/gh.exe"),
            local.join("Microsoft/WinGet/Links/gh.exe"),
        ]);
        git.push(local.join("Programs/Git/cmd/git.exe"));
        gcm.push(local.join("Programs/Git/mingw64/bin/git-credential-manager.exe"));
    }
    let mut scoop_roots = Vec::new();
    if let Some(home) = get("USERPROFILE")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        scoop_roots.push(home.join("scoop"));
    }
    if let Some(data) = get("ProgramData")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        scoop_roots.push(data.join("scoop"));
    }
    for name in ["SCOOP", "SCOOP_GLOBAL"] {
        if let Some(root) = get(name).map(PathBuf::from).filter(|p| p.is_absolute()) {
            scoop_roots.push(root);
        }
    }
    gh.extend(
        scoop_roots
            .iter()
            .map(|root| root.join("apps/gh/current/bin/gh.exe")),
    );
    if let Some(root) = get("ChocolateyInstall")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        gh.extend([
            root.join("bin/gh.exe"),
            root.join("lib/gh/tools/bin/gh.exe"),
        ]);
    }
    // Resolve only absolute PATH entries to real .exe files. Never execute a project-local
    // binary, the working directory's gh.exe, a shell alias, or a .cmd wrapper.
    if let Some(path) = get("PATH") {
        for directory in std::env::split_paths(&path).filter(|p| p.is_absolute()) {
            gh.push(directory.join("gh.exe"));
            git.push(directory.join("git.exe"));
            gcm.push(directory.join("git-credential-manager.exe"));
        }
    }
    let first = |paths: Vec<PathBuf>| {
        paths
            .into_iter()
            .find_map(|path| safe_executable(path, cwd))
    };
    Programs {
        gh: first(gh),
        git: first(git),
        gcm: first(gcm),
    }
}

#[cfg(any(windows, test))]
pub(super) fn safe_executable(path: PathBuf, cwd: &Path) -> Option<PathBuf> {
    if !path.is_absolute()
        || !path.is_file()
        || !path.extension()?.to_str()?.eq_ignore_ascii_case("exe")
    {
        return None;
    }
    let real = path.canonicalize().ok()?;
    let working = cwd.canonicalize().ok();
    if path
        .parent()
        .and_then(|parent| parent.canonicalize().ok())
        .as_ref()
        == working.as_ref()
        || real.parent() == working.as_deref()
        || real.ancestors().any(|parent| {
            let git = parent.join(".git");
            git.is_file() || git.join("HEAD").is_file()
        })
    {
        return None;
    }
    Some(real)
}
