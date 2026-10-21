<p align="center">
  <img src="app-icon.svg" width="96" height="96" alt="Logo do Pier">
</p>

<h1 align="center">Pier</h1>

<p align="center">
  Uma dock personalizável para Windows, macOS e Linux (GNOME e KDE Plasma).
</p>

<p align="center">
  <a href="https://github.com/pier-dock/pier-dock/releases/latest"><img src="https://img.shields.io/github/v/release/pier-dock/pier-dock?label=release" alt="Versão mais recente"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/pier-dock/pier-dock" alt="Licença: GPL-3.0"></a>
  <img src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-informational" alt="Plataformas: Windows, macOS e Linux">
</p>

<p align="center">
  <a href="README.md">English</a> · <b>Português</b>
</p>

O Pier substitui a barra de tarefas ou a dock do sistema por uma barra única com os seus apps fixados e abertos e widgets ao vivo. É feito com [Tauri 2](https://tauri.app) (Rust) e React, com vidro nativo no Windows 11, no macOS e nos compositores Linux que suportam desfoque.

## Recursos

- **Apps:** apps fixados e abertos, com ampliação, reordenação por arrastar, lista de janelas e separadores.
- **Barra personalizável:** coloque a dock na borda de baixo, de cima, da esquerda ou da direita da tela (nas laterais ela fica em pé), deixe-a no tamanho dos ícones ou de ponta a ponta e escolha o tamanho dos ícones, a altura do painel, o material e a cor de destaque.
- **Player de mídia:** mostra o que está tocando no Spotify, no YouTube Music e em outros players, com os controles de reprodução.
- **Widgets do GitHub:** pull requests aguardando a sua revisão, os seus pull requests abertos, commits recentes e o gráfico de contribuições.
- **Widgets do sistema:** rede (com as redes Wi-Fi por perto), volume (com a saída de áudio e o volume por app), Bluetooth (com os dispositivos pareados, que dá para conectar e desconectar no macOS e no Linux), bateria, relógio, mostrar a área de trabalho (minimiza todas as janelas com um clique), uso do sistema (CPU e memória com o histórico do último minuto) e, no Windows, a seta da bandeja que abre os ícones ocultos.
- **Widgets de produtividade:** clima, timer Pomodoro, o próximo compromisso da agenda, com botão para entrar na videochamada, e o uso das suas IAs de linha de comando.
- **Claude Code:** o widget Claude Agents mostra as suas sessões do Claude Code como em execução, pedindo permissão, com erro ou concluídas, e um clique leva ao terminal ou à IDE que a iniciou. O Clawd, mascote do Claude, pode andar pela dock enquanto uma sessão roda. Ele precisa dos hooks do Claude Code, que você ativa nas opções do widget.
- **Atalhos de teclado:** abra ou foque o app da posição 1 a 9 de qualquer lugar.
- **Atualização automática:** o Pier procura versões novas e as instala com um clique.
- **Português e inglês:** a interface segue o idioma do sistema. Para trocar, vá em **Configurações** > **Geral** > **Idioma**.

## Instalação

Baixe o instalador da sua plataforma na [versão mais recente](https://github.com/pier-dock/pier-dock/releases/latest).

| Plataforma | Arquivo | Observações |
| --- | --- | --- |
| Windows 10 e 11 | `Pier_x.y.z_x64-setup.exe` | Instala em `%LOCALAPPDATA%\Pier`, sem pedir administrador. |
| macOS 12 ou mais recente | `Pier_x.y.z_universal.dmg` | Roda nativamente em Macs com Apple Silicon e Intel. |
| Linux (GNOME 45 a 49, KDE Plasma 6) | `Pier_x.y.z_amd64.deb` ou `Pier_x.y.z_amd64.AppImage` | Instale o `.deb` com `sudo apt install ./Pier_x.y.z_amd64.deb`, ou torne o AppImage executável com `chmod +x` e rode-o. No KDE Plasma não é preciso mais nada; no GNOME, instale a extensão pelas configurações do Pier. |

### Primeira abertura

Os instaladores ainda não têm assinatura de código, então o sistema avisa na primeira vez que você abre o Pier.

- **Windows:** quando o SmartScreen aparecer, clique em **Mais informações** e depois em **Executar assim mesmo**.
- **macOS:** arraste o Pier para **Aplicativos** e abra. Quando o macOS disser que não pôde verificar o app, vá em **Ajustes do Sistema** > **Privacidade e Segurança** e clique em **Abrir Mesmo Assim**. Se preferir, rode `xattr -dr com.apple.quarantine /Applications/Pier.app` no Terminal.
- **Linux:** no GNOME, abra as configurações do Pier, clique em **Instalar extensão** e depois encerre a sessão e entre de novo. A extensão permite que o Pier veja e foque janelas, reserve espaço na tela, esconda o Dash, mostre a área de trabalho e registre os atalhos. No KDE Plasma não há nada para instalar: numa sessão X11 a dock lista as janelas, reserva o espaço na tela e aplica o blur do KWin.

### Atualizações

O Pier procura atualizações logo depois de abrir e a cada 12 horas. Quando houver uma versão nova, clique em **Atualizar para x.y.z** no menu da bandeja ou em **Instalar e reiniciar** em **Configurações** > **Geral**.

A atualização automática funciona com o instalador do Windows, com o app do macOS e com o AppImage. Se você instalou o pacote `.deb`, baixe e instale cada versão nova pela página de releases.

## Uso

- **Clique** num app para abri-lo ou focá-lo. Clique no app ativo para minimizá-lo. Se o app tiver várias janelas, cada clique alterna entre elas.
- **Shift+clique** num app abre uma nova instância.
- **Clique com o botão direito** num app para ver as janelas dele, fixá-lo, desafixá-lo ou fechá-lo.
- **Clique com o botão direito** no fundo da dock para abrir as configurações ou sair.
- **Arraste** os apps fixados para reordená-los. Para fixar um app, solte-o na dock (um `.exe` ou atalho no Windows, um `.app` no macOS ou um arquivo `.desktop` no Linux), ou adicione-o em **Configurações** > **Apps**.
- **Clique** num widget para abrir o painel dele. No widget de volume, use a roda do mouse para ajustar e o botão direito para silenciar; o painel dele tem a saída de áudio e o volume de cada app (no macOS, a partir do 14.2).

| Atalho | Windows e Linux | macOS |
| --- | --- | --- |
| Abrir ou focar o app da posição N | <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>1</kbd>…<kbd>9</kbd> | <kbd>Control</kbd>+<kbd>Option</kbd>+<kbd>1</kbd>…<kbd>9</kbd> |

### Widgets do GitHub

O Pier usa automaticamente o login do [GitHub CLI](https://cli.github.com) (`gh auth login`). Para usar outra conta, cole um personal access token com os escopos `repo` e `read:user` em **Configurações** > **GitHub**. O token tem prioridade sobre o GitHub CLI e fica guardado no chaveiro do sistema.

### Widgets de clima e agenda

- **Clima:** escolha a cidade em **Configurações** > **Widgets** > **Clima**. A previsão vem do [Open-Meteo](https://open-meteo.com) e não precisa de chave de API.
- **Agenda:** cole o endereço secreto iCal da sua agenda em **Configurações** > **Widgets** > **Agenda**. O Google Agenda, o Outlook e o iCloud publicam esse endereço. O Pier guarda o endereço no chaveiro do sistema.

### Widget de uso de IA

Um anel mostra quanto do limite da sua IA de linha de comando já foi usado — a mais apertada das duas janelas do Claude Code (a sessão de 5h e a semana) ou a janela do plano no Codex CLI — e o painel detalha cada uma com o tempo até o reset, mais os tokens do dia.

Ele lê o Claude Code e o Codex CLI, e **Configurações** > **Widgets** > **Uso de IA** escolhe qual delas: **Automática** (o padrão) usa a IA cuja leitura de limites é a mais recente; sem leitura em nenhuma, a que gastou tokens hoje. Não há nada para configurar e nada sai para a rede — o Pier lê arquivos que os próprios CLIs mantêm em disco:

- **Claude Code:** `~/.claude.json`, campo `cachedUsageUtilization`, para os percentuais de limite. É a mesma fonte do comando `/usage`, então os números batem com o terminal. Os tokens vêm de `~/.claude/projects/**/*.jsonl`, os transcripts das sessões.
- **Codex CLI:** `~/.codex/sessions/**/rollout-*.jsonl`. A linha `rate_limits` é a mesma fonte do comando `/status`, com as janelas `primary` e `secondary` do plano, e os tokens vêm do `last_token_usage` de cada turno.

Só os campos numéricos de `usage`, o nome do modelo e o timestamp são lidos — nunca o conteúdo das conversas, os comandos executados ou os diffs. Os CLIs só renovam os percentuais enquanto estão abertos, então com eles fechados o número congela: o widget esmaece e diz há quanto tempo foi a leitura em vez de passar um valor velho por atual.

## Particularidades de cada plataforma

### macOS

O Pier pede estas permissões:

- **Acessibilidade** (**Ajustes do Sistema** > **Privacidade e Segurança** > **Acessibilidade**), para listar, focar, minimizar e fechar janelas. Sem ela, o Pier ainda mostra os apps abertos e consegue focá-los, escondê-los e encerrá-los. As configurações do Pier mostram o botão **Conceder permissão** para pedi-la.
- **Automação**, na primeira vez que o Spotify ou o Música tocarem algo, para que o widget de mídia leia e controle esses apps por AppleScript.
- **Gravação de Tela** (macOS 14 ou mais novo), na primeira vez que as prévias das janelas abrirem, para que cada cartão mostre a miniatura da janela. Sem ela, os cartões mostram o ícone do app.
- **Gravação de Áudio do Sistema** (macOS 14.2 ou mais novo), na primeira vez que você mexer no controle de um app no painel de volume. O Pier passa o áudio desse app por ele para mudar o volume. Sem ela, o painel lista os apps mas não muda o volume deles, e mostra um botão que abre o ajuste.

Como o app ainda não é assinado com um Apple Developer ID, o macOS pede as permissões de Acessibilidade, de Gravação de Tela e de Gravação de Áudio do Sistema de novo a cada atualização.

O macOS tem algumas limitações em relação ao Windows:

- Os apps não podem reservar espaço na tela, então janelas maximizadas passam por baixo da dock. Para evitar isso, ative **Ocultar automaticamente** em **Configurações** > **Aparência**.
- O Dock nativo não pode ser removido. O Pier o deixa em ocultar automaticamente com um atraso muito longo para reaparecer e devolve as suas preferências originais ao sair.
- O widget de mídia só funciona com o Spotify e o Música.
- O widget de Wi-Fi só mostra o nome da rede e as redes por perto se o Pier tiver permissão de Localização. O painel dele pede a permissão.
- O widget de Bluetooth pede a permissão de Bluetooth na primeira vez, não liga nem desliga o Bluetooth (o macOS não permite) e não mostra a bateria dos dispositivos.

### Linux

O Pier tem suporte completo ao GNOME 45 a 49, em Wayland e X11, pela extensão do GNOME Shell. No KDE Plasma 6 a dock não precisa de extensão: numa sessão X11 ela lista as janelas, reserva o espaço na tela e desenha o vidro com o blur do KWin, e no Wayland lista as janelas dos apps X11 (as janelas dos apps nativos do Wayland ainda não aparecem). Em outros ambientes de desktop, o Pier só lista janelas em sessões X11 e não consegue esconder o painel nativo. Apps fixados e widgets funcionam em todos.

Para deixar a dock em outro monitor, ou fazer ela seguir o cursor, o GNOME precisa da versão 4 da extensão. Se **Configurações** > **Geral** > **Mostrar a dock em** oferecer a atualização, clique em **Atualizar a extensão** e depois encerre a sessão e entre de novo.

Rodando como AppImage, o Pier cadastra a própria entrada de menu e os ícones em `~/.local/share` na primeira execução, então a dock aparece no menu de aplicativos sem nenhum ajuste manual.

## Configuração

O Pier guarda as configurações em `config.json`:

- **Windows:** `%APPDATA%\pier\config.json`
- **macOS:** `~/Library/Application Support/pier/config.json`
- **Linux:** `~/.config/pier/config.json`

Se você usava o Pier quando ele se chamava WinDock, o Pier importa as suas configurações, o token do GitHub e o início automático na primeira execução. No GNOME, reinstale a extensão pelas configurações do Pier.

## Solução de problemas

### A barra de tarefas ou o Dock não voltou

O Pier restaura a barra de tarefas nativa (ou o Dock do macOS) ao sair. Se o processo for encerrado à força, rode o comando abaixo para restaurá-la:

```sh
%LOCALAPPDATA%\Pier\pier.exe --restore-taskbar                # Windows
/Applications/Pier.app/Contents/MacOS/pier --restore-taskbar   # macOS
```

No GNOME, o Dash volta sozinho.

## Contribuindo

Encontrou um bug ou tem uma ideia? [Abra uma issue](https://github.com/pier-dock/pier-dock/issues/new/choose). Para compilar o Pier a partir do código, rodar os testes ou enviar um pull request, veja o [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md).

## Licença

Copyright © 2026 Nathan Ferreira e Matheus Basilio.

O Pier é software livre: você pode redistribuí-lo e/ou modificá-lo sob os termos da [Licença Pública Geral GNU](LICENSE), conforme publicada pela Free Software Foundation, na versão 3 da Licença ou (a seu critério) em qualquer versão posterior.

O Pier é distribuído na esperança de que seja útil, mas SEM NENHUMA GARANTIA, nem mesmo a garantia implícita de COMERCIALIZAÇÃO ou de ADEQUAÇÃO A UM DETERMINADO FIM. Consulte a Licença Pública Geral GNU para mais detalhes.
