import { Document, HeadingLevel, Packer, Paragraph, Table, TableCell, TableRow, TextRun, WidthType } from 'docx';
import ExcelJS from 'exceljs';
import PDFDocument from 'pdfkit';

export const formats = {
  docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  pdf: 'application/pdf',
};

const title = 'Relatório de diagnóstico';
const section = 'Resultados da verificação';
const headings = ['Área', 'Estado', 'Evidência'];
const format = new Intl.NumberFormat('pt-BR', { maximumFractionDigits: 1 });

function reportContent(report) {
  const metrics = [['Memória', report.ram.free, report.ram.total, report.ram.percent]];
  if (report.disk?.total) metrics.push(['Disco do sistema', report.disk.free, report.disk.total, Math.round(100 * (1 - report.disk.free / report.disk.total))]);
  const partial = report.events.some(({ status }) => status === 'Indisponível');
  const intro = `Coleta de ${new Date(report.time).toLocaleString('pt-BR')} em ${report.platform}. Foram identificados ${report.cores} processadores lógicos disponíveis. ${partial ? 'Há consultas indisponíveis, indicadas na tabela.' : 'As consultas previstas para esta coleta responderam.'}`;
  const scope = 'A coleta consulta capacidade e estado da proteção. A identificação de malware e o diagnóstico físico de componentes exigem verificações adicionais.';
  return { metrics, intro, scope, events: report.events.map(({ area, status, detail }) => [area, status, detail]) };
}

function metricLabel([label, free, total, used]) {
  return `${label} · ${format.format(free)} de ${format.format(total)} GiB livres · ${used}% ocupado`;
}

async function word({ metrics, intro, scope, events }) {
  const table = new Table({ width: { size: 100, type: WidthType.PERCENTAGE }, rows: [headings, ...events].map((row, index) => new TableRow({ tableHeader: index === 0, children: row.map((value) => new TableCell({ children: [new Paragraph({ children: [new TextRun({ text: value, bold: index === 0 })] })] })) })) });
  const children = [new Paragraph({ text: title, heading: HeadingLevel.TITLE }), new Paragraph(intro), new Paragraph({ text: 'Uso de recursos', heading: HeadingLevel.HEADING_1 })];
  for (const metric of metrics) {
    const bars = Math.round(metric[3] / 5);
    children.push(new Paragraph(metricLabel(metric)), new Paragraph({ children: [new TextRun({ text: '■'.repeat(bars), color: '343B72' }), new TextRun({ text: '□'.repeat(20 - bars), color: '5A6273' })] }));
  }
  children.push(new Paragraph({ text: section, heading: HeadingLevel.HEADING_1 }), table, new Paragraph(scope));
  return Packer.toBuffer(new Document({ sections: [{ children }] }));
}

async function excel({ metrics, intro, scope, events }) {
  const book = new ExcelJS.Workbook();
  const dashboard = book.addWorksheet('Painel');
  dashboard.columns = [{ width: 28 }, { width: 18 }, { width: 18 }, { width: 18 }, { width: 34 }];
  dashboard.mergeCells('A1:E1');
  dashboard.mergeCells('A2:E2');
  dashboard.getCell('A1').value = title;
  dashboard.getCell('A2').value = intro;
  dashboard.getCell('A2').alignment = { wrapText: true };
  dashboard.getRow(2).height = 36;
  dashboard.addRow(['Recurso', 'Livre (GiB)', 'Total (GiB)', 'Ocupação', 'Gráfico']);
  for (const [label, free, total, used] of metrics) {
    const row = dashboard.addRow([label, free, total, used / 100, '■'.repeat(Math.round(used / 5))]);
    row.getCell(4).numFmt = '0%';
    row.getCell(5).font = { color: { argb: 'FF343B72' } };
  }
  const audit = book.addWorksheet('Eventos');
  audit.columns = [{ width: 24 }, { width: 23 }, { width: 80 }];
  audit.addRows([headings, ...events]);
  audit.getColumn(3).alignment = { wrapText: true };
  const notes = book.addWorksheet('Escopo');
  notes.getColumn(1).width = 85;
  notes.addRow([scope]).height = 40;
  notes.getColumn(1).alignment = { wrapText: true };
  for (const row of [dashboard.getRow(1), dashboard.getRow(3), audit.getRow(1)]) {
    row.font = { bold: true, color: { argb: 'FF212750' } };
  }
  return book.xlsx.writeBuffer();
}

function pdf({ metrics, intro, scope, events }) {
  return new Promise((resolve, reject) => {
    const doc = new PDFDocument({ size: 'A4', margin: 48 });
    const chunks = [];
    doc.on('data', (chunk) => chunks.push(chunk));
    doc.on('end', () => resolve(Buffer.concat(chunks)));
    doc.on('error', reject);
    doc.fontSize(23).text(title).moveDown();
    doc.fontSize(11).text(intro).moveDown();
    doc.fontSize(15).text('Uso de recursos').moveDown(0.5);
    for (const metric of metrics) {
      doc.fontSize(11).text(metricLabel(metric));
      const y = doc.y + 4;
      doc.rect(48, y, 240, 9).fill('#E1E4EF');
      doc.rect(48, y, 240 * metric[3] / 100, 9).fill('#343B72');
      doc.fillColor('#171F35').text('', 48, y + 17);
    }
    doc.moveDown().fontSize(15).text(section).moveDown(0.5);
    for (const [area, status, detail] of events) {
      doc.font('Helvetica-Bold').fontSize(11).text(`${area} · ${status}`);
      doc.font('Helvetica').text(detail).moveDown(0.5);
    }
    doc.moveDown().text(scope);
    doc.end();
  });
}

export function renderReport(extension, report) {
  return { docx: word, xlsx: excel, pdf }[extension](reportContent(report));
}
