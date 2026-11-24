# Central de Diagnóstico

Aplicativo desktop em Rust para diagnóstico local, revisão de links e arquivos, monitoramento de recursos e suporte mediado por IA local. No Windows, integra verificações sob demanda com o Microsoft Defender.

- [Compilar e abrir o desktop](desktop/README.md)
- [Segurança, assinatura e distribuição](SEGURANCA.md)
- [Arquitetura do projeto](ARQUITETURA.md)

O chat usa Ollama quando disponível. A saída do diagnóstico é somente leitura, e o terminal interativo é aberto para uso manual. O projeto está em desenvolvimento e a proteção contra ações de agentes de IA ainda não foi validada de ponta a ponta.
