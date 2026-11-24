#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use central_agente::Check;
use eframe::egui;
use std::sync::mpsc::{self, Receiver, TryRecvError};

mod backup;
mod backup_ui;
mod local_ai;
#[cfg(windows)]
mod monitor;
mod support;

#[derive(Clone, Copy, Default, PartialEq)]
enum Page {
    #[default]
    Overview,
    Tools,
    Backup,
    Settings,
    Chat,
}

#[derive(Clone, Copy, PartialEq)]
enum ToolShortcut {
    File,
    Link,
    Folder,
}

#[derive(Clone, Copy, Default, PartialEq)]
enum ResultFilter {
    #[default]
    All,
    Available,
    Review,
    Unavailable,
}

impl ResultFilter {
    fn includes(self, check: &Check) -> bool {
        match self {
            Self::All => true,
            Self::Review => check.status == "Requer revisão",
            Self::Unavailable => check.status == "Indisponível",
            Self::Available => !matches!(check.status, "Requer revisão" | "Indisponível"),
        }
    }
}

fn open_manual_terminal() -> std::io::Result<()> {
    #[cfg(windows)]
    let mut terminal = std::process::Command::new("wt.exe");
    #[cfg(not(windows))]
    let mut terminal = if let Some(distro) = std::env::var_os("WSL_DISTRO_NAME") {
        let mut command = std::process::Command::new("wt.exe");
        command.arg("wsl.exe").arg("--distribution").arg(distro);
        command
    } else {
        std::process::Command::new("x-terminal-emulator")
    };
    let mut child = terminal.spawn()?;
    std::thread::spawn(move || { let _ = child.wait(); });
    Ok(())
}

fn render_summary(ui: &mut egui::Ui, checks: &[Check], filter: &mut ResultFilter, progress: f32) {
    let review = checks
        .iter()
        .filter(|check| check.status == "Requer revisão")
        .count();
    let unavailable = checks
        .iter()
        .filter(|check| check.status == "Indisponível")
        .count();
    let active = checks.len() - review - unavailable;
    let categories = [
        (
            "Ativos ou consultados",
            active,
            egui::Color32::from_rgb(22, 116, 97),
            ResultFilter::Available,
        ),
        (
            "Para revisar",
            review,
            egui::Color32::from_rgb(162, 89, 26),
            ResultFilter::Review,
        ),
        (
            "Indisponíveis",
            unavailable,
            egui::Color32::from_rgb(92, 105, 116),
            ResultFilter::Unavailable,
        ),
    ];
    ui.horizontal_wrapped(|ui| {
        let (chart, _) = ui.allocate_exact_size(egui::vec2(104.0, 104.0), egui::Sense::hover());
        let center = chart.center();
        let radius = 40.0;
        ui.painter().circle_stroke(center, radius, egui::Stroke::new(11.0_f32, egui::Color32::from_rgb(227, 234, 233)));
        let mut angle = -std::f32::consts::FRAC_PI_2;
        for (_, count, color, _) in categories {
            if count == 0 || progress == 0.0 {
                continue;
            }
            let sweep = std::f32::consts::TAU * progress * count as f32 / checks.len() as f32;
            let points = (0..=48)
                .map(|step| center + egui::Vec2::angled(angle + sweep * step as f32 / 48.0) * radius)
                .collect();
            ui.painter().add(egui::Shape::line(points, egui::Stroke::new(11.0_f32, color)));
            angle += sweep;
        }
        let total = if checks.is_empty() { "—".to_owned() } else { checks.len().to_string() };
        ui.painter().text(center, egui::Align2::CENTER_CENTER, total, egui::FontId::proportional(30.0), ui.visuals().text_color());
        ui.vertical(|ui| {
            ui.strong("Distribuição das verificações");
            ui.label(if checks.is_empty() { "Os dados aparecem após a leitura do sistema." } else { "Selecione um cartão para filtrar a tabela." });
        });
    });
    ui.add_space(5.0);
    ui.horizontal_wrapped(|ui| {
        for (label, count, color, category) in categories {
            let value = if checks.is_empty() { "—".to_owned() } else { count.to_string() };
            let text = egui::RichText::new(format!("{value}\n{label}"))
                .size(16.0)
                .strong()
                .color(color);
            let button = egui::Button::new(text)
                .min_size(egui::vec2(138.0, 60.0))
                .selected(*filter == category);
            if ui.add(button).clicked() {
                *filter = if *filter == category {
                    ResultFilter::All
                } else {
                    category
                };
            }
        }
    });
}

