use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

const LIMIT: u64 = 24 * 1024 * 1024;

#[derive(Serialize)]
pub struct FileReview {
    pub bytes: u64,
    pub sha256: String,
    pub executable_extension: bool,
    pub email_findings: Vec<String>,
}

pub fn inspect(path: &Path) -> Result<FileReview, &'static str> {
    let display = path.to_string_lossy();
    if !path.is_absolute() || display.starts_with("\\\\") || display.starts_with("//") {
        return Err("Informe o caminho completo de um arquivo local.");
    }
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| "Não foi possível localizar o arquivo.")?;
    if !metadata.is_file() || metadata.len() > LIMIT {
        return Err("Escolha um arquivo comum de até 24 MiB.");
    }
    let file = File::open(path).map_err(|_| "Não foi possível abrir o arquivo para leitura.")?;
    let metadata = file
        .metadata()
        .map_err(|_| "Não foi possível consultar o arquivo.")?;
    if !metadata.is_file() || metadata.len() > LIMIT {
        return Err("Escolha um arquivo comum de até 24 MiB.");
    }
    let mut stream = file.take(LIMIT + 1);
    let mut hash = Sha256::new();
    let is_email = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("eml"));
    let mut email_prefix = Vec::new();
    let mut buffer = [0; 32 * 1024];
    let mut bytes = 0;
    loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|_| "A leitura do arquivo falhou.")?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > LIMIT {
            return Err("O arquivo ultrapassou 24 MiB durante a leitura.");
        }
        hash.update(&buffer[..count]);
        if is_email {
            let take = (crate::email::PREVIEW_LIMIT - email_prefix.len()).min(count);
            email_prefix.extend_from_slice(&buffer[..take]);
        }
    }
    if metadata.len() != bytes {
        return Err("O tamanho do arquivo mudou durante a leitura.");
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let executable_extension = matches!(
        extension.as_str(),
        "exe"
            | "msi"
            | "scr"
            | "com"
            | "bat"
            | "cmd"
            | "ps1"
            | "vbs"
            | "js"
            | "hta"
            | "lnk"
            | "iso"
            | "sh"
            | "appimage"
    );
    Ok(FileReview {
        bytes,
        sha256: format!("{:x}", hash.finalize()),
        executable_extension,
        email_findings: if is_email {
            crate::email::review(&email_prefix, bytes > email_prefix.len() as u64)
        } else {
            Vec::new()
        },
    })
}
