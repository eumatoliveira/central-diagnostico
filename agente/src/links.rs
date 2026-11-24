use url::{Host, Url};

pub struct LinkReview {
    pub host: String,
    pub findings: Vec<&'static str>,
}

pub fn review(input: &str, trusted_domain: &str) -> Result<LinkReview, &'static str> {
    let input = input.trim();
    if input.len() > 4096
        || input
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return Err("Cole um único link completo, sem espaços.");
    }
    let (scheme, authority) = input
        .split_once("://")
        .ok_or("Inclua https:// ou http:// no começo do link.")?;
    if !(scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
        || authority.starts_with('/')
    {
        return Err("Informe um link web com http:// ou https://.");
    }
    let address = Url::parse(input).map_err(|_| "Não foi possível ler este endereço.")?;
    let host = address
        .host_str()
        .ok_or("O endereço não contém um destino.")?
        .trim_end_matches('.');
    let mut findings = Vec::new();
    let trusted = trusted_domain.trim();
    if !trusted.is_empty() {
        if trusted.len() > 253
            || trusted
                .chars()
                .any(|c| !(c.is_alphanumeric() || matches!(c, '.' | '-')))
        {
            return Err("Informe só o domínio conhecido, como exemplo.com.br.");
        }
        let normalized = Host::parse(trusted).map_err(|_| "O domínio conhecido não é válido.")?;
        let Host::Domain(domain) = normalized else {
            return Err("Informe um domínio conhecido, em vez de um endereço IP.");
        };
        let domain = domain.trim_end_matches('.');
        if !domain.contains('.')
            || domain.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
            })
        {
            return Err("O domínio conhecido não é válido.");
        }
        let same_or_subdomain = host == domain
            || host
                .strip_suffix(domain)
                .is_some_and(|prefix| prefix.ends_with('.'));
        if !same_or_subdomain {
            findings.push("O endereço está fora do domínio conhecido informado.");
        }
    }
    if address.scheme() == "http" {
        findings.push("A conexão usa HTTP, sem a proteção de HTTPS.");
    }
    if authority
        .split(['/', '?', '#'])
        .next()
        .is_some_and(|value| value.contains('@'))
    {
        findings.push("Um trecho antes do domínio pode confundir quem lê o endereço.");
    }
    if matches!(address.host(), Some(Host::Ipv4(_) | Host::Ipv6(_))) {
        findings.push("O endereço aponta para um número de rede.");
    }
    if host.split('.').any(|label| label.starts_with("xn--")) {
        findings.push("O domínio tem caracteres internacionalizados que podem parecer letras de outro endereço.");
    }
    if address.port().is_some() {
        findings.push("O endereço usa uma porta fora do padrão.");
    }
    Ok(LinkReview {
        host: host.to_owned(),
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::review;

    #[test]
    fn matches_only_exact_domain_and_label_boundaries() {
        assert!(
            review("https://conta.banco.test", "BANCO.TEST.")
                .unwrap()
                .findings
                .is_empty()
        );
        assert_eq!(
            review("https://banco.test.outro.test", "banco.test")
                .unwrap()
                .findings
                .len(),
            1
        );
        assert_eq!(
            review("https://falsobanco.test", "banco.test")
                .unwrap()
                .findings
                .len(),
            1
        );
    }

    #[test]
    fn reports_actual_destination_and_disguises() {
        let result = review("http://banco.test@127.1:8080", "banco.test").unwrap();
        assert_eq!(result.host, "127.0.0.1");
        assert_eq!(result.findings.len(), 5);
        assert_eq!(
            review("https://éxemplo.test", "").unwrap().findings.len(),
            1
        );
    }

    #[test]
    fn rejects_unexpected_schemes_paths_and_reference_urls() {
        assert!(review("javascript:alert(1)", "").is_err());
        assert!(review("https:///banco.test", "").is_err());
        assert!(review("https://banco.test", "https://banco.test").is_err());
    }
}
