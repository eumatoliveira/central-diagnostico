# Arquitetura do aplicativo local

Sentinel é um nome provisório para a Central de diagnóstico. O primeiro produto é um complemento gratuito de segurança e suporte no Windows, com foco em pessoas que precisam de instruções simples. A implementação principal é desktop nativo em Rust. O portal está adiado.

## Componentes e fluxo

| Componente | Responsabilidade | Situação |
| --- | --- | --- |
| `desktop` | Janela nativa egui, operação em segundo plano e exibição de evidências | Auditoria de firewall e Defender no Windows, configuração do kernel no Linux, conferência offline de links e exportação de auditoria |
| `agente` | Biblioteca Rust de verificações e CLI que serializa as mesmas evidências | Compartilhado entre janela nativa e CLI |
| `relatorios` e `relatorio-pdf` | Exportação nativa da última auditoria para Word, Excel e PDF | Arquivos novos, sem sobrescrever destinos existentes. O relatório nativo usa os dados já migrados da auditoria. |
| `extension` | Bloqueio de domínios cadastrados e aviso no Chrome | Protótipo separado, validado em Chromium com perfil temporário |
| Protótipo local Node | Referência executável para relatórios, triagem, inventário e backup | Recursos serão migrados por prioridade para Rust |
| `portal` | Protótipo de login, inventário e importação de resumo | Adiado. Sem dependência para operar o desktop |

Fluxo nativo atual: usuário inicia auditoria → thread de trabalho chama a biblioteca Rust → a biblioteca consulta o sistema → a janela exibe o estado e a evidência. A interface não pede elevação para a auditoria. A autenticação, a nuvem e o navegador não participam do fluxo.

## Evolução e critérios

- Migrar verificação de arquivos e ações do Defender antes dos módulos de servidor. A auditoria do Defender e a triagem de links já usam o núcleo Rust. A análise de links interpreta o destino sem abrir a página ou enviar a URL.
- A opção **Explicar com IA local** envia contagens da auditoria e o estado categórico da proteção em tempo real ao Ollama em `127.0.0.1:11434`, com modelo `gemma4:e2b`. O cliente usa limites de tempo e tamanho, sem proxy ou redirecionamento. O texto é exibido como explicação e a avaliação continua derivada das verificações determinísticas. O teste usa um servidor simulado no loopback. A inferência com o modelo real ainda precisa de validação.
- A janela nativa usa restic para backup, conferência e restauração em repositórios locais. Os relatórios incluem as verificações da auditoria nativa, com memória RAM e espaço livre. Resta migrar o destino S3 e o inventário avançado do protótipo. O repositório e sua senha permanecem independentes de uma conta web.
- No primeiro pacote de distribuição, verificar instalação e remoção em Windows limpo, interface com teclado e leitor de tela, resultados sob usuário comum e assinatura do pacote. Linux nativo passa por validação própria. Consultar [segurança e distribuição](../SEGURANCA.md) antes da publicação.
