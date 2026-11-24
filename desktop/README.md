# Central de diagnóstico para Windows

O aplicativo é um complemento local gratuito. Abra `central-desktop.exe` e selecione **Verificar meu computador**. A checagem reúne as configurações de proteção que o Windows informa e dados de memória e disco. Use **Detalhes da verificação** para ver as evidências e **Salvar relatório** para compartilhar o resultado com alguém de confiança.

**Ferramentas** inclui a conferência de links, a revisão de arquivos e a solicitação de varredura pelo Defender. O estado do Defender depende da configuração da máquina. Se ele estiver em modo passivo, confira qual antivírus está ativo na Segurança do Windows.

**Conferir destino Bitcoin** valida a rede principal e o checksum de dois endereços públicos usando rust-bitcoin. A comparação é offline e não solicita chave privada ou frase de recuperação. A conferência de valor, identidade e transação continua na carteira.

**Backup** exige o executável restic disponível no `PATH`. A explicação opcional por IA exige Ollama ativo no próprio computador, com o modelo `gemma4:e2b` instalado. As verificações básicas funcionam sem esses componentes, sem Node.js e sem conta web.

## Compilar

Na pasta `central-diagnostico`, com Rust e ferramentas de compilação instaladas:

```powershell
cargo build --release --locked --manifest-path desktop/Cargo.toml
```

O executável sai em `desktop/target/release/central-desktop.exe`. Esse build sai sem assinatura e pode ser bloqueado pelo Smart App Control. Para testar com as proteções do Windows ativas, gere a cópia assinada com `Sign-WindowsRelease.ps1` e siga os testes descritos em [Segurança e distribuição](../SEGURANCA.md#assinar-para-download-direto).
