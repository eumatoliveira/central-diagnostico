# Segurança e distribuição

## Controles implementados nesta etapa

- Agente Rust com uma operação de leitura, `auditar`. Windows consulta a política efetiva dos perfis de firewall. Linux consulta o nível de ASLR do kernel. O diagnóstico mostra separadamente controles ativos, configurações a revisar e verificações indisponíveis.
- O protótipo chama um executável por caminho fixo com argumentos fixos e limita tempo e saída. A API local escuta apenas em IPv4 loopback e valida `Host`, `Origin` e token por processo nas operações de diagnóstico e exportação. O protótipo é executado sob a conta do usuário.
- O diagnóstico preserva a exportação de resultados e recomendações em Word, PDF, Excel e texto. Nenhuma ação de remediação é aplicada pelo agente atual.

## Referências e critérios para lançamento

| Referência | Aplicação no produto | Evidência necessária para lançar |
| --- | --- | --- |
| [NSA/CISA sobre erros de configuração](https://www.nsa.gov/Press-Room/Press-Releases-Statements/Press-Release-View/Article/3549369/nsa-and-cisa-advise-on-top-ten-cybersecurity-misconfigurations/) | Separar operação comum, ações administrativas e revisão de conectividade. Planejar MFA, inventário, auditoria central e restrições de execução. | Testes de segregação, revisão por perfil de servidor, status de cada verificação e evidências de aprovação e reversão nas ações de alto impacto. |
| [NIST SSDF SP 800-218](https://csrc.nist.gov/pubs/sp/800/218/final) | Usar dependências travadas, repositório com revisão e modelo de ameaças, testes de segurança, SBOM, assinaturas e canal de reporte. | CI em Windows e Linux, auditoria de dependências Rust e JS, SBOM da distribuição, validação de proveniência e procedimento de correção de vulnerabilidades. |
| [Assinatura no Windows](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) | Avaliar MSIX com certificação Microsoft Store para desktop. Para downloads diretos, usar Authenticode com assinatura pública e timestamp. Tratar o agente de servidor como pacote separado. | Instalar e desinstalar em máquina limpa, validar assinatura e UAC, verificar compatibilidade do pacote e submeter à certificação aplicável antes de alegar aprovação. |
| [Repositórios de terceiros no Ubuntu](https://documentation.ubuntu.com/server/explanation/software/third-party-repository-usage/) | Empacotar para Ubuntu LTS e distribuir por repositório APT com metadados assinados e chave restrita à origem. | Validar cadeia de confiança do repositório, chave por origem, atualização, rollback e desinstalação em Ubuntu nativo e WSL2. |

As referências da NSA/CISA e do NIST orientam a engenharia. Certificação da loja, assinatura de distribuição e conformidade com exigências de um cliente são verificações distintas. O produto pode ser gratuito para o usuário mesmo que publicação, assinatura ou hospedagem tenham custo operacional.

## Critérios de cada versão para Windows

1. Manter o desktop como complemento da Segurança do Windows. Consultar o estado do Microsoft Defender e solicitar varreduras por ação do usuário. Preservar a proteção em tempo real, as exclusões e as políticas vigentes. Gemma e Ollama são opcionais. A instalação do Ollama começa pelo site oficial.
2. Assinar os artefatos de distribuição com um certificado de assinatura de código emitido por um fornecedor confiável para o Smart App Control. Antes de publicar, verificar a cadeia da assinatura, o editor esperado e o carimbo de tempo de todos os artefatos. [Requisitos do Smart App Control](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control)

3. Validar instalação, atualização, desinstalação, abertura pelo atalho e solicitações ao Defender em um Windows limpo com Smart App Control e as proteções padrão ativas. Exercitar também o modo passivo para confirmar que a interface orienta o usuário a consultar o antivírus ativo.
4. Para o canal Microsoft Store, guardar os resultados dos testes de certificação aplicáveis ao pacote escolhido e a aprovação no Partner Center. A Store assina pacotes MSIX após certificação. Instaladores EXE e MSI submetidos à Store precisam chegar assinados pelo editor. [Opções de assinatura e distribuição](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) · [Windows App Certification Kit](https://learn.microsoft.com/en-us/windows/uwp/debug-test-perf/windows-app-certification-kit)

### Assinar para download direto

Com o certificado autorizado do projeto disponível em `Cert:\CurrentUser\My` e a chave acessível pelo provedor, configure `$publisherThumbprint` com a impressão digital desse certificado. A impressão digital identifica o certificado. A chave privada permanece no provedor de assinatura. Se o certificado estiver em `Cert:\LocalMachine\My`, passe `-CertificateStore LocalMachine`.

Execute na pasta `central-diagnostico`:

```powershell
cargo build --release --locked --manifest-path desktop/Cargo.toml
if ($LASTEXITCODE -ne 0) { throw 'Falha ao compilar o release.' }

$exe = 'desktop/target/release/central-desktop.exe'
$signedExe = 'desktop/target/release/central-desktop-signed.exe'
./desktop/Sign-WindowsRelease.ps1 -Artifact $exe -OutputPath $signedExe -PublisherThumbprint $publisherThumbprint
```

O script usa o SignTool do Windows SDK para gravar a assinatura Authenticode e o carimbo RFC 3161 com SHA-256. Ele publica a cópia de saída quando o verificador confirma a assinatura e retorna seu hash SHA-256. O arquivo de origem e qualquer arquivo existente no destino são preservados. Se o SDK estiver fora do caminho padrão, passe `-SignTool` com o caminho do executável oficial. [SignTool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool)

Se o log `Microsoft-Windows-CodeIntegrity/Operational` mostrar que a política bloqueou uma DLL de compilação do Rust, gere o release em um ambiente de build Windows autorizado para essa toolchain. Use um Windows com o Smart App Control ativo para testar o executável final assinado. [Smart App Control](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/smart-app-control)

Se o lançamento incluir um instalador, assine também o instalador e os binários distribuídos. Verifique todos os arquivos finais antes de disponibilizá-los. Para uma assinatura feita por um serviço externo, use o mesmo verificador com a impressão digital esperada para aquela versão.

```powershell
./desktop/Verify-WindowsRelease.ps1 -Artifact @($signedExe, $signedInstaller) -PublisherThumbprint $publisherThumbprint -SignTool $signToolPath
```

## Executar a etapa local

O caminho do produto é `cargo run --locked --manifest-path desktop/Cargo.toml`, usando a interface nativa Rust. A compilação mais recente passou no Windows. Um teste de automação local encontrou a janela, acionou **Verificar meu computador** e confirmou a exibição dos resultados. A revisão de arquivos foi verificada na CLI por comparação com o SHA-256 calculado pelo Windows. Os comandos abaixo executam o protótipo anterior para comparar funcionalidades durante a migração.

No diretório `central-diagnostico`:

```sh
npm ci
cargo build --locked --manifest-path agente/Cargo.toml
node server.mjs
```

Abra `http://127.0.0.1:47831` e execute o diagnóstico. Para testar apenas o agente, use `cargo run --locked --manifest-path agente/Cargo.toml -- auditar`.

Em Ubuntu 24.04 via WSL2, o agente foi compilado com Rust 1.91.1 e executado junto do painel em uma cópia temporária. O teste verificou a presença das quatro consultas do kernel no diagnóstico e a geração de DOCX, PDF e XLSX. A cobertura observada é a do kernel da distribuição WSL2.

## Componentes em desenvolvimento

- A seção **Revisar arquivo ou e-mail** calcula SHA-256 no navegador e mostra sinais básicos para arquivos executáveis e mensagens `.eml`. A leitura ocorre localmente e a triagem não emite um veredito de malware.
- A pasta `extension` contém a extensão Chrome para bloquear domínios cadastrados pelo usuário e as instruções para instalação de desenvolvimento.
- A pasta `portal` contém a base web para conta e dispositivos. A configuração do provedor e a ligação do agente devem ser verificadas antes de publicar.
- A varredura rápida chama o Microsoft Defender com um tipo de verificação fixo e confirmação local. O painel encaminha a pessoa ao Histórico de proteção para consultar detecções.
- O diagnóstico de rede consulta o próprio host com ferramentas fixas de Windows e Linux. A saída fica na tela local e pode conter endereços privados. Os guias para NVIDIA Cumulus e IBM Z são orientações para operadores, sem conexão a equipamentos remotos.
- A pasta `backup` contém instruções para criar, verificar e restaurar um repositório local criptografado com restic. O portal aceita a importação manual do JSON do diagnóstico e guarda apenas o resumo validado.
- No Linux, a verificação opcional de um arquivo exige `clamscan` no caminho `/usr/bin/clamscan` e assinaturas locais provisionadas. O arquivo é entregue ao processo pela entrada padrão via servidor local. A interface limita o envio a 24 MiB. Configure as assinaturas seguindo o fornecedor. [ClamAV](https://docs.clamav.net/manual/Usage/Scanning.html)

Em Ubuntu 24.04 no WSL2, o adaptador foi validado com o executável ClamAV do pacote Ubuntu extraído em pasta temporária e um banco de assinaturas sintético. O teste cobriu uma entrada sem detecção, uma detecção conhecida de texto de teste e a falha por banco ausente. A instalação padrão com a base oficial e o fluxo completo no navegador Linux ainda precisam de validação.
- O painel Bitcoin compara endereços públicos da rede principal e verifica o checksum localmente. A extensão oferece comparação e verificação de forma básica. Nenhum dos fluxos solicita frases de recuperação, assina transações ou consulta saldo. [BIP 173](https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki) · [BIP 350](https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki)
