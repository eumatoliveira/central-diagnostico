#[cfg(target_os = "windows")]
pub fn defender_quick_scan() -> Result<&'static str, &'static str> {
    defender_scan(None)
}

#[cfg(target_os = "windows")]
pub fn defender_file_scan(path: &std::path::Path) -> Result<&'static str, &'static str> {
    let display = path.to_string_lossy();
    if !path.is_absolute()
        || display.starts_with("\\\\")
        || display.starts_with("//")
        || display.contains(['*', '?', '[', ']'])
    {
        return Err("Escolha um arquivo com caminho completo e sem caracteres de busca.");
    }
    if !std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return Err("O arquivo selecionado está indisponível ou não é um arquivo comum.");
    }
    defender_scan(Some(path))
}

#[cfg(target_os = "windows")]
fn defender_scan(path: Option<&std::path::Path>) -> Result<&'static str, &'static str> {
    use std::{
        os::windows::process::CommandExt,
        path::PathBuf,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    let system =
        std::env::var_os("SystemRoot").ok_or("O Windows não informou a pasta do sistema.")?;
    let powershell = PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let preflight = concat!(
        "$ErrorActionPreference='Stop'; ",
        "$state=Get-MpComputerStatus; ",
        "if ($state.AMRunningMode -like '*Passive*') { exit 20 }; ",
        "if (-not ($state.AMServiceEnabled -and $state.AntivirusEnabled)) { exit 21 }; "
    );
    let mut command = Command::new(powershell);
    let operation = if let Some(path) = path {
        command.env("CENTRAL_DEFENDER_SCAN_FILE", path);
        format!(
            "{preflight}Start-MpScan -ScanType CustomScan -ScanPath $env:CENTRAL_DEFENDER_SCAN_FILE; exit 0"
        )
    } else {
        format!("{preflight}Start-MpScan -ScanType QuickScan; exit 0")
    };
    let mut child = command
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .args(["-NoProfile", "-NonInteractive", "-Command", &operation])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Não foi possível iniciar a integração com o Defender.")?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(exit)) => {
                return match exit.code() {
                    Some(0) => Ok(
                        "O Defender aceitou a solicitação. Confira as detecções no Histórico de proteção do Windows.",
                    ),
                    Some(20) => Err(
                        "O Defender está em modo passivo. Use o antivírus ativo indicado na Segurança do Windows.",
                    ),
                    Some(21) => Err("O Defender não está disponível para iniciar a varredura."),
                    _ => Err(
                        "O Defender não confirmou a solicitação. Confira a Segurança do Windows e as permissões da conta.",
                    ),
                };
            }
            Ok(None) if start.elapsed() < Duration::from_secs(900) => {
                std::thread::sleep(Duration::from_millis(200))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "A solicitação ao Defender não terminou no tempo esperado. Confira o estado no Windows.",
                );
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn defender_quick_scan() -> Result<&'static str, &'static str> {
    Err("A varredura do Microsoft Defender está disponível no Windows.")
}

#[cfg(not(target_os = "windows"))]
pub fn defender_file_scan(_path: &std::path::Path) -> Result<&'static str, &'static str> {
    defender_quick_scan()
}

pub fn scan_file(path: &std::path::Path) -> Result<&'static str, &'static str> {
    #[cfg(target_os = "linux")]
    {
        clamav_file_scan(path)
    }
    #[cfg(not(target_os = "linux"))]
    {
        defender_file_scan(path)
    }
}

#[cfg(target_os = "linux")]
fn clamav_file_scan(path: &std::path::Path) -> Result<&'static str, &'static str> {
    use std::{
        fs::File,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    const LIMIT: u64 = 24 * 1024 * 1024;
    if !path.is_absolute()
        || !std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() <= LIMIT)
    {
        return Err("Escolha um arquivo comum de até 24 MiB com caminho completo.");
    }
    let input = File::open(path).map_err(|_| "Não foi possível ler o arquivo selecionado.")?;
    if !input
        .metadata()
        .is_ok_and(|meta| meta.is_file() && meta.len() <= LIMIT)
    {
        return Err("O arquivo selecionado mudou ou excede o limite de 24 MiB.");
    }
    let mut child = Command::new("/usr/bin/clamscan")
        .args(["--no-summary", "-"])
        .stdin(Stdio::from(input))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => {
                "ClamAV não encontrado. Instale o clamscan e configure as assinaturas locais."
            }
            _ => "Não foi possível iniciar o ClamAV.",
        })?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(exit)) => {
                return match exit.code() {
                    Some(0) => Ok("ClamAV não encontrou uma ameaça nesta verificação."),
                    Some(1) => Ok(
                        "ClamAV detectou uma possível ameaça. Evite abrir o arquivo e consulte o suporte.",
                    ),
                    _ => Err(
                        "ClamAV não concluiu a verificação. Confira as assinaturas e as permissões locais.",
                    ),
                };
            }
            Ok(None) if started.elapsed() < Duration::from_secs(60) => {
                std::thread::sleep(Duration::from_millis(100))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("O ClamAV não respondeu no tempo esperado.");
            }
        }
    }
}
