# Contribuindo com o Pier

<p>
  <a href="CONTRIBUTING.md">English</a> · <b>Português</b>
</p>

Obrigado por ajudar o Pier. Este guia explica como compilar o Pier a partir do código, rodar os testes e enviar um pull request.

Para reportar um bug ou sugerir um recurso, [abra uma issue](https://github.com/pier-dock/pier-dock/issues/new/choose) e escolha o modelo correspondente.

## Estrutura do projeto

- `src/`: o frontend em React (o dock, os painéis dos widgets e a janela de configurações).
- `src-tauri/`: o backend em Rust, feito com [Tauri 2](https://tauri.app), com o código de cada plataforma para Windows, macOS e Linux.
- `gnome-extension/pier@pier.dev/`: a extensão do GNOME Shell que o Pier instala no GNOME.
- `scripts/`: scripts auxiliares para gerar os ícones e preparar uma versão.

## Ambiente de desenvolvimento

### Pré-requisitos

- [Node.js](https://nodejs.org) 22 ou mais recente
- [Rust](https://rustup.rs) (stable)
- Ferramentas de cada plataforma:
  - **Windows:** Visual Studio Build Tools com a carga de trabalho de C++ e o Windows SDK. Use o toolchain `stable-x86_64-pc-windows-msvc` do Rust.
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`).
  - **Linux:** `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` e `patchelf`.

### Rodar e compilar

```sh
npm install
npm run tauri dev     # roda em modo de desenvolvimento
npm run tauri build   # gera os instaladores em src-tauri/target/release/bundle/
```

Quando você roda `npm run tauri dev` no macOS, o sistema concede as permissões ao terminal que rodou o comando, e não ao Pier.

### Pré-visualização no navegador

Fora do Tauri, o `npm run dev` responde as chamadas de IPC com dados de exemplo, para você conferir a interface no navegador. Abra `http://localhost:1420` com um destes parâmetros:

- `?view=dock`
- `?view=panel&kind=media` (ou `github-mine`, `github-review`, `github-contrib`, `volume`, `network`)
- `?view=settings`

### Testes

```sh
npm test                               # testes do frontend
cd src-tauri
cargo test                             # testes unitários do Rust
cargo test -- --ignored --nocapture    # smoke tests contra o sistema real
```

Os smoke tests usam janelas, apps, sessões de mídia e a API do GitHub de verdade, na máquina em que rodam.

### Ícones

Os ícones de todas as plataformas saem de `app-icon.svg` e `src-tauri/icons/source/`. Para gerá-los de novo, rode `./scripts/icons.sh` no macOS (o script precisa do `iconutil`).

## Pull requests

- Mantenha cada pull request pequeno e focado em uma mudança.
- Use um título no estilo [Conventional Commits](https://www.conventionalcommits.org/pt-br), `tipo(escopo): resumo`, por exemplo `feat(dock): …` ou `fix(windows): …`, em inglês. Os pull requests entram com squash, então o título vira a mensagem do commit.
- O CI roda `npm run build` e `npm test`, e `cargo test --locked` no Windows, no macOS e no Linux. Rode tudo localmente antes de abrir o pull request.
- Se a mudança aparece para o usuário, atualize o `README.md` e o `README.pt-BR.md`.

## Publicando uma versão

Esta seção é para quem mantém o projeto. Para publicar uma versão nova:

1. Adicione uma seção `## x.y.z` com as novidades no `CHANGELOG.md` e no `CHANGELOG.pt-BR.md`. O Pier mostra essas notas depois de atualizar, então escreva pensando no usuário. O workflow **Release** falha se faltar a seção em qualquer um dos dois arquivos.
2. Rode `./scripts/release.sh x.y.z`. O script atualiza a versão no `package.json` e no `Cargo.toml`, faz o commit e cria a tag `vx.y.z`.
3. Rode `git push --follow-tags`.
4. Espere o workflow **Release** terminar. Ele compila o Pier no macOS, no Windows e no Linux e cria um release rascunho com os instaladores e o `latest.json` do atualizador.
5. Confira o rascunho e publique. As cópias instaladas do Pier só enxergam a versão nova depois da publicação.

Para testar o build sem criar um release, rode o workflow à mão em **Actions** > **Release** > **Run workflow**. Os instaladores ficam anexados à execução como artefatos.

O workflow assina os pacotes de atualização com a chave do atualizador guardada nos secrets `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` do repositório. A chave pública correspondente está em `src-tauri/tauri.conf.json`. Guarde uma cópia da chave privada: se ela for perdida, as cópias instaladas do Pier recusam as próximas atualizações.

Para assinar e notarizar o app do macOS no futuro, adicione os secrets `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` e `APPLE_TEAM_ID`, passe-os no `env` do passo `tauri-action` e remova o `"signingIdentity": "-"` de `src-tauri/tauri.macos.conf.json`.

## Licença

Ao contribuir, você concorda que suas contribuições sejam licenciadas sob a [Licença Pública Geral GNU, versão 3 ou posterior](LICENSE), a mesma licença do Pier.
