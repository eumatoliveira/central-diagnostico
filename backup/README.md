# Backup criptografado

Instale [restic pelo canal oficial](https://restic.net/). Crie antes as pastas de repositório e de restauração. O repositório deve começar vazio e ficar em uma árvore separada da pasta que será copiada.

Na janela Rust, abra a aba **Backup**, escolha uma pasta de repositório e selecione a operação. Para recuperar uma cópia, liste o repositório, selecione **Restaurar cópia**, escolha a data e indique onde criar uma pasta nova. O desktop usa o restic instalado e funciona sem Node.js. A integração nativa usa repositórios em pasta local.

## Protótipo anterior e CLI Node.js

Para usar o protótipo anterior ou a CLI, tenha Node.js disponível e execute os comandos na pasta `central-diagnostico`.

No painel local, abra **Backup e restauração de arquivos**. Escolha uma operação e preencha o caminho e a senha solicitados. O painel envia a senha apenas ao processo local de backup e limpa o campo após o envio. Para restaurar, use **Listar cópias**, selecione a data desejada e informe a pasta de destino. O painel lista até 50 cópias.

```sh
node backup/restic.mjs iniciar <repositorio-vazio>
node backup/restic.mjs salvar <repositorio> <pasta-origem>
node backup/restic.mjs listar <repositorio>
node backup/restic.mjs verificar <repositorio>
node backup/restic.mjs restaurar <repositorio> <id-completo> <pasta-pai>
```

O restic pede a senha interativamente. Em automação, aceite o segredo por `RESTIC_PASSWORD` fornecido pelo ambiente da tarefa. Guarde a senha de recuperação fora do repositório. O comando `listar` retorna JSON, com o identificador completo em `id`. `verificar` lê o conteúdo dos dados e pode demorar. Cada restauração usa uma pasta nova. Se a execução falhar, essa pasta pode conter arquivos parciais.

Para backup local, use um disco separado quando precisar recuperar de falha do disco de origem.

No Windows, aceite caminhos locais como `C:\Backups` e coloque o caminho entre aspas quando tiver espaços. Novos backups capturam o conteúdo a partir da pasta escolhida. Após restaurar, o comando só anuncia sucesso quando o restic termina também a verificação dos arquivos. Backups antigos com caminhos completos continuam listáveis e restauráveis, mas o Windows pode negar a recriação de metadados de alguma pasta ancestral.

## Destino S3 opcional

Crie um bucket privado compatível com S3. Configure o ambiente do processo que executa a ferramenta:

- `CENTRAL_BACKUP_S3_REPOSITORY` recebe um destino fixo no formato `s3:https://host/bucket/prefixo`. Use um prefixo exclusivo para o repositório. O prefixo é opcional.
- `AWS_ACCESS_KEY_ID` e `AWS_SECRET_ACCESS_KEY` identificam a conta de serviço. Prefira credenciais restritas ao bucket e prefixo do repositório. Para credenciais temporárias, configure também `AWS_SESSION_TOKEN`. Se o provedor exigir região, configure `AWS_REGION` ou `AWS_DEFAULT_REGION`.
- A senha do repositório é separada das credenciais S3. O restic usa essa senha para criptografar os dados antes do envio. Use um meio seguro para guardar as credenciais e a senha.

Substitua o caminho do repositório por `@s3` nos comandos da CLI. Faça a inicialização uma vez e valide a restauração antes de usar dados importantes. A operação `verificar` lê todo o conteúdo e pode consumir tráfego do provedor. O destino S3 fica associado ao ambiente do processo local. [Documentação do restic para armazenamentos compatíveis com S3](https://restic.readthedocs.io/en/stable/030_preparing_a_new_repo.html#amazon-s3)

No protótipo, a opção **Armazenamento remoto configurado** aparece quando o servidor local detecta uma configuração válida. O destino e as credenciais do bucket ficam no processo local. O envio de uma pasta exige confirmação. Esta opção ainda precisa ser migrada ao desktop Rust e validada com um bucket real.
