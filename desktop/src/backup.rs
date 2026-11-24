use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LIST_BYTES: u64 = 2 * 1024 * 1024;

pub enum BackupAction {
    Init,
    Save(PathBuf),
    List,
    Check,
    Restore { snapshot: String, parent: PathBuf },
}

pub struct BackupSnapshot {
    pub id: String,
    pub time: String,
}

pub struct BackupResult {
    pub message: String,
    pub snapshots: Vec<BackupSnapshot>,
    pub destination: Option<PathBuf>,
}

fn local_path(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        ) && path.is_absolute()
    }
    #[cfg(not(windows))]
    {
        path.is_absolute()
    }
}

fn directory(path: &Path) -> Result<PathBuf, String> {
    if !local_path(path) {
        return Err("Informe o caminho absoluto de uma pasta local.".into());
    }
    let resolved =
        fs::canonicalize(path).map_err(|_| "Não foi possível abrir a pasta informada.")?;
    if !local_path(&resolved) || !resolved.is_dir() {
        return Err("Escolha uma pasta local existente.".into());
    }
    #[cfg(windows)]
    {
        use std::{
            ffi::OsString,
            os::windows::ffi::{OsStrExt, OsStringExt},
        };
        let wide = resolved.as_os_str().encode_wide().collect::<Vec<_>>();
        if let Some(normal) = wide.strip_prefix(&[92, 92, 63, 92]) {
            return Ok(PathBuf::from(OsString::from_wide(normal)));
        }
    }
    Ok(resolved)
}

fn overlaps(parent: &Path, child: &Path) -> bool {
    #[cfg(windows)]
    {
        child.starts_with(parent)
            || child.components().count() >= parent.components().count()
                && child.components().zip(parent.components()).all(|(a, b)| {
                    a.as_os_str().to_string_lossy().to_lowercase()
                        == b.as_os_str().to_string_lossy().to_lowercase()
                })
    }
    #[cfg(not(windows))]
    {
        child.starts_with(parent)
    }
}

fn executable() -> Result<PathBuf, String> {
    let name = if cfg!(windows) {
        "restic.exe"
    } else {
        "restic"
    };
    env::var_os("PATH")
        .into_iter()
        .flat_map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .filter(|path| local_path(path))
        .filter_map(|path| fs::canonicalize(path.join(name)).ok())
        .find(|path| local_path(path) && path.is_file())
        .ok_or_else(|| "Instale o restic pelo canal oficial e adicione sua pasta ao PATH.".into())
}

fn run_command(mut command: Command, args: &[&str], capture: bool) -> Result<Vec<u8>, String> {
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(if capture {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command
        .spawn()
        .map_err(|_| "Não foi possível iniciar o restic.")?;
    let mut output = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let read = stdout.take(MAX_LIST_BYTES + 1).read_to_end(&mut output);
        if read.is_err() || output.len() as u64 > MAX_LIST_BYTES {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Não foi possível ler a lista de cópias no limite do aplicativo.".into());
        }
    }
    let status = child
        .wait()
        .map_err(|_| "Não foi possível obter o resultado do restic.")?;
    if !status.success() {
        return Err(format!(
            "O restic não concluiu a operação ({status}). Confira a senha, as permissões e o estado do repositório."
        ));
    }
    Ok(output)
}

fn is_snapshot_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_snapshots(bytes: &[u8]) -> Result<Vec<BackupSnapshot>, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "O restic retornou uma lista inválida.")?;
    if value.is_null() {
        return Ok(Vec::new());
    }
    let rows = value
        .as_array()
        .ok_or("O restic retornou uma lista inválida.")?;
    rows.iter()
        .rev()
        .take(50)
        .map(|row| {
            let id = row["id"]
                .as_str()
                .filter(|value| is_snapshot_id(value))
                .ok_or("Uma cópia não tem um identificador válido.")?;
            let time = row["time"]
                .as_str()
                .filter(|value| !value.is_empty())
                .ok_or("Uma cópia não tem uma data válida.")?;
            Ok(BackupSnapshot {
                id: id.into(),
                time: time.into(),
            })
        })
        .collect()
}

fn create_restore_destination(parent: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "O relógio do sistema não permitiu nomear a restauração.")?
        .as_nanos();
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = builder;
        builder.mode(0o700);
        builder
    };
    for index in 0..16 {
        let destination = parent.join(format!("restauracao-{timestamp}-{index}"));
        match builder.create(&destination) {
            Ok(()) => return Ok(destination),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("Não foi possível criar a pasta de restauração.".into()),
        }
    }
    Err("Não foi possível reservar uma pasta nova para restauração.".into())
}

