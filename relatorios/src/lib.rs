use std::error::Error;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;

use central_agente::Check;
use docx_rs::{Docx, Paragraph, Run, Table, TableCell, TableRow};
use rust_xlsxwriter::{Chart, ChartType, Format, Workbook};

pub type ReportError = Box<dyn Error + Send + Sync>;

const TITLE: &str = "Relatório de diagnóstico";
const HEADERS: [&str; 5] = [
    "Identificador",
    "Verificação",
    "Estado",
    "Evidência",
    "Ação",
];
const SCOPE: &str = "O relatório apresenta as verificações disponíveis na coleta do agente.";

fn values(check: &Check) -> [&str; 5] {
    [
        check.id,
        check.title,
        check.status,
        &check.evidence,
        check.action,
    ]
}

fn ensure_extension(path: &Path, extension: &str) -> Result<(), ReportError> {
    if path
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|v| v.eq_ignore_ascii_case(extension))
    {
        return Ok(());
    }
    Err(format!("O destino precisa usar a extensão .{extension}.").into())
}

fn save_new(path: &Path, bytes: &[u8]) -> Result<(), ReportError> {
    let mut output = File::create_new(path)?;
    output.write_all(bytes)?;
    Ok(())
}

pub fn write_docx(checks: &[Check], path: &Path) -> Result<(), ReportError> {
    ensure_extension(path, "docx")?;
    let text = |value: &str| Paragraph::new().add_run(Run::new().add_text(value));
    let mut document = Docx::new()
        .add_paragraph(
            Paragraph::new()
                .style("Title")
                .add_run(Run::new().add_text(TITLE).size(40)),
        )
        .add_paragraph(text(SCOPE));
    if checks.is_empty() {
        document = document.add_paragraph(text("Nenhuma verificação disponível."));
    } else {
        let row = |columns: [&str; 5]| {
            TableRow::new(
                columns
                    .into_iter()
                    .map(|value| TableCell::new().add_paragraph(text(value)))
                    .collect(),
            )
        };
        let rows =
            std::iter::once(row(HEADERS)).chain(checks.iter().map(|check| row(values(check))));
        document = document.add_table(Table::new(rows.collect()));
    }
    let mut buffer = Cursor::new(Vec::new());
    document.build().pack(&mut buffer)?;
    save_new(path, buffer.get_ref())
}

pub fn write_xlsx(checks: &[Check], path: &Path) -> Result<(), ReportError> {
    ensure_extension(path, "xlsx")?;
    let mut workbook = Workbook::new();
    let heading = Format::new().set_bold();
    let wrapped = Format::new().set_text_wrap();

    let summary = workbook.add_worksheet().set_name("Painel")?;
    summary.set_column_width(0, 26)?.set_column_width(1, 15)?;
    summary.write_string_with_format(0, 0, TITLE, &heading)?;
    summary.write_string(1, 0, SCOPE)?;
    let categories = ["Ativo", "Requer revisão", "Indisponível", "Outros"];
    let mut totals = [0u32; 4];
    for check in checks {
        let index = categories
            .iter()
            .position(|status| *status == check.status)
            .unwrap_or(3);
        totals[index] += 1;
    }
    for (index, (category, count)) in categories.into_iter().zip(totals).enumerate() {
        let row = index as u32 + 3;
        summary
            .write_string(row, 0, category)?
            .write_number(row, 1, count)?;
    }
    let mut chart = Chart::new(ChartType::Bar);
    chart.title().set_name("Verificações por estado");
    chart
        .add_series()
        .set_categories(("Painel", 3, 0, 6, 0))
        .set_values(("Painel", 3, 1, 6, 1));
    summary.insert_chart(3, 3, &chart)?;

    let details = workbook.add_worksheet().set_name("Verificações")?;
    details
        .set_column_width(0, 38)?
        .set_column_width(1, 40)?
        .set_column_width(2, 20)?;
    details.set_column_width(3, 60)?.set_column_width(4, 60)?;
    details.set_freeze_panes(1, 0)?;
    for (column, name) in HEADERS.into_iter().enumerate() {
        details.write_string_with_format(0, column as u16, name, &heading)?;
    }
    for (index, check) in checks.iter().enumerate() {
        let row = u32::try_from(index + 1)?;
        for (column, value) in values(check).into_iter().enumerate() {
            details.write_string_with_format(row, column as u16, value, &wrapped)?;
        }
    }
    let bytes = workbook.save_to_buffer()?;
    save_new(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::ErrorKind,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn writes_office_archives_without_overwriting_existing_files() -> Result<(), ReportError> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let folder = std::env::temp_dir().join(format!(
            "central-relatorios-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&folder)?;
        let checks = [Check {
            id: "test.firewall",
            title: "Firewall & proteção",
            status: "Ativo",
            evidence: "<observação>".to_owned(),
            action: "=conteúdo literal",
        }];
        type Writer = fn(&[Check], &Path) -> Result<(), ReportError>;
        let formats: [(&str, &[u8], Writer); 2] = [
            ("docx", b"word/document.xml", write_docx),
            ("xlsx", b"xl/charts/chart1.xml", write_xlsx),
        ];
        for (extension, archive_member, export) in formats {
            let destination = folder.join(format!("report.{extension}"));
            export(&checks, &destination)?;
            let bytes = fs::read(&destination)?;
            let collision = export(&[], &destination).unwrap_err();
            assert!(bytes.starts_with(b"PK\x03\x04"));
            assert!(
                bytes
                    .windows(archive_member.len())
                    .any(|entry| entry == archive_member)
            );
            assert_eq!(
                collision
                    .downcast_ref::<std::io::Error>()
                    .map(std::io::Error::kind),
                Some(ErrorKind::AlreadyExists)
            );
            assert_eq!(bytes, fs::read(&destination)?);
            fs::remove_file(destination)?;
        }
        fs::remove_dir(folder)?;
        Ok(())
    }
}
