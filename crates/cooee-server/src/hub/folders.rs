//! The folders a thread may work in: the roots, browsing them, and new folders.

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{Hub, HubError, Listing, lock, outside, refused};

/// The folders right inside `dir` (hidden ones if `hidden`; never symlinks), sorted.
fn dirs(dir: &Path, hidden: bool) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| hidden || !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

/// `path` (default the home folder), if it is a folder inside the home folder, resolved but spelled from `$HOME`
/// (as `~` shortens it); and `$HOME`.
fn in_home(path: Option<&Path>) -> Result<(PathBuf, PathBuf), HubError> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let real = home.canonicalize().unwrap_or_default();
    let path = path.unwrap_or(&home);
    match path.canonicalize() {
        Ok(p) if p.is_dir() && p.starts_with(&real) && real.has_root() => {
            let rest = p.strip_prefix(&real).unwrap_or(&p);
            let p = if rest.as_os_str().is_empty() {
                home.clone()
            } else {
                home.join(rest)
            };
            Ok((p, home))
        }
        _ => Err(HubError::Refused(format!(
            "{} is not a folder in your home folder",
            path.display()
        ))),
    }
}

impl Hub {
    /// The folders a thread may work in, with their depth: each root (0), then the folders inside it (1) and inside
    /// those (2), by name; never hidden ones or symlinks.
    #[must_use]
    pub fn folders(&self) -> Vec<(PathBuf, u8)> {
        let mut out = Vec::new();
        for root in lock(&self.roots).iter() {
            out.push((root.clone(), 0));
            for a in dirs(root, false) {
                let inside = dirs(&a, false);
                out.push((a, 1));
                out.extend(inside.into_iter().map(|b| (b, 2)));
            }
        }
        out
    }

    /// `folder`, resolved, if it is a root or inside one; none (empty) stays empty: the thread's scratch folder.
    pub(super) fn place(&self, folder: &Path) -> Result<PathBuf, HubError> {
        if folder.as_os_str().is_empty() {
            return Ok(PathBuf::new());
        }
        self.allowed(folder).ok_or_else(|| outside(folder))
    }

    /// `folder`, resolved, if it is a root or inside one.
    pub(super) fn allowed(&self, folder: &Path) -> Option<PathBuf> {
        let folder = folder.canonicalize().ok()?;
        let inside = |root: &PathBuf| root.canonicalize().is_ok_and(|r| folder.starts_with(r));
        lock(&self.roots).iter().any(inside).then_some(folder)
    }

    /// `folder`, if it is a folder that is a root or inside one, spelled from that root as listed.
    fn in_root(&self, folder: &Path) -> Option<PathBuf> {
        let real = folder.canonicalize().ok().filter(|p| p.is_dir())?;
        lock(&self.roots).iter().find_map(|root| {
            let rest = real.strip_prefix(root.canonicalize().ok()?).ok()?;
            Some(if rest.as_os_str().is_empty() {
                root.clone()
            } else {
                root.join(rest)
            })
        })
    }

    /// Settings › Folders and the persona's folder browser: the folders inside `path` (default the home folder; hidden
    /// ones if `hidden`), and the roots. A root or a folder inside one is spelled from the root as listed.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a path outside both the home folder and the roots, or not a folder.
    pub fn browse(&self, path: Option<&Path>, hidden: bool) -> Result<Listing, HubError> {
        let (path, home) = match path.and_then(|p| self.in_root(p)) {
            Some(p) => (p, in_home(None)?.1),
            None => in_home(path)?,
        };
        let dirs = dirs(&path, hidden).into_iter().filter_map(|d| {
            let name = d.file_name()?.to_string_lossy();
            Some(name.into_owned())
        });
        Ok(Listing {
            dirs: dirs.collect(),
            path,
            home,
            roots: lock(&self.roots).clone(),
        })
    }

    /// Adds folder `path` (inside the home folder) to the roots, or removes it (as listed), and saves them in the
    /// config file. Returns the roots.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a path outside the home folder, no config file, or one that can't be written.
    pub fn set_root(&self, path: &Path, add: bool) -> Result<Vec<PathBuf>, HubError> {
        let mut roots = lock(&self.roots);
        let mut new = roots.clone();
        if add {
            let (path, _) = in_home(Some(path))?;
            if !new.contains(&path) {
                new.push(path);
            }
        } else {
            new.retain(|r| r != path);
        }
        let Some(config) = &self.config else {
            return Err(HubError::Refused(
                "there is no config file to save roots in".to_owned(),
            ));
        };
        cooee_core::save_roots(config, &new).map_err(|e| refused(config, &e))?;
        *roots = new;
        Ok(roots.clone())
    }

    /// Makes folder `name` inside `parent` (a root or inside one); returns its path.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a parent outside the roots, a name that isn't one plain folder name, or a folder that
    /// can't be made (it exists).
    pub fn new_folder(&self, parent: &Path, name: &str) -> Result<PathBuf, HubError> {
        let mut parts = Path::new(name).components();
        let plain = matches!(parts.next(), Some(std::path::Component::Normal(_)))
            && parts.next().is_none()
            && !name.contains('/')
            && !name.starts_with('.');
        if !plain {
            return Err(HubError::Refused(format!("{name:?} is not a folder name")));
        }
        if self.allowed(parent).is_none() {
            return Err(outside(parent));
        }
        let path = parent.join(name);
        fs::create_dir(&path).map_err(|e| refused(&path, &e))?;
        Ok(path)
    }
}