#[derive(Clone, Copy, Default, PartialEq)]
enum ReportFormat {
    #[default]
    Pdf,
    Word,
    Excel,
}

impl ReportFormat {
    fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::Word => "Word",
            Self::Excel => "Excel",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Word => "docx",
            Self::Excel => "xlsx",
        }
    }

    fn save(self, path: &std::path::Path, checks: &[Check]) -> Result<(), String> {
        match self {
            Self::Pdf => {
                central_relatorio_pdf::write_pdf(path, checks).map_err(|error| error.to_string())
            }
            Self::Word => {
                central_relatorios::write_docx(checks, path).map_err(|error| error.to_string())
            }
            Self::Excel => {
                central_relatorios::write_xlsx(checks, path).map_err(|error| error.to_string())
            }
        }
    }
}

#[derive(Default)]
struct Desktop {
    support: support::SupportPanel,
    ai_setup: local_ai::SetupPanel,
    filter: ResultFilter,
    reduce_motion: bool,
    #[cfg(windows)]
    monitor: monitor::Monitor,
    page: Page,
    shortcut: Option<ToolShortcut>,
    backup: backup_ui::BackupPanel,
    pending: Option<Receiver<Vec<Check>>>,
    checks: Vec<Check>,
    notice: &'static str,
    link: String,
    trusted_domain: String,
    link_review: Option<Result<central_agente::links::LinkReview, &'static str>>,
    file_path: String,
    file_pending: Option<Receiver<Result<central_agente::files::FileReview, &'static str>>>,
    file_review: Option<Result<central_agente::files::FileReview, &'static str>>,
    scan_confirm: bool,
    scan_file: Option<std::path::PathBuf>,
    scan_pending: Option<Receiver<Result<&'static str, &'static str>>>,
    scan_notice: Option<Result<&'static str, &'static str>>,
    report_format: ReportFormat,
    report_pending: Option<Receiver<Result<String, String>>>,
    report_notice: Option<String>,
    ai_pending: Option<Receiver<Result<String, &'static str>>>,
    ai_result: Option<Result<String, &'static str>>,
    chat: local_ai::ChatPanel,
    terminal_notice: Option<String>,
    wallet_reference: String,
    wallet_destination: String,
    wallet_result: Option<Result<bool, &'static str>>,
}

impl Desktop {
    fn start_audit(&mut self, context: egui::Context) {
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        self.notice = "Verificando o computador…";
        self.checks.clear();
        self.filter = ResultFilter::All;
        context.animate_bool_with_time(egui::Id::new("audit-summary"), false, 0.0);
        self.ai_result = None;
        self.ai_pending = None;
        std::thread::spawn(move || {
            let _ = sender.send(central_agente::audit_desktop());
            context.request_repaint();
        });
    }

    fn start_export(&mut self, context: egui::Context, frame: &eframe::Frame) {
        let format = self.report_format;
        let dialog = rfd::FileDialog::new()
            .set_parent(frame)
            .set_title("Salvar relatório da verificação")
            .set_file_name(format!("relatorio-diagnostico.{}", format.extension()))
            .add_filter(format.label(), &[format.extension()]);
        let (sender, receiver) = mpsc::channel();
        let checks = self.checks.clone();
        self.report_pending = Some(receiver);
        self.report_notice = Some("Escolha onde salvar o relatório.".to_owned());
        std::thread::spawn(move || {
            let result = (|| {
                let Some(mut path) = dialog.save_file() else {
                    return Ok("Salvamento cancelado.".to_owned());
                };
                if path.extension().is_none() {
                    path.set_extension(format.extension());
                }
                if !path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(format.extension()))
                {
                    return Err(format!(
                        "Use a extensão .{} para este formato.",
                        format.extension()
                    ));
                }
                format
                    .save(&path, &checks)
                    .map(|()| format!("Relatório salvo em {}", path.display()))
            })();
            let _ = sender.send(result);
            context.request_repaint();
        });
    }
}

