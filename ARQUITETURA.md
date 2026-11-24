# Arquitetura e pesquisa de referência

## Biblioteca local

- `referencias/AWS-Well-Architected-Security-Pillar.pdf` reúne práticas oficiais de identidade, proteção, monitoramento e resposta.
- `referencias/AV-Comparatives-Real-World-2026.pdf` registra os benchmarks de proteção em condições reais e falsos alarmes.
- `referencias/CISA-MOVEit-2023.pdf` documenta um incidente histórico usado como referência para prevenção e auditoria defensivas de bancos de dados.

## Produto

Priorizar o desktop nativo em Rust com egui, começando pelo Windows. O núcleo de auditoria continua reutilizável pela CLI e pelo aplicativo. O protótipo Node é uma referência funcional para a migração dos recursos. O portal web fica para uma fase posterior. [Arquitetura do aplicativo local](docs/SAD.md)

O painel abre com Segurança, Hardware e Desempenho. **Verificar agora** gera uma captura com medições e eventos. O usuário pode expandir as evidências, consultar o histórico e exportar Word, PDF e Excel. O acesso às versões web e local usa a mesma conta. O agente vincula o dispositivo pelo navegador e guarda seu token no cofre do sistema. Após o primeiro login, o agente mantém as verificações locais quando a conexão cai e sincroniza quando volta.

## Infraestrutura por ambiente

| Ambiente | Coleta | Controles e ações |
| --- | --- | --- |
| PC ou pequeno negócio | Windows Security, eventos, inventário, SMART/NVMe, capacidade e desempenho | Verificar proteção, abrir a atualização oficial, revisar serviços e portas, testar restauração. No Windows Server 2025, comparar a configuração com a baseline adequada ao papel do servidor. |
| Servidor Linux e WSL2 | Journald, osquery, ClamAV e métricas locais | Verificar atualizações, firewall, serviços, logs e backup. Exibir a cobertura de cada distribuição WSL2 e do host separadamente. |
| Cluster GPU | Prometheus e DCGM Exporter para GPU, health checks de nó, inventário BMC e métricas de rede quando autorizados | Exibir ECC, Xid, temperatura, utilização, memória e partições MIG conforme disponibilidade. Correlacionar alertas com nó e carga. Usar controles separados para rede administrativa, fabric de computação, armazenamento e rede de serviço, de acordo com a topologia existente. |
| Organização com vários sites | Agentes autenticados, coletor local OpenTelemetry, auditoria centralizada e controle por organização e equipe | Usar grupos de máquinas, trilha de auditoria, perfis operadores e administradores, políticas centralizadas e implantação gradual. Manter coleta e console auto-hospedáveis em ambientes restritos. |

