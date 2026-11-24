# Portal de dispositivos

Interface React com autenticação Supabase por e-mail e senha, recuperação por PKCE e cadastro manual de dispositivos isolados por conta. O inventário exibe os 50 registros mais recentes. O usuário importa manualmente um diagnóstico JSON por dispositivo e revisa os indicadores antes de salvar. A vinculação automática com o agente, sincronização contínua e backup exigem implementação própria.

## Preparar o ambiente

1. Aplique as duas migrações de `supabase/migrations` em ordem em um projeto Supabase destinado ao portal.
2. Habilite confirmação de e-mail no Supabase Auth e configure a entrega de mensagens. Cadastre a origem exata do portal como URL de redirecionamento. Para desenvolvimento, use `http://localhost:5173`.
3. Copie `.env.example` para `.env.local` e preencha a URL HTTPS do projeto e a chave **publishable**. A chave de administração deve permanecer no servidor e não deve usar prefixo `VITE_`.
4. Execute `npm ci` e `npm run dev`. Para validar a produção, execute `npm run build`. No Vercel, selecione a pasta `portal` como raiz do projeto e configure as mesmas variáveis públicas antes do build.

Mantenha a confirmação e a recuperação de senha no mesmo perfil de navegador que iniciou a solicitação. Valide com duas contas de teste que cada usuário só consegue listar seus próprios dispositivos e que requisições anônimas não têm acesso à tabela. Configure política de senha e limites de autenticação no provedor antes de disponibilizar registro público.

`npm test` valida as políticas com PostgreSQL em memória. O resumo de diagnóstico guarda horário, percentuais e contagens. Detalhes dos eventos permanecem no arquivo original. O portal exibe os 20 diagnósticos mais recentes de cada dispositivo. A origem do relatório manual e a identidade física do dispositivo dependem da escolha do usuário.
