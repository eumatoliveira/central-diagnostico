use central_agente::Check;
use pdf_writer::{Content, Name, Pdf, Rect, Ref, Str};
use std::{fs::OpenOptions, io::Write, path::Path};

const WIDTH: f32 = 595.0;
const HEIGHT: f32 = 842.0;
const LEFT: f32 = 48.0;
const LINE_HEIGHT: f32 = 14.0;
const PAGE_LINES: usize = 50;
const LINE_COLUMNS: usize = 78;

fn wrap(text: &str, lines: &mut Vec<String>) {
    let mut line = String::new();
    let mut columns = 0;
    for word in text.split_whitespace() {
        let word_columns = word.chars().count();
        if columns != 0 && columns + 1 + word_columns > LINE_COLUMNS {
            lines.push(std::mem::take(&mut line));
            columns = 0;
        }
        if columns != 0 {
            line.push(' ');
            columns += 1;
        }
        for character in word.chars() {
            if columns == LINE_COLUMNS {
                lines.push(std::mem::take(&mut line));
                columns = 0;
            }
            line.push(character);
            columns += 1;
        }
    }
    lines.push(line);
}

fn win_ansi(text: &str) -> Vec<u8> {
    text.chars()
        .map(|character| match character {
            ' '..='~' | '\u{00A0}'..='\u{00FF}' => character as u8,
            '€' => 0x80,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            '…' => 0x85,
            _ => b'?',
        })
        .collect()
}

fn text(stream: &mut Content, font: Name<'_>, size: f32, y: f32, value: &str) {
    let encoded = win_ansi(value);
    stream
        .begin_text()
        .set_font(font, size)
        .next_line(LEFT, y)
        .show(Str(&encoded))
        .end_text();
}

fn render(checks: &[Check]) -> Vec<u8> {
    let mut lines = Vec::new();
    if checks.is_empty() {
        lines.push("Nenhuma verificação disponível nesta coleta.".to_owned());
    }
    for check in checks {
        wrap(&format!("{}  |  {}", check.title, check.status), &mut lines);
        wrap(&format!("Evidência • {}", check.evidence), &mut lines);
        wrap(&format!("Ação sugerida • {}", check.action), &mut lines);
        lines.push(String::new());
    }
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }

    let mut pdf = Pdf::new();
    pdf.set_version(1, 4);
    let catalog = Ref::new(1);
    let tree = Ref::new(2);
    let body = Ref::new(3);
    let heading = Ref::new(4);
    let body_name = Name(b"Body");
    let heading_name = Name(b"Heading");
    let count = lines.len().div_ceil(PAGE_LINES);
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree)
        .kids((0..count).map(|index| Ref::new(5 + index as i32 * 2)))
        .count(count as i32);
    pdf.type1_font(body)
        .base_font(Name(b"Courier"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));
    pdf.type1_font(heading)
        .base_font(Name(b"Helvetica-Bold"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));

    for (index, page_lines) in lines.chunks(PAGE_LINES).enumerate() {
        let page_id = Ref::new(5 + index as i32 * 2);
        let stream_id = Ref::new(6 + index as i32 * 2);
        pdf.page(page_id)
            .parent(tree)
            .media_box(Rect::new(0.0, 0.0, WIDTH, HEIGHT))
            .contents(stream_id)
            .resources()
            .fonts()
            .pair(body_name, body)
            .pair(heading_name, heading);

        let mut stream = Content::new();
        text(
            &mut stream,
            heading_name,
            20.0,
            788.0,
            "Relatório de diagnóstico",
        );
        for (line, value) in page_lines.iter().enumerate() {
            if !value.is_empty() {
                let y = 754.0 - LINE_HEIGHT * line as f32;
                text(&mut stream, body_name, 10.0, y, value);
            }
        }
        let footer = format!("Página {} de {}", index + 1, count);
        text(&mut stream, body_name, 10.0, 42.0, &footer);
        pdf.stream(stream_id, &stream.finish());
    }
    pdf.finish()
}

/// Writes a Portuguese audit report to a new file and preserves any existing file.
/// Text outside WinAnsiEncoding is rendered with a question mark.
pub fn write_pdf(path: &Path, checks: &[Check]) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&render(checks))?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_portuguese_and_wraps_long_words() {
        assert_eq!(win_ansi("Ação —"), b"A\xe7\xe3o \x97");
        let mut lines = Vec::new();
        wrap(&"á".repeat(LINE_COLUMNS + 1), &mut lines);
        assert_eq!(lines[0].chars().count(), LINE_COLUMNS);
        assert_eq!(lines[1], "á");
    }

    #[test]
    fn paginates_without_overwriting_existing_report() {
        let checks = (0..31)
            .map(|_| Check {
                id: "test",
                title: "Proteção pública",
                status: "Requer revisão",
                evidence: "Conexão interrompida".to_owned(),
                action: "Revise a configuração.",
            })
            .collect::<Vec<_>>();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("central-pdf-{}-{now}.pdf", std::process::id()));
        write_pdf(&path, &checks).unwrap();
        assert!(write_pdf(&path, &[]).is_err());
        let data = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(data.starts_with(b"%PDF-1.4"));
        assert!(data.windows(8).any(|bytes| bytes == b"/Count 3"));
    }
}
