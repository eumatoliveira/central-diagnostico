fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let [action, confirmation, path] = args.as_slice()
        && action == "varredura-arquivo"
        && confirmation == "--confirmar"
    {
        println!(
            "{}",
            central_agente::scan::scan_file(std::path::Path::new(path))?
        );
        return Ok(());
    }
    if let [action, path] = args.as_slice()
        && action == "revisar-arquivo"
    {
        let result = central_agente::files::inspect(std::path::Path::new(path))?;
        serde_json::to_writer(std::io::stdout().lock(), &result)?;
        return Ok(());
    }
    let checks = match args.as_slice() {
        [action, confirmation] if action == "varredura-rapida" && confirmation == "--confirmar" => {
            println!("{}", central_agente::scan::defender_quick_scan()?);
            return Ok(());
        }
        [action] if action == "auditar" => central_agente::audit(),
        [action] if action == "auditar-desktop" => central_agente::audit_desktop(),
        _ => {
            return Err(
                "Uso: central-agente auditar | auditar-desktop | revisar-arquivo <caminho> | varredura-rapida --confirmar".into(),
            );
        }
    };
    serde_json::to_writer(std::io::stdout().lock(), &checks)?;
    Ok(())
}
