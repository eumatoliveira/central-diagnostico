# Proteção no Chrome

Em `chrome://extensions`, ative o Modo do desenvolvedor, selecione **Carregar sem compactação** e abra esta pasta. No ícone da extensão, informe um domínio por linha e pressione **Salvar lista**. Por exemplo, adicione `exemplo.test` e, em outra guia, acesse `https://exemplo.test/` para verificar o aviso. Para remover o bloqueio, apague o domínio da lista e salve novamente.

As regras dinâmicas ficam no perfil do Chrome. Cada regra cobre o domínio e seus subdomínios na navegação principal. O aviso é uma página local da extensão e não recebe o endereço solicitado. A permissão de hosts HTTP e HTTPS permite aplicar o redirecionamento aos domínios escolhidos. Esta versão usa lista manual e alerta visual.

Em **Conferir destinatário Bitcoin**, cole separadamente o endereço público obtido por um canal confiável e o exibido na carteira. A comparação acontece localmente e avisa quando os endereços divergem. Ela verifica a forma básica e compara o conteúdo, sem validar checksum, titularidade ou transação.

## Validação

Em um perfil temporário do Chromium com a extensão instalada sem compactação, o teste cadastrou um domínio de laboratório, confirmou o redirecionamento para o aviso local, fechou a guia pelo botão da extensão e verificou que o servidor de teste voltou a carregar após remover o domínio. O aviso também oferece um botão para tocar o som sob demanda.

[Documentação do Chrome sobre regras declarativas](https://developer.chrome.com/docs/extensions/reference/api/declarativeNetRequest)