impl eframe::App for Desktop {
    fn update(&mut self, context: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(windows)]
        self.monitor.update();
        if let Some(receiver) = &self.ai_pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.ai_pending = None;
                    self.ai_result = Some(result);
                }
                Err(TryRecvError::Disconnected) => {
                    self.ai_pending = None;
                    self.ai_result = Some(Err("A explicação local foi interrompida."));
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.report_pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.report_pending = None;
                    self.report_notice = Some(
                        result.unwrap_or_else(|error| format!("Não foi possível salvar: {error}")),
                    );
                }
                Err(TryRecvError::Disconnected) => {
                    self.report_pending = None;
                    self.report_notice = Some("A exportação foi interrompida.".to_owned());
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.scan_pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.scan_pending = None;
                    self.scan_notice = Some(result);
                }
                Err(TryRecvError::Disconnected) => {
                    self.scan_pending = None;
                    self.scan_notice = Some(Err("A integração com o Defender parou."));
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.file_pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.file_pending = None;
                    self.file_review = Some(result);
                }
                Err(TryRecvError::Disconnected) => {
                    self.file_pending = None;
                    self.file_review = Some(Err("A leitura do arquivo parou."));
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(checks) => {
                    self.pending = None;
                    self.notice = if checks.is_empty() {
                        "Nenhuma verificação está disponível neste sistema."
                    } else {
                        "Verificação concluída. Veja os resultados abaixo."
                    };
                    self.checks = checks;
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.notice = "A verificação parou. Você pode tentar novamente.";
                }
                Err(TryRecvError::Empty) => {}
            }
        }

        let sections = [
            (Page::Overview, "Visão geral"),
            (Page::Tools, "Ferramentas"),
            (Page::Backup, "Backup"),
            (Page::Settings, "Configurações"),
            (Page::Chat, "IA local"),
        ];
        let compact = context.content_rect().width() < 640.0;
        if compact {
            egui::TopBottomPanel::top("central-compact-navigation").show(context, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (page, title) in sections {
                        ui.selectable_value(&mut self.page, page, title);
                    }
                });
            });
        } else {
        egui::SidePanel::left("central-navigation")
            .exact_width(152.0)
            .resizable(false)
            .frame(egui::Frame::new()
                .fill(egui::Color32::from_rgb(232, 241, 239))
                .inner_margin(egui::Margin::same(14)))
            .show(context, |ui| {
                ui.heading("Central");
                ui.label("Diagnóstico local");
                ui.add_space(18.0);
                for (page, title) in sections {
                    let button = egui::Button::selectable(self.page == page, title);
                    if ui.add_sized([ui.available_width(), 36.0], button).clicked() {
                        self.page = page;
                    }
                }
            });
        }
        egui::CentralPanel::default().frame(egui::Frame::central_panel(&context.style()).inner_margin(egui::Margin::same(22))).show(context, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.page == Page::Overview {
                egui::Frame::new().fill(egui::Color32::from_rgb(223, 240, 235))
                    .corner_radius(14).inner_margin(egui::Margin::same(18)).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(egui::RichText::new("Cuidado com seu computador").size(27.0).strong());
                ui.label("Confira as configurações de proteção deste dispositivo.");
                ui.add_space(14.0);
                let button =
                    egui::Button::new(egui::RichText::new("Verificar meu computador").color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(17, 100, 86)).min_size(egui::vec2(230.0, 44.0));
                if ui.add_enabled(self.pending.is_none() && self.ai_pending.is_none(), button).clicked() {
                    self.start_audit(context.clone());
                }
                if self.pending.is_some() {
                    ui.horizontal_wrapped(|ui| {
                        if !self.reduce_motion {
                            ui.spinner();
                        }
                        ui.label("Consultando as configurações de proteção e os dados do sistema.");
                    });
                } else if !self.notice.is_empty() {
                    ui.label(self.notice);
                }
                });
                ui.add_space(16.0);
                ui.heading("Ações rápidas");
                ui.horizontal_wrapped(|ui| {
                    for (label, page, shortcut) in [
                        ("Conferir um link", Page::Tools, Some(ToolShortcut::Link)),
                        ("Revisar um arquivo", Page::Tools, Some(ToolShortcut::File)),
                        ("Revisar uma pasta", Page::Tools, Some(ToolShortcut::Folder)),
                        ("Abrir backup", Page::Backup, None),
                    ] {
                        let card = egui::Button::new(egui::RichText::new(label).strong())
                            .min_size(egui::vec2(120.0, 48.0))
                            .fill(egui::Color32::WHITE);
                        if ui.add(card).clicked() {
                            self.page = page;
                            self.shortcut = shortcut;
                        }
                    }
                });
                #[cfg(windows)]
                egui::CollapsingHeader::new("Monitorar em segundo plano").default_open(true).show(ui, |ui| self.monitor.show(ui));
                {
                    ui.add_space(14.0);
                    ui.heading("Resultados");
                    let progress = if self.reduce_motion { 1.0 } else { context.animate_bool_with_time(egui::Id::new("audit-summary"), true, 0.32) };
                    render_summary(ui, &self.checks, &mut self.filter, progress);
                    ui.add_space(8.0);
                    for check in self.checks.iter().filter(|check| matches!(check.id, "hardware.memory" | "hardware.disk")) {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(check.title);
                            ui.label(&check.evidence);
                        });
                    }
                    ui.heading("Detalhes da verificação");
                    ui.horizontal_wrapped(|ui| {
                        for (filter, label) in [
                            (ResultFilter::All, "Todos"),
                            (ResultFilter::Available, "Ativos ou consultados"),
                            (ResultFilter::Review, "Para revisar"),
                            (ResultFilter::Unavailable, "Indisponíveis"),
                        ] {
                            ui.selectable_value(&mut self.filter, filter, label);
                        }
                    });
                    let mut visible = 0;
                    egui::Grid::new("audit-table").num_columns(2).striped(true).show(ui, |ui| {
                        ui.strong("Verificação"); ui.strong("Estado"); ui.end_row();
                        for check in self.checks.iter().filter(|check| self.filter.includes(check)) {
                            visible += 1;
                            ui.push_id(check.id, |ui| ui.collapsing(check.title, |ui| {
                                ui.label(&check.evidence);
                                if check.status != "Ativo" { ui.label(check.action); }
                            }));
                            let color = match check.status {
                                "Requer revisão" => egui::Color32::from_rgb(143, 70, 12),
                                "Indisponível" => egui::Color32::from_rgb(78, 91, 106),
                                _ => egui::Color32::from_rgb(17, 100, 86),
                            };
                            ui.label(egui::RichText::new(check.status).color(color).strong());
                            ui.end_row();
                        }
                    });
                    if self.pending.is_some() {
                        ui.label("Carregando os resultados da verificação…");
                    } else if self.checks.is_empty() {
                        ui.label("Nenhum resultado disponível.");
                    } else if visible == 0 {
                        ui.label("Nenhum item neste filtro.");
                    } else {
                        ui.label(format!("{visible} de {} verificações exibidas", self.checks.len()));
                    }
                    egui::CollapsingHeader::new("Explicar com IA local").default_open(true).show(ui, |ui| {
                        ui.label("Opcional. Envia só contagens e o estado do Defender ao Ollama neste computador.");
                        if ui.add_enabled(!self.checks.is_empty() && self.ai_pending.is_none(), egui::Button::new("Pedir explicação ao Ollama")).clicked() {
                            let checks = self.checks.clone();
                            let graphics = context.clone();
                            let (sender, receiver) = mpsc::channel();
                            self.ai_pending = Some(receiver);
                            self.ai_result = None;
                            std::thread::spawn(move || {
                                let _ = sender.send(local_ai::explain(&checks));
                                graphics.request_repaint();
                            });
                        }
                        if self.ai_pending.is_some() { ui.label("Aguardando a explicação…"); }
                        if let Some(result) = &self.ai_result {
                            match result {
                                Ok(text) => { ui.label("Explicação gerada por IA. Confira os resultados da auditoria antes de agir."); ui.label(text); }
                                Err(message) => { ui.label(*message); }
                            }
                        }
                    });
                    ui.separator();
                    egui::ComboBox::from_label("Formato do relatório")
                        .selected_text(self.report_format.label())
                        .show_ui(ui, |ui| {
                            for format in [ReportFormat::Pdf, ReportFormat::Word, ReportFormat::Excel] {
                                ui.selectable_value(&mut self.report_format, format, format.label());
                            }
                        });
                    if ui.add_enabled(!self.checks.is_empty() && self.report_pending.is_none(), egui::Button::new("Salvar relatório")).clicked() {
                        self.start_export(context.clone(), frame);
                    }
                }
                if let Some(message) = &self.report_notice { ui.label(message); }
                } else if self.page == Page::Chat {
                    if ui.button("Abrir terminal interativo").clicked() {
                        self.terminal_notice = open_manual_terminal().err().map(|error| error.to_string());
                    }
                    if let Some(error) = &self.terminal_notice {
                        ui.label(format!("Não foi possível abrir o terminal: {error}"));
                    }
                    if ui.add_enabled(self.pending.is_none(), egui::Button::new("Verificar meu computador novamente")).clicked() {
                        self.start_audit(context.clone());
                    }
                    let show_output = |ui: &mut egui::Ui| {
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(232, 241, 239))
                            .inner_margin(egui::Margin::same(14))
                            .corner_radius(8)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.heading("Saída do diagnóstico");
                                ui.label("Somente leitura. Digite comandos no terminal interativo.");
                                if self.pending.is_some() {
                                    ui.label("Atualizando o diagnóstico…");
                                } else if self.checks.is_empty() {
                                    ui.label("Nenhum resultado disponível.");
                                }
                                for check in &self.checks {
                                    ui.add_space(8.0);
                                    ui.monospace(format!("[{}] {}", check.status, check.title));
                                    ui.monospace(&check.evidence);
                                }
                            });
                    };
                    if ui.available_width() >= 650.0 {
                        ui.columns(2, |columns| {
                            self.chat.show(&mut columns[0], &self.checks);
                            show_output(&mut columns[1]);
                        });
                    } else {
                        self.chat.show(ui, &self.checks);
                        ui.separator();
                        show_output(ui);
                    }
                } else if self.page == Page::Backup {
                    self.backup.show(ui);
                } else if self.page == Page::Settings {
                    ui.heading("Configurações de visualização");
                    ui.label("As preferências abaixo valem para esta sessão.");
                    ui.checkbox(&mut self.reduce_motion, "Reduzir movimento do gráfico");
                    let mut zoom = context.zoom_factor();
                    if ui.add(egui::Slider::new(&mut zoom, 1.0..=1.6).step_by(0.05).text("Tamanho da interface")).changed() {
                        context.set_zoom_factor(zoom);
                    }
                    ui.separator();
                    self.ai_setup.show(ui);
                } else {
                let shortcut = self.shortcut.take();
                ui.heading("Ferramentas deste computador");
                ui.add_space(12.0);
                egui::CollapsingHeader::new("Pastas, DLLs e SDKs").default_open(true).open((shortcut == Some(ToolShortcut::Folder)).then_some(true)).show(ui, |ui| self.support.show(ui));
                ui.collapsing("Conferir destino Bitcoin", |ui| {
                    ui.label("Use somente endereços públicos da rede principal, confirmados por um canal confiável. Nunca cole chaves privadas ou palavras de recuperação.");
                    ui.label("Endereço público confirmado");
                    let reference = ui.add(egui::TextEdit::singleline(&mut self.wallet_reference).char_limit(90));
                    ui.label("Endereço que aparece na carteira");
                    let destination = ui.add(egui::TextEdit::singleline(&mut self.wallet_destination).char_limit(90));
                    if reference.changed() || destination.changed() { self.wallet_result = None; }
                    ui.horizontal(|ui| {
                        if ui.button("Comparar endereços").clicked() {
                            self.wallet_result = Some(central_agente::wallet::compare_recipients(&self.wallet_reference, &self.wallet_destination));
                        }
                        if ui.button("Limpar endereços").clicked() {
                            self.wallet_reference.clear();
                            self.wallet_destination.clear();
                            self.wallet_result = None;
                        }
                    });
                    if let Some(result) = self.wallet_result {
                        ui.label(match result {
                            Ok(true) => "O destino coincide. Confira também o valor e o endereço na tela de assinatura da carteira.",
                            Ok(false) => "Os destinos são diferentes. Pare e confirme o endereço por um canal confiável.",
                            Err(message) => message,
                        });
                    }
                });
                if cfg!(any(windows, target_os = "linux")) {
                    let engine = if cfg!(windows) { "Defender" } else { "ClamAV" };
                    ui.collapsing(format!("Varredura com o {engine}"), |ui| {
                        if cfg!(windows) { ui.label("As detecções ficam no Histórico de proteção do Windows."); }
                        else { ui.label("O ClamAV usa as assinaturas instaladas no próprio computador."); }
                        if self.scan_pending.is_some() {
                            ui.label(format!("Aguardando resposta do {engine}…"));
                        } else if self.scan_confirm {
                            if let Some(path) = &self.scan_file {
                                ui.label(format!("Verificar este arquivo com o {engine}? {}", path.display()));
                            } else {
                                ui.label("Deseja iniciar a varredura rápida neste computador?");
                            }
                            ui.horizontal(|ui| {
                                if ui.button("Sim, iniciar varredura").clicked() {
                                    let (sender, receiver) = mpsc::channel();
                                    let graphics = context.clone();
                                    let file = self.scan_file.take();
                                    self.scan_confirm = false;
                                    self.scan_notice = None;
                                    self.scan_pending = Some(receiver);
                                    std::thread::spawn(move || {
                                        let result = match file {
                                            Some(path) => central_agente::scan::scan_file(&path),
                                            None => central_agente::scan::defender_quick_scan(),
                                        };
                                        let _ = sender.send(result);
                                        graphics.request_repaint();
                                    });
                                }
                                if ui.button("Cancelar").clicked() { self.scan_confirm = false; self.scan_file = None; }
                            });
                        } else {
                            if cfg!(windows) && ui.button("Solicitar varredura rápida").clicked() { self.scan_confirm = true; }
                            if ui.button("Escolher arquivo para varredura…").clicked()
                                && let Some(path) = rfd::FileDialog::new().set_title("Escolher arquivo para varredura").pick_file() {
                                self.scan_file = Some(path);
                                self.scan_confirm = true;
                            }
                        }
                        if let Some(Ok(message) | Err(message)) = self.scan_notice { ui.label(message); }
                    });
                }
                egui::CollapsingHeader::new("Revisar arquivo local").default_open(true).open((shortcut == Some(ToolShortcut::File)).then_some(true)).show(ui, |ui| {
                    ui.label("Escolha um arquivo para conferir seu identificador sem executá-lo.");
                    if ui.add_enabled(self.file_pending.is_none(), egui::Button::new("Escolher arquivo…")).clicked()
                        && let Some(path) = rfd::FileDialog::new().set_title("Escolher arquivo para revisão").pick_file() {
                        self.file_path = path.display().to_string();
                        self.file_review = None;
                    }
                    let input = egui::TextEdit::singleline(&mut self.file_path).hint_text("Caminho do arquivo").char_limit(4096);
                    if ui.add_enabled(self.file_pending.is_none(), input).changed() { self.file_review = None; }
                    let enabled = self.file_pending.is_none() && !self.file_path.trim().is_empty();
                    if ui.add_enabled(enabled, egui::Button::new("Revisar arquivo")).clicked() {
                        let path = std::path::PathBuf::from(self.file_path.trim().trim_matches('"'));
                        let (sender, receiver) = mpsc::channel();
                        let graphics = context.clone();
                        self.file_pending = Some(receiver);
                        self.file_review = None;
                        std::thread::spawn(move || {
                            let _ = sender.send(central_agente::files::inspect(&path));
                            graphics.request_repaint();
                        });
                    }
                    if self.file_pending.is_some() { ui.label("Lendo arquivo…"); }
                    match &self.file_review {
                        Some(Ok(result)) => {
                            ui.label(format!("{} bytes", result.bytes));
                            ui.monospace(format!("SHA-256: {}", result.sha256));
                            if result.executable_extension { ui.label("A extensão pode iniciar código. Confira a origem antes de executar."); }
                            for finding in &result.email_findings { ui.label(finding); }
                            ui.label("A impressão digital identifica esta cópia. Use um antivírus para detectar ameaças.");
                        }
                        Some(Err(error)) => { ui.label(*error); }
                        None => {}
                    }
                });
                egui::CollapsingHeader::new("Conferir link antes de abrir").default_open(true).open((shortcut == Some(ToolShortcut::Link)).then_some(true)).show(ui, |ui| {
                    ui.label("Cole o link recebido. A conferência não abre o site.");
                    let link = ui.add(egui::TextEdit::singleline(&mut self.link).hint_text("https://…").char_limit(4096));
                    ui.label("Domínio oficial, se você já o confirmou");
                    let trusted = ui.add(egui::TextEdit::singleline(&mut self.trusted_domain).hint_text("exemplo.com.br").char_limit(253));
                    if link.changed() || trusted.changed() { self.link_review = None; }
                    if ui.button("Conferir link").clicked() {
                        self.link_review = Some(central_agente::links::review(&self.link, &self.trusted_domain));
                    }
                    match &self.link_review {
                        Some(Ok(result)) => {
                            ui.label(format!("Destino: {}", result.host));
                            for finding in &result.findings { ui.label(*finding); }
                            ui.label("O resultado descreve o endereço. A reputação do site não foi consultada.");
                        }
                        Some(Err(error)) => { ui.label(*error); }
                        None => {}
                    }
                });
                }
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1060.0, 820.0])
            .with_min_inner_size([380.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Central de diagnóstico",
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = egui::Color32::from_rgb(247, 249, 249);
            visuals.selection.bg_fill = egui::Color32::from_rgb(192, 229, 219);
            cc.egui_ctx.set_visuals(visuals);
            cc.egui_ctx.set_zoom_factor(1.15);
            let mut desktop = Desktop::default();
            desktop.start_audit(cc.egui_ctx.clone());
            Ok(Box::new(desktop))
        }),
    )
}