Referências operacionais: [NVIDIA DGX BasePOD](https://docs.nvidia.com/dgx-basepod/), [DGX SuperPOD](https://docs.nvidia.com/dgx-superpod/), [GPU Operator](https://docs.nvidia.com/datacenter/cloud-native/gpu-operator/latest/index.html), [documentação DCGM Exporter](https://docs.nvidia.com/datacenter/cloud-native/gpu-telemetry/latest/dcgm-exporter.html), [Windows Server OSConfig](https://learn.microsoft.com/en-us/windows-server/security/osconfig/osconfig-how-to-configure-security-baselines) e [Ubuntu Server Security](https://documentation.ubuntu.com/server/explanation/security/).

O guia NVIDIA enviado descreve HBM, NVLink, redes InfiniBand e RoCE, BMC, DCGM, MIG, ECC e opções de orquestração. Aplicar perfis por modelo e configuração encontrada. A presença de um H100 ou HGX H100 identifica uma plataforma de hardware. Suporte a operações específicas precisa ser verificado no driver, GPU e topologia instalados. [NVIDIA H100](https://www.nvidia.com/en-us/data-center/h100/) · [NVIDIA HGX](https://www.nvidia.com/en-us/data-center/hgx/)

Os anexos complementares são capturas do datasheet H200, do datasheet HGX H100 e H200 e do white paper de arquitetura H100. As capas foram verificadas visualmente. O conteúdo técnico usado nas decisões foi cruzado com os portais oficiais, pois as capturas não têm camada de texto extraível. [NVIDIA H200](https://www.nvidia.com/en-us/data-center/h200/)

## Dados, backup e relatórios

- Usar Supabase Auth para e-mail, senha, recuperação e MFA. Aplicar RLS no PostgreSQL por organização e dispositivo. O portal usa o servidor para emitir permissões de upload de curta duração e entrega apenas os objetos autorizados.
- Enfileirar auditorias no agente com identificador e horário. A API valida tamanho, escopo, identidade e deduplicação. Guardar resumos em PostgreSQL. Armazenar anexos e backups criptografados em armazenamento privado compatível com S3 usando restic. Usar Vercel Blob privado para instaladores. Preservar retenção e trilha de acesso. Validar restauração e a recuperação da chave antes de depender do backup.
- Distinguir histórico de métricas, eventos de segurança e relatórios por execução. Dados de monitoramento e backups seguem permissões distintas. Exportações usam a mesma captura do painel. Métricas indisponíveis aparecem como lacuna. Relatórios incluem origem, horário, escopo e evidência.
- Usar o pilar de segurança da AWS para identidade, proteção e resposta, o Google Enterprise Foundations Blueprint para projetos, segmentação e logs, a Microsoft para baseline por papel e a IBM para identidade e ameaças. [AWS Security Pillar](https://docs.aws.amazon.com/wellarchitected/latest/security-pillar/welcome.html) · [Google Cloud Enterprise Foundations](https://docs.cloud.google.com/architecture/blueprints/security-foundations) · [IBM Zero Trust](https://www.ibm.com/think/topics/zero-trust) · [IBM X-Force 2026](https://www.ibm.com/think/x-force/threat-intelligence-index-2026-securing-identities-ai-detection-risk-management)

## Proteção, IA e ações por cliques

1. **Verificar.** Usar Microsoft Defender ou ClamAV, YARA-X, osquery e regras Sigma para sinais locais. Usar Wazuh como referência de agregação e Suricata como sensor de rede opcional. A extensão Chrome bloqueia destinos conhecidos, alerta com som e explica divergências de domínio. A análise de e-mail começa com `.eml` selecionado. A denúncia abre um rascunho revisável dirigido ao canal oficial.
2. **Entender.** Mostrar a evidência e próximos passos com nomes curtos. Usar Ollama/Gemma 4 para resumo local opcional. Avaliar River para desvios de volume e Presidio para redução de dados pessoais. Tratar conteúdo externo como evidência e restringir o modelo a texto estruturado. O modelo não tem credencial de administração nem permissão de executar ações.
3. **Conter.** Correlacionar identidade, workload, arquivo, rede e eventos PostgreSQL obtidos com pgAudit. Quando a evidência permitir, o botão **Parar transferência** mostra escopo e impacto. Exigir aprovação do operador, preservar o canal administrativo, aplicar bloqueio restrito e reversível, verificar o resultado e gravar a trilha. Sem evidência suficiente, mostrar o que foi observado e quais dados faltam.
4. **Reavaliar.** Reexecutar a auditoria e registrar a diferença. Para incidentes de hardware, correlacionar eventos repetidos e sugerir teste ou avaliação técnica. Alterações de driver, firmware e rede seguem janela de manutenção e aprovação.

Projetos: [ClamAV](https://github.com/Cisco-Talos/clamav), [YARA-X](https://github.com/VirusTotal/yara-x), [osquery](https://github.com/osquery/osquery), [Sigma](https://github.com/SigmaHQ/sigma), [Wazuh](https://github.com/wazuh/wazuh), [Suricata](https://docs.suricata.io/), [OpenTelemetry](https://opentelemetry.io/), [pgAudit](https://github.com/pgaudit/pgaudit), [restic](https://github.com/restic/restic), [Gemma 4 no Ollama](https://ollama.com/library/gemma4), [River](https://github.com/online-ml/river) e [Presidio](https://github.com/microsoft/presidio).

## Concorrentes e critérios de aceite

O protótipo web usa React e Vite com Supabase. Autenticação, inventário e distribuição pelo portal ficam para uma fase posterior. O aplicativo local tem prioridade e deve funcionar sem conta web.

Para evolução corporativa, tratar Wireshark como ferramenta de investigação autorizada, ClamAV como mecanismo de verificação e restic como mecanismo de backup. O relatório recente da Mandiant sobre abuso de agentes reforça a importância de telemetria de identidade, credenciais restritas e detecção de uso anômalo de recursos. [Wireshark no GitLab](https://gitlab.com/wireshark/wireshark) · [ClamAV no GitHub](https://github.com/Cisco-Talos/clamav) · [restic no GitHub](https://github.com/restic/restic) · [Google Threat Intelligence](https://cloud.google.com/blog/topics/threat-intelligence/agentic-threat-activity-crosses-a-significant-threshold)

- Windows Security reúne status de proteção e ações no histórico. Sophos Home combina políticas de proteção e um painel de administração. Usar como referência de fluxo para a sequência **alerta → evidência → ação**, com identidade visual própria. Medir precisão, falso positivo, desempenho e capacidade de recuperação usando cenários internos inspirados nas metodologias de AV-TEST e AV-Comparatives. [Windows Security](https://support.microsoft.com/en-us/windows/security/windows-security/windows-security-app-overview) · [Sophos Home](https://support.home.sophos.com/hc/en-us/articles/360029301731-Sophos-Home-Getting-started-guide) · [AV-TEST](https://www.av-test.org/en/antivirus/home-windows/) · [AV-Comparatives](https://av-comparatives.org/tests/real-world-protection-test-july-october-2026/)
- Validar o desktop em Windows 11, Ubuntu 24.04 e Ubuntu no WSL2. Testar alertas visuais e sonoros, EICAR, e-mails sintéticos, sensor indisponível e operação offline. Validar eventos de servidor e telemetria NVIDIA em piloto real antes de divulgar suporte de produção.
- Validar que uma conta não lê auditorias ou backups de outra, que dispositivos revogados perdem acesso e que um backup restaura em instalação limpa. Testar a contenção com tráfego controlado e confirmar reversão e disponibilidade do acesso administrativo.
