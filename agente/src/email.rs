use mailparse::{MailAddr, MailHeaderMap, addrparse_header, parse_headers};
use std::collections::BTreeSet;

pub(super) const PREVIEW_LIMIT: usize = 512 * 1024;

pub(super) fn review(prefix: &[u8], truncated: bool) -> Vec<String> {
    let Some(header_end) = prefix
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .map(|offset| offset + 4)
        .or_else(|| {
            prefix
                .windows(2)
                .position(|bytes| bytes == b"\n\n")
                .map(|offset| offset + 2)
        })
        .filter(|length| *length <= 64 * 1024)
    else {
        return vec!["Não foi possível analisar o cabeçalho do e-mail.".to_owned()];
    };
    let Ok((headers, _)) = parse_headers(&prefix[..header_end]) else {
        return vec!["O cabeçalho do e-mail não pôde ser lido.".to_owned()];
    };
    let domain = |key: &str| -> Option<String> {
        let addresses = addrparse_header(headers.get_first_header(key)?).ok()?;
        if let [MailAddr::Single(address)] = addresses.as_slice() {
            let raw = address.addr.rsplit_once('@')?.1;
            if let Ok(url::Host::Domain(normalized)) = url::Host::parse(raw) {
                return Some(normalized.trim_end_matches('.').to_owned());
            }
        }
        None
    };
    let mut findings = Vec::new();
    match (domain("From"), domain("Reply-To")) {
        (Some(sender), Some(reply)) if sender != reply => findings.push(format!(
            "O domínio de resposta ({reply}) difere do remetente ({sender})."
        )),
        (None, _) => findings.push("O domínio do remetente não pôde ser identificado.".to_owned()),
        _ => {}
    }
    let body = String::from_utf8_lossy(&prefix[header_end..]);
    let mut hosts = BTreeSet::new();
    let mut flagged = BTreeSet::new();
    for candidate in body
        .split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '(' | ')'))
        .filter_map(|token| {
            let start = token.find("https://").or_else(|| token.find("http://"))?;
            crate::links::review(token[start..].trim_end_matches([',', '.', ';']), "").ok()
        })
        .take(200)
    {
        if !candidate.findings.is_empty() {
            flagged.insert(candidate.host.clone());
        }
        hosts.insert(candidate.host);
    }
    if !hosts.is_empty() {
        findings.push(format!(
            "Domínios visíveis: {}.",
            hosts
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !flagged.is_empty() {
        findings.push(format!(
            "Domínios com sinais para revisar: {}.",
            flagged
                .iter()
                .take(12)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if truncated || hosts.len() > 12 {
        findings.push(
            "A revisão foi limitada ao início da mensagem e a uma lista curta de domínios."
                .to_owned(),
        );
    }
    findings.push("A análise usa cabeçalhos e links visíveis. Partes codificadas, anexos e autenticidade do remetente exigem verificação adicional.".to_owned());
    findings
}
