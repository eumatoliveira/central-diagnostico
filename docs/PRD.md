# Sentinel Personal — requisitos de produto

**Nome de trabalho:** Sentinel, sujeito à validação de marca antes de publicação. **Primeira plataforma:** Windows 11, com aplicativo nativo em Rust. **Destinatários iniciais:** pessoas que querem entender a proteção do Windows e receber ajuda para verificar arquivos e links, incluindo pessoas idosas e quem as auxilia com autorização.

## Objetivo e experiência

O Sentinel Personal complementa o Microsoft Defender com verificações assistidas, proteção no navegador e explicações opcionais por IA local. O núcleo local é gratuito e funciona sem conta. A aplicação apresenta cada evidência, a origem da verificação, o que aconteceu e o próximo passo. O primeiro piloto inclui apenas o aplicativo nativo e a extensão local do Chrome. Portal, conta, sincronização e backup em nuvem ficam para expansão.

- **Primeira verificação.** O usuário identifica se a proteção nativa está ativa, degradada ou indisponível, verifica um arquivo e vê resultados com horário e origem. O sistema distingue observação, indício e detecção de um mecanismo reconhecido.
- **Navegação.** Ao encontrar um domínio bloqueado, o navegador mostra aviso visual com uma ação fácil de entender e acessar pelo teclado. O som é complementar. O usuário pode revisar as próprias regras e conferir um endereço oficial por canal conhecido.
- **Orientação.** O usuário pode explicar um resultado com IA local, após revisar quais dados serão usados. A resposta identifica incertezas e recomenda ações manuais seguras. A explicação padrão por regras permanece disponível quando o modelo estiver desativado ou indisponível.
- **Compartilhamento.** O usuário exporta um diagnóstico local e escolhe como compartilhá-lo com o suporte. A futura importação no portal requer confirmação dos dados que sairão do computador.

## Escopo dos 15 módulos

O quadro separa o que existe no código das entregas previstas. Ver [roteiro e evidências](ROADMAP.md) para o estado de validação.

| Módulo | Base atual | Próxima entrega ou expansão |
| --- | --- | --- |
| 01 Antivirus Core | Consulta e varredura rápida via Defender. | Personal. Estado confiável, histórico de execução, integração segura com ações nativas e cobertura validada com amostras inofensivas. |
| 02 AI Security Assistant | Guia de IA local. | Personal. Explicação de campos selecionados por Ollama opt-in, sem ferramentas de execução. |
| 03 Browser Guard | Extensão Chrome com lista manual, aviso visual e botão de som. | Personal. Regras atualizáveis e reversíveis, navegação segura e alertas acessíveis. Avaliar outros navegadores depois. |
| 04 File Scanner | Adaptador ClamAV invocado sob demanda em Linux e diagnóstico de triagem. | Personal. Seleção de arquivo no Windows, resultado do motor e indicação explícita de análise inconclusiva. Avaliar assinaturas, metadados e YARA por disponibilidade e evidência. |
| 05 Behavior Monitor | Auditorias pontuais e indícios de falha de aplicativos. | Expansão. Correlacionar eventos de processo e arquivo com baixo impacto, incluindo sinais de ransomware e persistência. |
| 06 Network Guard | Inventário local de rede e consulta de perfis do Firewall no Windows. | Expansão. Alertar sobre serviços expostos e validar recomendações por perfil. Remediação depende de política, autorização e reversão. |
| 07 Email Security | Triagem local de texto sem integração à caixa postal. | Expansão. Analisar arquivo escolhido pelo usuário ou integração específica autorizada. Exibir sinais de domínio, remetente e links. |
| 08 Data Protection | Base de triagem e comparação local de destinatário Bitcoin. | Expansão. Alertas defensivos para diretórios selecionados e exposição de credenciais por fontes autorizadas. |
| 09 Server Security | Guias de revisão e auditorias pontuais do host. | Business. Inventário, baselines, autorização administrativa, logs e telemetria NVIDIA em piloto real. |
| 10 Cloud Backup | Integração com restic para local e S3 compatível previamente configurado. | Cloud. Agendamento, retenção e recuperação com chave independente. Validar integridade e restauração em cada ambiente. |
| 11 Threat Intelligence | Lista de bloqueio manual no navegador. | Personal. Pacotes de regras verificáveis e aprovados. Feeds externos exigem revisão de termos, atualização e privacidade. |
| 12 Incident Response | Resultados locais e recomendações de revisão. | Personal. Histórico das verificações e encaminhamento a ações nativas. Contenção corporativa entra após testes de reversão. |
| 13 System Health | RAM, disco, diagnóstico de GPU, problemas de dispositivo e eventos de aplicativos no protótipo. | Personal. Diferenciar falha confirmada de possibilidade e informar limites dos sensores. Sem limpeza agressiva de RAM. |
| 14 Security Analytics | Relatórios locais e protótipo web de importação manual de resumos. | Personal. Tendências locais sem misturar relatórios ausentes com medições de valor zero. O painel web e a vinculação automática ficam para Cloud. |
| 15 Family & Business | Aviso de navegador com texto e ação principal, portal com inventário individual. | Família após validação de acessibilidade e consentimento. Business com papéis por organização, trilha de auditoria e políticas delegadas. |

## Requisitos de confiança e qualidade

- **Proteção complementar.** Preservar proteção nativa ativa, evitar drivers próprios na primeira versão e validar a coexistência de todos os componentes. Sem cobertura comprovada para rootkits, ataques sem arquivo ou prevenção integral de exfiltração.
- **Decisões rastreáveis.** Bloqueios usam política determinística e identificam a regra. A IA não tem credenciais administrativas, ferramentas de execução nem autoridade para determinar uma detecção. Conteúdo analisado é entrada não confiável. Consultas remotas dependem de adesão explícita.
- **Privilégio mínimo.** O agente local começa com permissões do usuário. Ações futuras com elevação são previamente definidas, mostram impacto, exigem a autorização adequada e registram a decisão. Operações automatizadas requerem política revisada e recuperação testada.
- **Privacidade e isolamento.** Dados permanecem locais por padrão. Na expansão Cloud, exigir confirmação para importar, limitar os campos enviados, isolar registros por conta e manter a senha de recuperação de backup separada da conta web.
- **Acessibilidade e desempenho.** Avisos legíveis por leitor de tela, via teclado e sem som. Medir RAM, CPU, disco e latência antes e depois da instalação. Registrar a máquina, a carga e a versão em cada teste.
- **Distribuição.** Preparar dependências travadas, SBOM, atualização verificável, assinatura de artefatos e canal de vulnerabilidades. Tratar certificações e aprovações como resultados externos separados da implementação.

**Métrica principal:** dispositivos protegidos e saudáveis, contados apenas quando há agente atualizado, evidência recente de proteção funcional e diagnóstico válido. Acompanhar taxa de verificações concluídas, falsos bloqueios em conjunto de teste, tempo de verificação, testes de restauração e compreensão de alertas em testes com usuários. A coleta de métricas respeita as escolhas de privacidade. Metas numéricas dependem da medição inicial e devem ser registradas antes do piloto.
