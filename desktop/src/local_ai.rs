use central_agente::Check;
use eframe::egui;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

const ENDPOINT: &str = "http://127.0.0.1:11434/api/generate";
const MODEL: &str = "gemma4:e2b";

#[derive(Default)]
pub(super) struct SetupPanel {
    pending: Option<Receiver<&'static str>>,
    confirmed: bool,
    status: Option<&'static str>,
}

impl SetupPanel {
    pub(super) fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(status) => {
                    self.pending = None;
                    self.status = Some(status);
                }
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.status = Some("O pedido de download foi interrompido.");
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        ui.heading("Preparar IA local");
        ui.label("Instale e inicie o Ollama antes de baixar o Gemma 4. A Central usa a IA para explicar resultados resumidos da verificação.");
        let installer = if cfg!(windows) {
            "https://ollama.com/download/windows"
        } else {
            "https://ollama.com/download/linux"
        };
        ui.hyperlink_to("Abrir instalador oficial do Ollama", installer);
        if self.pending.is_some() {
            ui.label("Aguardando o download no Ollama…");
        } else if self.confirmed {
            ui.label("O download usa internet e pode ocupar vários gigabytes na máquina.");
            ui.horizontal(|ui| {
                if ui.button("Confirmar download").clicked() {
                    let (sender, receiver) = mpsc::channel();
                    let context = ui.ctx().clone();
                    self.pending = Some(receiver);
                    self.confirmed = false;
                    self.status = None;
                    std::thread::spawn(move || {
                        let _ = sender.send(pull_model());
                        context.request_repaint();
                    });
                }
                if ui.button("Cancelar").clicked() {
                    self.confirmed = false;
                }
            });
        } else if ui.button("Preparar download do Gemma 4").clicked() {
            self.confirmed = true;
        }
        if let Some(message) = self.status {
            ui.label(message);
        }
    }
}

fn pull_model() -> &'static str {
    let client: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .timeout_connect(Some(Duration::from_secs(3)))
        .timeout_global(Some(Duration::from_secs(900)))
        .build()
        .into();
    let result = client
        .post("http://127.0.0.1:11434/api/pull")
        .send_json(serde_json::json!({ "model": MODEL, "stream": false }));
    if let Ok(mut response) = result
        && response.status().as_u16() == 200
        && let Ok(value) = response
            .body_mut()
            .with_config()
            .limit(65_536)
            .read_json::<serde_json::Value>()
        && value["status"] == "success"
    {
        return "O Ollama confirmou o download do Gemma 4.";
    }
    "O Ollama não confirmou a conclusão. Confira se ele está aberto e verifique o progresso no próprio Ollama."
}

fn prompt(checks: &[Check]) -> String {
    let total = checks.len();
    let review = checks
        .iter()
        .filter(|check| check.status == "Requer revisão")
        .count();
    let unavailable = checks
        .iter()
        .filter(|check| check.status == "Indisponível")
        .count();
    let protection = checks
        .iter()
        .find(|check| check.id == "windows.defender.realtime")
        .map_or("não consultada", |check| match check.status {
            "Ativo" => "ativa",
            "Requer revisão" => "requer revisão",
            _ => "indisponível",
        });
    format!(
        "Explique esta auditoria em português simples, em até 3 frases. Os dados são contagens, não diagnósticos. Total: {total}; para revisar: {review}; indisponíveis: {unavailable}; proteção em tempo real do Defender: {protection}. Sugira conferir os detalhes e pedir ajuda de confiança se necessário. Não proponha comandos, downloads, links, troca de antivírus ou mudança de configurações."
    )
}

pub(super) fn explain(checks: &[Check]) -> Result<String, &'static str> {
    request(ENDPOINT, &prompt(checks))
}

