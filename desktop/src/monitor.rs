use eframe::egui;
use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    time::Duration,
};

const INTERVAL: Duration = Duration::from_secs(30);

#[derive(Default)]
pub(super) struct Monitor {
    worker: Option<(Sender<()>, Receiver<Reading>)>,
    latest: Option<Reading>,
}

struct Reading {
    sequence: u64,
    memory: Option<(u64, u64)>,
    disk: Option<(u64, u64)>,
}

impl Monitor {
    fn start(&mut self, context: egui::Context) {
        let (stop, cancellation) = mpsc::channel();
        let (sender, updates) = mpsc::sync_channel(1);
        self.latest = None;
        self.worker = Some((stop, updates));
        std::thread::spawn(move || {
            for sequence in 1.. {
                let reading = read_resources(sequence);
                match sender.try_send(reading) {
                    Ok(()) => context.request_repaint(),
                    Err(mpsc::TrySendError::Full(_)) => {}
                    Err(mpsc::TrySendError::Disconnected(_)) => break,
                }
                if !matches!(
                    cancellation.recv_timeout(INTERVAL),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    break;
                }
            }
        });
    }

    pub(super) fn update(&mut self) {
        let Some((_, reader)) = &self.worker else {
            return;
        };
        match reader.try_recv() {
            Ok(reading) => self.latest = Some(reading),
            Err(TryRecvError::Disconnected) => self.worker = None,
            Err(TryRecvError::Empty) => {}
        }
    }

    pub(super) fn show(&mut self, ui: &mut egui::Ui) {
        ui.label("Acompanha memória e disco a cada 30 segundos, inclusive com a janela minimizada. Fechar a Central encerra o monitoramento.");
        if self.worker.is_some() {
            if ui.button("Pausar monitoramento").clicked() {
                self.worker = None;
            }
        } else if ui.button("Ativar monitoramento").clicked() {
            self.start(ui.ctx().clone());
        }
        if let Some(reading) = &self.latest {
            ui.label(format!("Última leitura: {}", reading.sequence));
            for (name, value, limit) in [
                ("Memória RAM", reading.memory, 10),
                ("Disco do sistema", reading.disk, 5),
            ] {
                if let Some((available, total)) =
                    value.filter(|&(available, total)| total > 0 && available <= total)
                {
                    let gib = available as f64 / 1_073_741_824.0;
                    let low = (available as u128) * 100 < (total as u128) * limit;
                    let status = if low {
                        "capacidade baixa"
                    } else {
                        "capacidade disponível"
                    };
                    ui.label(format!("{name}: {gib:.1} GiB livres · {status}"));
                    let free = available as f64 / total as f64;
                    let color = if low {
                        egui::Color32::from_rgb(162, 87, 11)
                    } else {
                        egui::Color32::from_rgb(17, 100, 86)
                    };
                    ui.add(
                        egui::ProgressBar::new(free as f32)
                            .fill(color)
                            .text(format!("{:.1}% livres", free * 100.0)),
                    )
                    .on_hover_text(format!(
                        "{name}. O alerta de capacidade baixa aparece abaixo de {limit}% livres."
                    ));
                } else {
                    ui.label(format!("{name}: leitura indisponível"));
                }
            }
        } else if self.worker.is_some() {
            ui.label("Aguardando a primeira leitura.");
        }
    }
}

fn read_resources(sequence: u64) -> Reading {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Storage::FileSystem::GetDiskFreeSpaceExW,
        System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
    };
    let mut memory = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // Windows writes only to the supplied structure for this synchronous call.
    let memory = (unsafe { GlobalMemoryStatusEx(&mut memory) } != 0)
        .then_some((memory.ullAvailPhys, memory.ullTotalPhys));
    let disk = std::env::var_os("SystemDrive").and_then(|drive| {
        let mut root: Vec<u16> = drive.encode_wide().collect();
        if root.len() != 2 || root[1] != 58 {
            return None;
        }
        root.extend([92, 0]);
        let mut available = 0;
        let mut total = 0;
        // The path is NUL-terminated and output pointers remain valid during the call.
        (unsafe {
            GetDiskFreeSpaceExW(
                root.as_ptr(),
                &mut available,
                &mut total,
                std::ptr::null_mut(),
            )
        } != 0)
            .then_some((available, total))
    });
    Reading {
        sequence,
        memory,
        disk,
    }
}
