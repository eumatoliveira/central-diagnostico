use crate::backup::{self, BackupAction, BackupResult, BackupSnapshot};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
};

#[derive(Default, Clone, Copy, PartialEq)]
enum Operation {
    #[default]
    List,
    Init,
    Save,
    Check,
    Restore,
}

impl Operation {
    fn label(self) -> &'static str {
        match self {
            Self::List => "Listar cópias",
            Self::Init => "Criar repositório",
            Self::Save => "Fazer backup",
            Self::Check => "Conferir integridade",
            Self::Restore => "Restaurar cópia",
        }
    }
}

#[derive(Default)]
pub(super) struct BackupPanel {
    operation: Operation,
    repository: Option<PathBuf>,
    folder: Option<PathBuf>,
    selected: Option<String>,
    password: String,
    snapshots: Vec<BackupSnapshot>,
    notice: String,
    pending: Option<Receiver<Result<BackupResult, String>>>,
}

impl BackupPanel {
    pub(super) fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(result) => {
                            self.notice = result.message;
                            if self.operation == Operation::List {
                                self.snapshots = result.snapshots;
                            }
                            if let Some(destination) = result.destination {
                                self.notice
                                    .push_str(&format!(" Pasta: {}", destination.display()));
                            }
                        }
                        Err(error) => {
                            self.notice = error;
                        }
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.notice = "A operação foi interrompida.".to_owned();
                }
                Err(TryRecvError::Empty) => {}
            }
        }

        ui.heading("Backup e restauração");
        ui.label(
            "Crie cópias criptografadas em uma pasta local. É necessário ter o restic instalado.",
        );
        ui.add_space(10.0);
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        ui.add_enabled_ui(self.pending.is_none(), |ui| {
            if ui.button("Escolher pasta do repositório…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title("Escolher pasta do repositório")
                    .pick_folder()
            {
                self.repository = Some(path);
                self.snapshots.clear();
                self.selected = None;
                self.password.clear();
                self.notice.clear();
            }
            if let Some(path) = &self.repository {
                ui.label(path.display().to_string());
            }
            let previous = self.operation;
            egui::ComboBox::from_label("Operação")
                .selected_text(previous.label())
                .show_ui(ui, |ui| {
                    for choice in [
                        Operation::List,
                        Operation::Init,
                        Operation::Save,
                        Operation::Check,
                        Operation::Restore,
                    ] {
                        ui.selectable_value(&mut self.operation, choice, choice.label());
                    }
                });
            if previous != self.operation {
                self.folder = None;
            }
            if self.operation == Operation::Restore {
                if self.snapshots.is_empty() {
                    ui.label("Liste as cópias antes de escolher uma para restaurar.");
                }
                for snapshot in &self.snapshots {
                    let selected = self.selected.as_deref() == Some(snapshot.id.as_str());
                    if ui.selectable_label(selected, &snapshot.time).clicked() {
                        self.selected = Some(snapshot.id.clone());
                    }
                }
            }
            if matches!(self.operation, Operation::Save | Operation::Restore) {
                let caption = if self.operation == Operation::Save {
                    "Escolher arquivos de uma pasta…"
                } else {
                    "Escolher pasta onde restaurar…"
                };
                if ui.button(caption).clicked()
                    && let Some(path) = rfd::FileDialog::new().set_title(caption).pick_folder()
                {
                    self.folder = Some(path);
                }
                if let Some(path) = &self.folder {
                    ui.label(path.display().to_string());
                }
            }
            ui.label("Senha do repositório");
            ui.add(
                egui::TextEdit::singleline(&mut self.password)
                    .password(true)
                    .char_limit(1024),
            );
            let action = match self.operation {
                Operation::List => Some(BackupAction::List),
                Operation::Init => Some(BackupAction::Init),
                Operation::Check => Some(BackupAction::Check),
                Operation::Save => self.folder.clone().map(BackupAction::Save),
                Operation::Restore => self
                    .selected
                    .clone()
                    .zip(self.folder.clone())
                    .map(|(snapshot, parent)| BackupAction::Restore { snapshot, parent }),
            };
            if let (Some(repository), Some(action)) = (self.repository.clone(), action)
                && ui
                    .add_enabled(
                        !self.password.is_empty(),
                        egui::Button::new(self.operation.label()).min_size(egui::vec2(170.0, 40.0)),
                    )
                    .clicked()
            {
                if self.operation == Operation::List {
                    self.snapshots.clear();
                    self.selected = None;
                }
                let password = std::mem::take(&mut self.password);
                let (sender, receiver) = mpsc::channel();
                self.pending = Some(receiver);
                self.notice = "Operação de backup em andamento…".to_owned();
                let context = ui.ctx().clone();
                std::thread::spawn(move || {
                    let _ = sender.send(backup::run(&repository, &password, action));
                    context.request_repaint();
                });
            }
        });
    }
}
