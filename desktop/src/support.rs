use eframe::egui;
use std::{
    path::{Component, Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
};

#[derive(Default)]
pub(super) struct SupportPanel {
    folder: Option<PathBuf>,
    expected: String,
    result: Vec<String>,
    pending: Option<Receiver<Vec<String>>>,
}

fn inspect(folder: &Path, expected: &str) -> Vec<String> {
    let mut result = Vec::new();
    let name = expected.trim();
    if !name.is_empty() {
        if name.contains(['/', '\\', ':'])
            || !matches!(
                Path::new(name).components().collect::<Vec<_>>().as_slice(),
                [Component::Normal(_)]
            )
        {
            return vec!["Informe só o nome do arquivo esperado, como exemplo.dll.".to_owned()];
        }
        let text = match std::fs::symlink_metadata(folder.join(name)) {
            Ok(meta) if meta.is_file() => "Arquivo encontrado na pasta selecionada.",
            Ok(_) => "O nome existe, mas não corresponde a um arquivo comum.",
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                "Arquivo não encontrado nesta pasta. A aplicação pode procurar em outros locais."
            }
            Err(_) => "O sistema não permitiu concluir a consulta do arquivo.",
        };
        result.push(text.to_owned());
    }
    if let Ok(entries) = std::fs::read_dir(folder) {
        let mut observed = 0;
        let mut dlls = 0;
        let mut partial = false;
        for entry in entries.take(1001) {
            if observed == 1000 {
                partial = true;
                break;
            }
            observed += 1;
            if let Ok(entry) = entry {
                if entry.file_type().is_ok_and(|kind| kind.is_file())
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
                {
                    dlls += 1;
                }
            } else {
                partial = true;
            }
        }
        result.push(format!(
            "DLLs observadas: {dlls} em {observed} itens da pasta, sem entrar nas subpastas.{}",
            if partial {
                " A leitura foi parcial."
            } else {
                ""
            }
        ));
    } else {
        result.push("Não foi possível listar a pasta.".to_owned());
    }
    result
}

impl SupportPanel {
    pub(super) fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(reader) = &self.pending {
            match reader.try_recv() {
                Ok(items) => {
                    self.pending = None;
                    self.result = items;
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.result = vec!["A consulta foi interrompida.".to_owned()];
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if ui
            .add_enabled(self.pending.is_none(), egui::Button::new("Escolher pasta…"))
            .clicked()
            && let Some(folder) = rfd::FileDialog::new()
                .set_title("Escolher pasta para diagnóstico")
                .pick_folder()
        {
            self.folder = Some(folder);
            self.result.clear();
        }
        if let Some(folder) = &self.folder {
            ui.label(folder.display().to_string());
        }
        ui.label("Nome do arquivo que deveria estar na pasta (opcional)");
        if ui
            .add_enabled(
                self.pending.is_none(),
                egui::TextEdit::singleline(&mut self.expected)
                    .hint_text("exemplo.dll")
                    .char_limit(255),
            )
            .changed()
        {
            self.result.clear();
        }
        if let Some(folder) = self.folder.clone()
            && ui
                .add_enabled(self.pending.is_none(), egui::Button::new("Verificar pasta"))
                .clicked()
        {
            self.result.clear();
            let expected = self.expected.clone();
            let context = ui.ctx().clone();
            let (sender, receiver) = mpsc::channel();
            self.pending = Some(receiver);
            std::thread::spawn(move || {
                let _ = sender.send(inspect(&folder, &expected));
                context.request_repaint();
            });
        }
        for finding in &self.result {
            ui.label(finding);
        }
        ui.separator();
        ui.label(
            "Drivers aparecem em Dispositivos e drivers, nos detalhes da verificação do Windows.",
        );
        ui.collapsing("Instalação de SDKs", |ui| {
            for name in ["DOTNET_ROOT", "JAVA_HOME", "ANDROID_HOME", "WindowsSdkDir"] {
                let state = if std::env::var_os(name).is_some_and(|path| Path::new(&path).is_dir()) { "pasta configurada e encontrada" } else { "pasta não confirmada pela variável" };
                ui.label(format!("{name}: {state}"));
            }
            ui.label("Um SDK pode estar instalado sem essa variável. Use os instaladores oficiais quando o projeto exigir o SDK.");
            ui.hyperlink_to("Instaladores do .NET SDK", "https://dotnet.microsoft.com/en-us/download");
            ui.hyperlink_to("Instaladores do Windows SDK", "https://learn.microsoft.com/en-us/windows/apps/windows-sdk/");
        });
    }
}
