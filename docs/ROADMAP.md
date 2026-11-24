# Roteiro de engenharia do Sentinel

Este roteiro começa pelo aplicativo nativo Sentinel Personal em Rust no Windows. Navegador, arquivos e IA local orientativa formam o primeiro piloto, sem conta e sem componente web. O portal existente fica como protótipo para a expansão, junto com Linux e Business. Os marcos abaixo dependem de critérios de saída, sem representar datas de lançamento.

## Base disponível

| Componente | Evidência disponível | Limite atual |
| --- | --- | --- |
| Diagnóstico local | Consulta ao Firewall Windows e fluxo do agente Rust até relatório validados no Windows. Word, PDF e Excel gerados. | Resultados do protótipo demonstram o fluxo, não a eficácia antivírus. Distribuição e cobertura Linux nativa ainda precisam de validação específica. |
| Chrome | Teste Chromium em perfil temporário confirmou bloqueio de domínio de teste, aviso, fechamento de aba e desbloqueio após remoção. | Lista manual. Som sob demanda. Sem publicação na loja, reputação remota ou varredura integrada de downloads. |
| Portal | Build React e testes de isolamento com PostgreSQL em memória. Importação JSON valida agregados antes de gravar. | Implantação, autenticação Supabase real e fluxos de recuperação exigem ambiente configurado e testes ponta a ponta. |
| Verificações, backup e IA | Código contém integração sob demanda com Defender e ClamAV, utilitário restic e guia de Ollama. | Validar por sistema e instalação. Guia não comprova integração do modelo. S3 compatível depende de credenciais, repositório e teste de restauração. |

## Marcos entregáveis

| Ordem | Frente e entrega | Critério de saída |
| --- | --- | --- |
| P0.1 | **Personal Windows.** Migrar o fluxo do protótipo para o executável nativo em Rust. Consolidar painel, varredura rápida e arquivo selecionado usando interfaces suportadas. Mostrar estado do motor e histórico. | Testar em Windows limpo com arquivo de teste inofensivo, arquivo normal, mecanismo desabilitado e acesso negado. Confirmar coexistência com Defender, encerramento seguro de operação e relatório coerente. Executar sem navegador servindo a interface, conta ou conexão externa. |
| P0.2 | **Browser Guard.** Preservar regras manuais e evoluir para pacotes verificáveis. Melhorar o aviso e o fluxo para corrigir falso bloqueio. | Testar domínio de laboratório, redirecionamento, domínio permitido, pacote inválido e reversão. Operar offline com o último conjunto válido. Confirmar aviso pelo teclado, leitor de tela e com áudio silenciado. |
| P0.3 | **Assistente local.** Integrar Ollama atrás de consentimento, modelagem de dados mínima e resposta estruturada. | Validar casos com modelo indisponível, resposta inválida, conteúdo com instruções hostis e ausência de evidência. Comprovar que entradas não saem da máquina e que nenhuma resposta dispara ação. |
| P0.4 | **Piloto gratuito.** Preparar instalador Windows, assinatura, atualização verificável, telemetria opt-in e suporte. | Instalação, atualização, rollback e desinstalação reproduzíveis em um ambiente limpo. Registrar SBOM, proveniência e baseline de consumo. Concluir avaliação de falsos positivos e usabilidade antes de ampliar o público. |
| P1 | **Personal Linux e Cloud.** Validar Ubuntu LTS nativo e WSL2 separadamente. Evoluir portal, backup e restauração. | Comprovar permissões e limites do agente, duas contas reais isoladas, revogação, restauração em instalação limpa e política de retenção. A função local segue offline. |
| P2 | **Business e infraestrutura.** Pilotar papéis por organização, logs, telemetria de servidor, GPU com DCGM e auditoria PostgreSQL. | Testar origem e lacunas de eventos, isolamento organizacional e carga real. Validação de contenção em laboratório autorizado preserva o canal administrativo e permite reversão. |

## Frentes paralelas e dependências

- **Agente e verificação:** publicar o contrato versionado do diagnóstico. O desktop nativo em Rust interpreta e exporta um subconjunto permitido. O painel Node e o portal Vite atuais são referências do protótipo. O binário nativo deve funcionar sem Node, aplicação web empacotada ou login.
- **Extensão e acesso:** o navegador mantém lista e decisão local. Integrações futuras com o agente usam interface restrita e validação de origem. O portal mantém o fluxo de conta separado da verificação local.
- **Qualidade e lançamento:** preparar corpus de arquivos e páginas de teste inofensivos, cenários de regressão, baseline de desempenho e critérios de acessibilidade. Qualquer mudança em regras, modelos ou integrações deve passar por revisão de licenças, testes e recuperação.
- **Operação:** preparar documentos de arquitetura detalhada, ameaças e integração de projetos. O piloto deve informar qual versão foi testada e quais plataformas ficaram de fora. Usar o [guia de segurança e distribuição](../SEGURANCA.md) para acompanhar referências NSA/CISA, NIST, Windows e Ubuntu.
