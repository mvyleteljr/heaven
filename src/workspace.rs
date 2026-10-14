use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Read,
    os::unix::process::ExitStatusExt,
    path::{Component, Path},
    process::{Command, Stdio},
};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_ENTRIES: usize = 128;
const OUTPUT_LIMIT: usize = 32 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub files: BTreeMap<String, Vec<u8>>,
    pub directories: Vec<String>,
}

pub fn path(value: &str) -> Result<String> {
    let relative = value.strip_prefix("/workspace/").unwrap_or(value);
    ensure!(
        !relative.is_empty() && relative.len() <= 512 && !relative.contains('\0'),
        "Invalid workspace path"
    );
    ensure!(
        Path::new(relative)
            .components()
            .all(|p| matches!(p, Component::Normal(_) | Component::CurDir)),
        "Paths must stay within /workspace; traversal is not allowed"
    );
    let normalized = Path::new(relative)
        .components()
        .filter_map(|part| match part {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    ensure!(!normalized.is_empty(), "Path must name a file or directory");
    Ok(normalized)
}

/// Exercise the same isolation path used by shell tools and hosted handlers.
pub fn doctor() -> Result<Value> {
    ensure!(
        cfg!(target_os = "linux"),
        "Shell isolation requires Linux. On macOS run scripts/dev up (Docker Desktop). Native host execution is never used."
    );
    let result = Workspace::default().exec(
        "test ! -e /etc/passwd && test -z \"$OPENROUTER_API_KEY\" && test \"$(pwd)\" = /workspace && printf frontier-isolated",
    )?;
    ensure!(
        result["exit_status"] == 0 && result["stdout"] == "frontier-isolated",
        "Sandbox self-check failed: {}. Install bubblewrap, busybox-static, util-linux and coreutils and enable unprivileged user namespaces; use scripts/dev doctor for the Docker setup.",
        result["stderr"]
    );
    Ok(json!({"ok":true,"sandbox":"bubblewrap","shell":"static BusyBox"}))
}

impl Workspace {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.files.len() + self.directories.len() <= MAX_ENTRIES,
            "Workspace exceeds 128 entries; changes not committed"
        );
        ensure!(
            self.files.values().map(Vec::len).sum::<usize>() <= MAX_BYTES,
            "Workspace exceeds 8 MiB; changes not committed"
        );
        Ok(())
    }
    pub fn write(&mut self, name: &str, content: &str) -> Result<Value> {
        let name = path(name)?;
        ensure!(content.len() <= 1048576, "File exceeds 1 MiB");
        ensure!(!self.directories.contains(&name), "Path is a directory");
        let mut next = self.clone();
        for parent in Path::new(&name)
            .ancestors()
            .skip(1)
            .filter(|p| !p.as_os_str().is_empty())
        {
            let parent = parent.to_str().unwrap().to_owned();
            ensure!(!next.files.contains_key(&parent), "Parent path is a file");
            if !next.directories.contains(&parent) {
                next.directories.push(parent);
            }
        }
        next.files.insert(name.clone(), content.as_bytes().to_vec());
        next.directories.sort();
        next.validate()?;
        *self = next;
        Ok(json!({"path":name,"bytes_written":content.len()}))
    }
    pub fn read(&self, name: &str, offset: usize, limit: usize) -> Result<Value> {
        let name = path(name)?;
        let bytes = self
            .files
            .get(&name)
            .ok_or_else(|| anyhow::anyhow!("File not found"))?;
        let start = offset.min(bytes.len());
        let end = start
            .saturating_add(limit.min(OUTPUT_LIMIT))
            .min(bytes.len());
        Ok(
            json!({"content":String::from_utf8_lossy(&bytes[start..end]),"offset":start,"next_offset":end,"size_bytes":bytes.len(),"truncated":end<bytes.len()}),
        )
    }
    pub fn exec(&mut self, command: &str) -> Result<Value> {
        ensure!(command.len() <= 16000, "Command too long");
        let root = tempfile::tempdir()?;
        for dir in &self.directories {
            std::fs::create_dir_all(root.path().join(path(dir)?))?;
        }
        for (name, bytes) in &self.files {
            let dest = root.path().join(path(name)?);
            std::fs::create_dir_all(dest.parent().unwrap())?;
            std::fs::write(dest, bytes)?;
        }
        // Fail closed if namespace isolation is unavailable. Only this disposable
        // workspace and a static BusyBox binary are mounted into the actor root.
        let mut cmd = Command::new("/usr/bin/timeout");
        cmd.args([
            "--signal=KILL",
            "3",
            "/usr/bin/prlimit",
            "--as=134217728",
            "--cpu=2",
            "--fsize=1048576",
            "--nofile=64",
            "--nproc=256",
            "--",
            "/usr/bin/bwrap",
            "--unshare-all",
            "--die-with-parent",
            "--new-session",
            "--cap-drop",
            "ALL",
            "--clearenv",
            "--ro-bind",
            "/usr/bin/busybox",
            "/bin/busybox",
            "--bind",
        ])
        .arg(root.path())
        .arg("/workspace")
        .args([
            // No procfs is needed by the supported applets. An empty directory
            // also works inside Docker's masked-proc environment, where mounting
            // another procfs from a nested user namespace is denied.
            "--dir",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--chdir",
            "/workspace",
            "--setenv",
            "PATH",
            "/bin",
            "--setenv",
            "HOME",
            "/workspace",
            "--setenv",
            "LANG",
            "C.UTF-8",
        ]);
        for applet in [
            "sh",
            "cat",
            "ls",
            "mkdir",
            "rmdir",
            "rm",
            "cp",
            "mv",
            "printf",
            "echo",
            "grep",
            "sed",
            "awk",
            "head",
            "tail",
            "wc",
            "sort",
            "uniq",
            "cut",
            "tr",
            "find",
            "date",
            "sleep",
            "env",
            "pwd",
            "touch",
            "test",
            "true",
            "false",
            "base64",
            "sha256sum",
        ] {
            cmd.args(["--symlink", "busybox", &format!("/bin/{applet}")]);
        }
        let mut child = cmd
            .args(["/bin/sh", "-c", command])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        fn capture(mut stream: impl Read) -> (Vec<u8>, bool) {
            let mut kept = Vec::new();
            let mut total = 0;
            let mut chunk = [0; 4096];
            while let Ok(n) = stream.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                total += n;
                let count = n.min(OUTPUT_LIMIT - kept.len());
                kept.extend_from_slice(&chunk[..count]);
            }
            (kept, total > OUTPUT_LIMIT)
        }
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let out = std::thread::spawn(move || capture(stdout));
        let err = std::thread::spawn(move || capture(stderr));
        let status = child.wait()?;
        let (out, out_cut) = out.join().unwrap();
        let (err, err_cut) = err.join().unwrap();
        let mut snapshot = Workspace::default();
        snapshot.collect(root.path(), root.path())?;
        snapshot.directories.sort();
        snapshot.validate()?;
        *self = snapshot;
        Ok(
            json!({"stdout":String::from_utf8_lossy(&out),"stderr":String::from_utf8_lossy(&err),"exit_status":status.code().unwrap_or_else(||128+status.signal().unwrap_or(9)),"timed_out":status.signal()==Some(9),"truncated":out_cut||err_cut}),
        )
    }
    fn collect(&mut self, root: &Path, dir: &Path) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            ensure!(
                kind.is_file() || kind.is_dir(),
                "Only regular files and directories persist; changes not committed"
            );
            let full = entry.path();
            let name = full
                .strip_prefix(root)?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Paths must be UTF-8"))?
                .to_owned();
            path(&name)?;
            if kind.is_dir() {
                self.directories.push(name);
                self.validate()?;
                self.collect(root, &full)?;
            } else {
                ensure!(
                    entry.metadata()?.len() <= 1048576,
                    "File exceeds 1 MiB; changes not committed"
                );
                self.files.insert(name, std::fs::read(full)?);
                self.validate()?;
            }
        }
        Ok(())
    }
}
