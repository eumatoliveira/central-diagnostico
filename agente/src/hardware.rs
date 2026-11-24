use crate::Check;

type Capacity = Option<(u64, u64)>;

pub(super) fn inventory() -> Vec<Check> {
    let (memory, disk) = capacities();
    #[allow(unused_mut)]
    let mut checks = vec![
        capacity_check("hardware.memory", "Memória RAM", memory),
        capacity_check("hardware.disk", "Espaço no disco do sistema", disk),
        gpu_check(),
    ];
    #[cfg(target_os = "windows")]
    checks.push(driver_check());
    checks
}

fn gpu_check() -> Check {
    let evidence = gpu_output().and_then(|output| {
        let rows: Vec<_> = output.lines().map(str::trim).filter(|row| !row.is_empty()).take(16).collect();
        (!rows.is_empty()).then(|| format!("Modelo, driver, temperatura, memória usada e total, ventoinha comandada e clock gráfico: {}", rows.join(" | ")))
    });
    Check {
        id: "hardware.nvidia",
        title: "GPU NVIDIA",
        status: if evidence.is_some() {
            "Consultado"
        } else {
            "Indisponível"
        },
        evidence: evidence.unwrap_or_else(|| {
            "O utilitário NVIDIA não retornou sensores neste computador".to_owned()
        }),
        action: "A porcentagem da ventoinha indica o valor comandado e pode diferir da rotação física.",
    }
}

fn gpu_output() -> Option<String> {
    use std::process::Command;
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::{os::windows::process::CommandExt, path::PathBuf};
        let root = std::env::var_os("SystemRoot")?;
        let mut command = Command::new(PathBuf::from(root).join("System32/nvidia-smi.exe"));
        command.creation_flags(0x08000000);
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = Command::new("/usr/bin/nvidia-smi");
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let mut command = Command::new("/usr/bin/false");
    let result = command.args([
        "--query-gpu=name,driver_version,temperature.gpu,memory.used,memory.total,fan.speed,clocks.current.graphics",
        "--format=csv,noheader",
    ]).output().ok()?;
    if !result.status.success() || result.stdout.len() > 16_384 {
        return None;
    }
    String::from_utf8(result.stdout).ok()
}

#[cfg(target_os = "windows")]
fn driver_check() -> Check {
    use std::{os::windows::process::CommandExt, path::PathBuf, process::Command};
    let devices = std::env::var_os("SystemRoot").and_then(|root| {
        let powershell = PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let operation = "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.Encoding]::UTF8; ConvertTo-Json -InputObject @(Get-CimInstance Win32_PnPEntity -Filter 'Present = TRUE AND ConfigManagerErrorCode <> 0' | Select-Object -First 17 Name,ConfigManagerErrorCode) -Compress";
        let output = Command::new(powershell).creation_flags(0x08000000).args(["-NoProfile", "-NonInteractive", "-Command", operation]).output().ok()?;
        if !output.status.success() || output.stdout.len() > 32_768 { return None; }
        serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout).ok()
    });
    let (status, evidence) = match devices {
        None => (
            "Indisponível",
            "O Windows não retornou o estado dos dispositivos".to_owned(),
        ),
        Some(devices) if devices.is_empty() => (
            "Consultado",
            "O Windows não reportou problemas nos dispositivos presentes consultados".to_owned(),
        ),
        Some(devices) => {
            let mut notes = devices
                .iter()
                .take(16)
                .map(|item| {
                    let name = item["Name"].as_str().unwrap_or("Dispositivo sem nome");
                    let code = item["ConfigManagerErrorCode"].as_u64();
                    match code {
                        Some(28) => format!("{name}: código 28, driver não instalado"),
                        Some(22) => format!("{name}: código 22, dispositivo desativado"),
                        Some(code) => {
                            format!("{name}: código {code} no Gerenciador de Dispositivos")
                        }
                        None => format!("{name}: código indisponível"),
                    }
                })
                .collect::<Vec<_>>();
            if devices.len() > 16 {
                notes.push("Há outros dispositivos para revisar".to_owned());
            }
            ("Requer revisão", notes.join(". "))
        }
    };
    Check {
        id: "hardware.drivers",
        title: "Dispositivos e drivers",
        status,
        evidence,
        action: "Consulte o Gerenciador de Dispositivos e procure atualizações no Windows Update ou no fabricante.",
    }
}

fn capacity_check(id: &'static str, title: &'static str, capacity: Capacity) -> Check {
    let valid = capacity.filter(|&(available, total)| total > 0 && available <= total);
    let gib = |bytes: u64| format!("{:.1}", bytes as f64 / 1_073_741_824.0).replace('.', ",");
    Check {
        id,
        title,
        status: if valid.is_some() {
            "Consultado"
        } else {
            "Indisponível"
        },
        evidence: valid.map_or_else(
            || "O sistema não disponibilizou a capacidade".to_owned(),
            |(available, total)| {
                format!("{} GiB disponíveis de {} GiB", gib(available), gib(total))
            },
        ),
        action: "Se o computador estiver lento ou sem espaço, compartilhe este resultado com o suporte.",
    }
}

#[cfg(target_os = "windows")]
fn capacities() -> (Capacity, Capacity) {
    use std::{os::windows::process::CommandExt, path::PathBuf, process::Command};
    let Some(root) = std::env::var_os("SystemRoot") else {
        return (None, None);
    };
    let powershell = PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let operation = concat!(
        "$os=Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue; ",
        "$drive=Get-CimInstance Win32_LogicalDisk -ErrorAction SilentlyContinue | ",
        "Where-Object DeviceID -eq $env:SystemDrive | Select-Object -First 1; ",
        "@{memoryAvailable=$os.FreePhysicalMemory; memoryTotal=$os.TotalVisibleMemorySize; ",
        "diskAvailable=$drive.FreeSpace; diskTotal=$drive.Size} | ConvertTo-Json -Compress"
    );
    let Ok(output) = Command::new(powershell)
        .creation_flags(0x08000000)
        .args(["-NoProfile", "-NonInteractive", "-Command", operation])
        .output()
    else {
        return (None, None);
    };
    if !output.status.success() {
        return (None, None);
    }
    let Ok(data) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return (None, None);
    };
    let measure = |available: &str, total: &str, unit| -> Capacity {
        Some((
            data[available].as_u64()?.checked_mul(unit)?,
            data[total].as_u64()?.checked_mul(unit)?,
        ))
    };
    (
        measure("memoryAvailable", "memoryTotal", 1024),
        measure("diskAvailable", "diskTotal", 1),
    )
}

#[cfg(target_os = "linux")]
fn capacities() -> (Capacity, Capacity) {
    let memory = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            let value = |key: &str| {
                text.lines()
                    .find_map(|line| line.strip_prefix(key))?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()?
                    .checked_mul(1024)
            };
            Some((value("MemAvailable:")?, value("MemTotal:")?))
        });
    let disk = std::process::Command::new("/usr/bin/df")
        .env("LC_ALL", "C")
        .args(["-Pk", "/"])
        .output()
        .ok()
        .and_then(|output| {
            if !output.status.success() {
                return None;
            }
            let text = std::str::from_utf8(&output.stdout).ok()?;
            let row: Vec<&str> = text.lines().nth(1)?.split_whitespace().collect();
            let bytes = |index: usize| row.get(index)?.parse::<u64>().ok()?.checked_mul(1024);
            Some((bytes(3)?, bytes(1)?))
        });
    (memory, disk)
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn capacities() -> (Capacity, Capacity) {
    (None, None)
}