pub fn run(
    repository: &Path,
    password: &str,
    action: BackupAction,
) -> Result<BackupResult, String> {
    if password.is_empty() || password.len() > 16384 || password.contains('\0') {
        return Err("Informe uma senha válida para o repositório.".into());
    }
    let repository = directory(repository)?;
    let mut command = Command::new(executable()?);
    for (key, _) in env::vars_os() {
        let name = key.to_string_lossy().to_ascii_uppercase();
        if name.starts_with("RESTIC_") || name.starts_with("AWS_") {
            command.env_remove(key);
        }
    }
    command
        .env("RESTIC_PASSWORD", password)
        .arg("--repo")
        .arg(&repository);
    let mut result = BackupResult {
        message: String::new(),
        snapshots: Vec::new(),
        destination: None,
    };
    let message = match action {
        BackupAction::Init => {
            if fs::read_dir(&repository)
                .map_err(|_| "Não foi possível consultar o repositório.")?
                .next()
                .is_some()
            {
                return Err("Escolha uma pasta de repositório vazia.".into());
            }
            run_command(command, &["init"], false)?;
            "Repositório criado."
        }
        BackupAction::Save(source) => {
            let source = directory(&source)?;
            if overlaps(&repository, &source) || overlaps(&source, &repository) {
                return Err(
                    "A origem e o repositório precisam estar em árvores de pastas separadas."
                        .into(),
                );
            }
            command.current_dir(source).arg("backup");
            if !cfg!(windows) {
                command.arg("--one-file-system");
            }
            run_command(command, &["--", "."], false)?;
            "Backup concluído."
        }
        BackupAction::List => {
            result.snapshots =
                parse_snapshots(&run_command(command, &["snapshots", "--json"], true)?)?;
            "Lista de cópias atualizada."
        }
        BackupAction::Check => {
            run_command(command, &["check", "--read-data"], false)?;
            "Dados do repositório verificados."
        }
        BackupAction::Restore { snapshot, parent } => {
            if !is_snapshot_id(&snapshot) {
                return Err("Selecione o identificador completo de uma cópia.".into());
            }
            let parent = directory(&parent)?;
            if overlaps(&repository, &parent) {
                return Err("A restauração precisa ficar fora do repositório.".into());
            }
            let destination = create_restore_destination(&parent)?;
            command
                .args(["restore", &snapshot, "--target"])
                .arg(&destination);
            run_command(command, &["--verify"], false).map_err(|error| {
                format!(
                    "{error} A pasta {} pode conter arquivos parciais.",
                    destination.display()
                )
            })?;
            result.destination = Some(destination);
            "Restauração verificada pelo restic."
        }
    };
    result.message = message.into();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_list_accepts_empty_and_rejects_partial_ids() {
        assert!(parse_snapshots(b"null").unwrap().is_empty());
        assert!(parse_snapshots(b"[]").unwrap().is_empty());
        assert!(parse_snapshots(br#"[{"id":"1234","time":"2026-11-23"}]"#).is_err());
        let full_id = "f".repeat(64);
        let fixture = serde_json::json!([{ "id": full_id, "time": "2026-11-23T18:00:00Z" }]);
        let parsed = parse_snapshots(fixture.to_string().as_bytes()).unwrap();
        assert_eq!(parsed[0].id, full_id);
    }

    #[test]
    fn remote_backend_is_not_a_local_path() {
        assert!(!local_path(Path::new("s3:https://example.org/bucket")));
        assert!(!local_path(Path::new("relative")));
        #[cfg(windows)]
        assert!(!local_path(Path::new(r"\\server\share")));
    }

    #[test]
    #[ignore = "requer restic no PATH"]
    fn real_restic_round_trip() {
        let temp = fs::canonicalize(env::temp_dir()).unwrap();
        let fixture = create_restore_destination(&temp).unwrap();
        let source = fixture.join("origem sintética");
        let repository = fixture.join("repositorio");
        let restore = fixture.join("destino");
        for path in [&source, &repository, &restore] {
            fs::create_dir(path).unwrap();
        }
        fs::write(source.join("exemplo.txt"), b"backup synthetic fixture\n").unwrap();
        const PASSWORD: &str = "senha sintética de teste";
        run(&repository, PASSWORD, BackupAction::Init).unwrap();
        run(&repository, PASSWORD, BackupAction::Save(source.clone())).unwrap();
        run(&repository, PASSWORD, BackupAction::Check).unwrap();
        let listed = run(&repository, PASSWORD, BackupAction::List).unwrap();
        assert_eq!(listed.snapshots.len(), 1);
        let restored = run(
            &repository,
            PASSWORD,
            BackupAction::Restore {
                snapshot: listed.snapshots[0].id.clone(),
                parent: restore,
            },
        )
        .unwrap();
        assert_eq!(
            fs::read(restored.destination.unwrap().join("exemplo.txt")).unwrap(),
            fs::read(source.join("exemplo.txt")).unwrap()
        );
        assert!(run(&repository, PASSWORD, BackupAction::Save(fixture.clone())).is_err());
        let resolved = fs::canonicalize(fixture).unwrap();
        assert_eq!(resolved.parent(), Some(temp.as_path()));
        fs::remove_dir_all(resolved).unwrap();
    }
}