#[derive(Default)]
pub(super) struct ChatPanel {
    draft: String,
    turns: Vec<(String, Result<String, &'static str>)>,
    pending: Option<(String, Receiver<Result<String, &'static str>>)>,
}

impl ChatPanel {
    pub(super) fn show(&mut self, ui: &mut egui::Ui, checks: &[Check]) {
        if let Some((question, receiver)) = &self.pending {
            let answer = match receiver.try_recv() {
                Ok(answer) => Some(answer),
                Err(TryRecvError::Disconnected) => Some(Err("A consulta foi interrompida.")),
                Err(TryRecvError::Empty) => None,
            };
            if let Some(answer) = answer {
                self.turns.push((question.clone(), answer));
                self.pending = None;
                if self.turns.len() > 12 {
                    let _ = self.turns.remove(0);
                }
            }
        }

        ui.heading("Conversar com a IA local");
        ui.label("O Gemma recebe sua pergunta e os nomes e estados das verificações. As respostas orientam o diagnóstico. Alterações no computador exigem uma ação sua.");
        for (question, answer) in &self.turns {
            ui.strong(question);
            ui.label(answer.as_ref().map_or_else(|error| *error, String::as_str));
            ui.add_space(8.0);
        }
        if let Some((question, _)) = &self.pending {
            ui.strong(question);
            ui.label("Aguardando o Ollama local…");
        }
        ui.add(egui::TextEdit::multiline(&mut self.draft)
            .hint_text("Ex.: O que devo verificar primeiro?")
            .desired_width(f32::INFINITY)
            .char_limit(1000));
        let can_send = self.pending.is_none() && !self.draft.trim().is_empty();
        if ui.add_enabled(can_send, egui::Button::new("Enviar pergunta")).clicked() {
            let question = std::mem::take(&mut self.draft).trim().to_owned();
            let summary = checks.iter()
                .map(|check| format!("{}: {}", check.title, check.status))
                .collect::<Vec<_>>().join("\n");
            let recent = self.turns.iter().rev().take(3).collect::<Vec<_>>();
            let history = recent.into_iter().rev()
                .filter_map(|(question, answer)| answer.as_ref().ok().map(|answer| format!("Usuário: {question}\nIA: {answer}")))
                .collect::<Vec<_>>().join("\n");
            let input = format!(
                "Você é um assistente de suporte defensivo. Responda em português simples. Use o diagnóstico como contexto. Recomende passos reversíveis pelas configurações do sistema e explique o que precisa ser confirmado. Não afirme que alterou a máquina.\nDiagnóstico:\n{summary}\nConversa anterior:\n{history}\nPergunta atual: {question}"
            );
            let (sender, receiver) = mpsc::channel();
            self.pending = Some((question, receiver));
            let context = ui.ctx().clone();
            std::thread::spawn(move || {
                let _ = sender.send(request(ENDPOINT, &input));
                context.request_repaint();
            });
        }
    }
}

fn request(endpoint: &str, prompt: &str) -> Result<String, &'static str> {
    let client: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .max_redirects(0)
        .timeout_connect(Some(Duration::from_secs(3)))
        .timeout_global(Some(Duration::from_secs(45)))
        .build()
        .into();
    let mut response = client.post(endpoint).send_json(serde_json::json!({
        "model": MODEL, "prompt": prompt, "stream": false,
        "options": { "num_predict": 160, "temperature": 0 }
    })).map_err(|error| match error {
        ureq::Error::StatusCode(404) => "O Ollama não encontrou o modelo gemma4:e2b. Confira os modelos locais antes de tentar novamente.",
        _ => "O Ollama local não respondeu. A verificação do computador continua disponível.",
    })?;
    if response.status().as_u16() != 200 {
        return Err("O serviço local retornou uma resposta inesperada.");
    }
    let value: serde_json::Value = response
        .body_mut()
        .with_config()
        .limit(65_536)
        .read_json()
        .map_err(|_| "O Ollama retornou uma resposta que não pôde ser lida.")?;
    let text = value["response"]
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or("O Ollama não retornou uma explicação.")?;
    Ok(text.trim().chars().take(1800).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_excludes_raw_metadata() {
        let check = Check {
            id: "windows.defender.realtime",
            title: "SENTINEL_TITLE",
            status: "Requer revisão",
            evidence: "SENTINEL_EVIDENCE".into(),
            action: "SENTINEL_ACTION",
        };
        let result = prompt(&[check]);
        assert!(!result.contains("SENTINEL"));
        assert!(result.contains("para revisar: 1"));
        assert!(result.contains("Defender: requer revisão"));
    }

    #[test]
    fn reads_explanation_from_loopback_fixture() {
        use std::{
            io::{BufRead, BufReader, Read, Write},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let fixture = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.starts_with("POST /api/generate "));
            let mut length = 0;
            loop {
                line.clear();
                assert_ne!(reader.read_line(&mut line).unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            let mut request = vec![0; length];
            reader.read_exact(&mut request).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&request).unwrap();
            assert_eq!(value["model"], "gemma4:e2b");
            assert_eq!(value["stream"], false);
            let reply = r#"{"response":"Revise a proteção no painel."}"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len()).unwrap();
        });
        assert_eq!(
            request(&format!("http://{addr}/api/generate"), "Teste").unwrap(),
            "Revise a proteção no painel."
        );
        fixture.join().unwrap();
    }
}
