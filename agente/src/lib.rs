use serde::Serialize;

pub mod wallet;

mod email;
pub mod files;
mod hardware;
pub mod links;
pub mod scan;

#[derive(Clone, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub title: &'static str,
    pub status: &'static str,
    pub evidence: String,
    pub action: &'static str,
}

fn check(
    id: &'static str,
    title: &'static str,
    enabled: Option<bool>,
    evidence: String,
    action: &'static str,
) -> Check {
    let status = match enabled {
        Some(true) => "Ativo",
        Some(false) => "Requer revisão",
        None => "Indisponível",
    };
    Check {
        id,
        title,
        status,
        evidence,
        action,
    }
}

#[cfg(target_os = "windows")]
fn platform_checks() -> Vec<Check> {
    use std::{env, path::PathBuf, process::Command};
    let firewall = env::var_os("SystemRoot").and_then(|root| {
        let windows = PathBuf::from(root);
        let powershell = windows.join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let output = Command::new(powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                concat!(
                    "$ErrorActionPreference='Stop'; ",
                    "$module=Join-Path ([Environment]::SystemDirectory) ",
                    "'WindowsPowerShell\\v1.0\\Modules\\NetSecurity\\NetSecurity.psd1'; ",
                    "Import-Module -Name $module; ",
                    "$profiles=@(Get-NetFirewallProfile -PolicyStore ActiveStore | ",
                    "ForEach-Object { @{name=[string]$_.Name; enabled=[string]$_.Enabled; inbound=[string]$_.DefaultInboundAction} }); ",
                    "ConvertTo-Json -InputObject $profiles -Compress"
                ),
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        serde_json::from_slice::<serde_json::Value>(&output.stdout).ok()
    });
    ["Domain", "Private", "Public"].into_iter().flat_map(|profile| {
        let settings = firewall.as_ref().and_then(serde_json::Value::as_array)
            .and_then(|profiles| profiles.iter().find(|value| value["name"].as_str() == Some(profile)));
        let enabled = settings.and_then(|value| match value["enabled"].as_str() {
            Some("True") => Some(true), Some("False") => Some(false), _ => None,
        });
        let (id, title) = match profile {
            "Domain" => ("windows.firewall.domain", "Firewall de domínio"),
            "Private" => ("windows.firewall.private", "Firewall de rede privada"),
            _ => ("windows.firewall.public", "Firewall de rede pública"),
        };
        let evidence = match enabled {
            Some(true) => "Habilitado na política efetiva do Windows".to_owned(),
            Some(false) => "Desabilitado na política efetiva do Windows".to_owned(),
            None => "O Windows não retornou uma política determinável para este perfil".to_owned(),
        };
        let (inbound_id, inbound_title) = match profile {
            "Domain" => ("windows.firewall.inbound.domain", "Tráfego de entrada no domínio"),
            "Private" => ("windows.firewall.inbound.private", "Tráfego de entrada na rede privada"),
            _ => ("windows.firewall.inbound.public", "Tráfego de entrada na rede pública"),
        };
        let (blocks, inbound_evidence) = match settings.and_then(|value| value["inbound"].as_str()) {
            Some("Block") => (Some(true), "A política informa bloqueio como ação padrão de entrada"),
            Some("Allow") => (Some(false), "A política informa permissão como ação padrão de entrada"),
            _ => (None, "O Windows não informou uma ação padrão determinável"),
        };
        [
            check(id, title, enabled, evidence, "Revise o perfil em Segurança do Windows ou com a equipe responsável pela política de rede."),
            check(inbound_id, inbound_title, blocks, inbound_evidence.to_owned(), "Revise a política padrão de entrada e as exceções com a equipe de rede."),
        ]
    }).collect()
}

#[cfg(target_os = "linux")]
fn platform_checks() -> Vec<Check> {
    let aslr = std::fs::read_to_string("/proc/sys/kernel/randomize_va_space").ok();
    let value = aslr.as_deref().map(str::trim);
    let enabled = match value {
        Some("2") => Some(true),
        Some("0" | "1") => Some(false),
        _ => None,
    };
    let evidence = match value {
        Some("2") => "O kernel informou ASLR completo",
        Some("1") => "O kernel informou ASLR parcial",
        Some("0") => "O kernel informou ASLR desativado",
        _ => "O kernel não disponibilizou um valor reconhecido para ASLR",
    };
    let mut checks = vec![check(
        "linux.kernel.aslr",
        "Aleatorização de memória",
        enabled,
        evidence.to_owned(),
        "Revise a política de ASLR com a administração da máquina. Em WSL, este resultado refere-se ao kernel Linux.",
    )];
    for (path, id, title, action) in [
        (
            "/proc/sys/kernel/dmesg_restrict",
            "linux.kernel.dmesg_restrict",
            "Acesso aos registros do kernel",
            "Revise as permissões de leitura dos registros do kernel com a administração da máquina.",
        ),
        (
            "/proc/sys/fs/protected_symlinks",
            "linux.fs.protected_symlinks",
            "Proteção de links simbólicos",
            "Revise a proteção de links simbólicos em diretórios compartilhados com a administração da máquina.",
        ),
        (
            "/proc/sys/fs/protected_hardlinks",
            "linux.fs.protected_hardlinks",
            "Proteção de links físicos",
            "Revise as restrições para criar links físicos com a administração da máquina.",
        ),
    ] {
        let value = std::fs::read_to_string(path).ok();
        let (state, evidence) = match value.as_deref().map(str::trim) {
            Some("1") => (Some(true), "O kernel informou a restrição habilitada"),
            Some("0") => (Some(false), "O kernel informou a restrição desabilitada"),
            _ => (None, "O kernel não disponibilizou um valor reconhecido"),
        };
        checks.push(check(id, title, state, evidence.to_owned(), action));
    }
    checks
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn platform_checks() -> Vec<Check> {
    Vec::new()
}

pub fn audit() -> Vec<Check> {
    platform_checks()
}

/// The desktop includes the Antivirus status supplied by the operating system.
pub fn audit_desktop() -> Vec<Check> {
    let mut checks = audit();
    if cfg!(target_os = "windows") {
        checks.extend(defender_checks());
    }
    checks.extend(hardware::inventory());
    checks
}

#[cfg(not(target_os = "windows"))]
fn defender_checks() -> Vec<Check> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn defender_checks() -> Vec<Check> {
    use std::{os::windows::process::CommandExt, path::PathBuf, process::Command};
    let state = std::env::var_os("SystemRoot").and_then(|root| {
        let powershell = PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let output = Command::new(powershell)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .args([
                "-NoProfile", "-NonInteractive", "-Command",
                "$ErrorActionPreference='Stop'; Get-MpComputerStatus | Select-Object AMRunningMode, AntivirusEnabled, RealTimeProtectionEnabled, DefenderSignaturesOutOfDate, IsTamperProtected | ConvertTo-Json -Compress",
            ])
            .output().ok()?;
        if !output.status.success() { return None; }
        serde_json::from_slice::<serde_json::Value>(&output.stdout).ok()
    });
    let value = |key| {
        state
            .as_ref()
            .and_then(|record| record.get(key))
            .and_then(serde_json::Value::as_bool)
    };
    let passive = state
        .as_ref()
        .and_then(|record| record.get("AMRunningMode"))
        .and_then(serde_json::Value::as_str)
        == Some("Passive Mode");
    let action = "Abra Segurança do Windows e confira qual antivírus protege este computador.";
    let fields = [
        (
            "windows.defender.realtime",
            "Proteção em tempo real do Defender",
            value("RealTimeProtectionEnabled"),
            "Proteção em tempo real habilitada",
            if passive {
                "Proteção em tempo real desativada. O Microsoft Defender está em modo passivo."
            } else {
                "Proteção em tempo real desativada"
            },
        ),
        (
            "windows.defender.antivirus",
            "Antivírus Defender",
            value("AntivirusEnabled"),
            "O componente antivírus está habilitado",
            "O componente antivírus está desabilitado",
        ),
        (
            "windows.defender.signatures",
            "Assinaturas Defender",
            value("DefenderSignaturesOutOfDate").map(|outdated| !outdated),
            "O Defender indica assinaturas atualizadas",
            "O Defender indica assinaturas desatualizadas",
        ),
        (
            "windows.defender.tamper",
            "Proteção contra adulteração",
            value("IsTamperProtected"),
            "Proteção contra adulteração habilitada",
            "Proteção contra adulteração desabilitada",
        ),
    ];
    fields
        .into_iter()
        .map(|(id, title, flag, active, inactive)| {
            let evidence = match flag {
                Some(true) => active,
                Some(false) => inactive,
                None => "O Windows não retornou este indicador",
            };
            check(id, title, flag, evidence.to_owned(), action)
        })
        .collect()
}
