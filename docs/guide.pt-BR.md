# Guia do Pier

[English](guide.md) · **Português** · [Voltar ao README](../README.pt-BR.md)

## Recursos

- **Apps:** apps fixados e abertos, com ampliação, reordenação por arrastar, lista de janelas e bolinhas de contagem.
- **Barra personalizável:** coloque a dock em qualquer borda da tela, deixe-a no tamanho dos ícones ou de ponta a ponta e escolha o tamanho dos ícones, o material e a cor de destaque.
- **Widgets:** player de mídia, GitHub (revisões, pull requests, commits, contribuições), sistema (rede, volume, Bluetooth, bateria, relógio, mostrar a área de trabalho, uso do sistema e, no Windows, layout do teclado e bandeja), clima, agenda, Pomodoro e uso de IA.
- **Agentes de IA:** as sessões do Claude Code, Codex, opencode e Gemini CLI num relance, e um clique leva ao terminal ou à IDE que a iniciou. O Clawd ou um Pokémon podem andar pela dock enquanto uma sessão roda.
- **E mais:** atalhos de teclado para os apps das posições 1 a 9, atualização com um clique e interface em português e inglês.

## Instalação

Baixe o instalador da sua plataforma na [versão mais recente](https://github.com/pier-dock/pier-dock/releases/latest).

| Plataforma | Arquivo | Observações |
| --- | --- | --- |
| Windows 10 e 11 | `Pier_x.y.z_x64-setup.exe` | Instala em `%LOCALAPPDATA%\Pier`, sem pedir administrador. |
| macOS 12 ou mais recente | `Pier_x.y.z_universal.dmg` | Roda nativamente em Macs com Apple Silicon e Intel. |
| Linux (GNOME 45 a 49, KDE Plasma 6) | `Pier_x.y.z_amd64.deb` ou `Pier_x.y.z_amd64.AppImage` | Instale o `.deb` com `sudo apt install ./Pier_x.y.z_amd64.deb`, ou torne o AppImage executável com `chmod +x` e rode-o. No KDE Plasma não é preciso mais nada; no GNOME, instale a extensão pelas configurações do Pier. |

### Primeira abertura

Na primeira vez, o Pier abre as **Configurações** e pergunta se deve substituir a barra de tarefas, o Dash ou o Dock. A barra do sistema continua visível até você escolher. Se ainda faltar configurar algo (a extensão do GNOME ou a Acessibilidade do macOS), aparece um aviso nas **Configurações** e um ponto âmbar no lançador de apps.

Os instaladores ainda não têm assinatura de código, então o sistema avisa na primeira vez que você abre o Pier.

- **Windows:** quando o SmartScreen aparecer, clique em **Mais informações** e depois em **Executar assim mesmo**.
- **macOS:** arraste o Pier para **Aplicativos** e abra. Quando o macOS disser que não pôde verificar o app, vá em **Ajustes do Sistema** > **Privacidade e Segurança** e clique em **Abrir Mesmo Assim**. Se preferir, rode `xattr -dr com.apple.quarantine /Applications/Pier.app` no Terminal.
- **Linux:** no GNOME, abra as configurações do Pier, clique em **Instalar extensão** e depois encerre a sessão e entre de novo. A extensão permite que o Pier veja e foque janelas, reserve espaço na tela, esconda o Dash, mostre a área de trabalho e registre os atalhos. No KDE Plasma não há nada para instalar: numa sessão X11 a dock lista as janelas, reserva o espaço na tela e aplica o blur do KWin.

### Atualizações

O Pier procura atualizações logo depois de abrir e a cada 12 horas, e nunca reinicia sozinho. Quando houver uma versão nova, aparece um ponto no lançador de apps. Clique nele (ou em **Atualizar para x.y.z** no menu da bandeja) e depois em **Instalar e reiniciar** em **Configurações** > **Geral**. Para parar de procurar, desligue **Procurar atualizações automaticamente** ali.

A atualização pelo app funciona com o instalador do Windows, com o app do macOS e com o AppImage. Se você instalou o pacote `.deb`, baixe e instale cada versão nova pela página de releases.

## Uso

- **Clique** num app para abri-lo ou focá-lo (no app ativo, minimiza; com várias janelas, alterna entre elas). **Shift+clique** abre uma nova instância.
- **Clique com o botão direito** num app para ver as janelas, fixá-lo ou desafixá-lo ou fechá-lo. No fundo da dock, abre as configurações ou sai.
- **Clique** num separador para abrir o menu de remoção; ele também pode ser acessado pelo teclado.
- **Arraste** os apps fixados para reordená-los. Para fixar um app, solte-o na dock ou adicione-o em **Configurações** > **Apps**.
- **Clique** num widget para abrir o painel. No volume, role para ajustar e clique com o botão direito para silenciar.
- **Pressione** <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>1</kbd>…<kbd>9</kbd> (<kbd>Control</kbd>+<kbd>Option</kbd>+<kbd>1</kbd>…<kbd>9</kbd> no macOS) para abrir ou focar o app dessa posição.

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

O Pier tem suporte completo ao GNOME 45 a 49, em Wayland e X11, pela extensão do GNOME Shell. No KDE Plasma 6 não precisa de extensão: numa sessão X11 ele lista as janelas, reserva o espaço na tela e desenha o vidro com o blur do KWin, e no Wayland lista só as janelas dos apps X11. Em outros ambientes de desktop, o Pier só lista janelas em sessões X11 e não consegue esconder o painel nativo. Apps fixados e widgets funcionam em todos.

Para deixar a dock em outro monitor, ou fazer ela seguir o cursor, o GNOME precisa da versão 4 da extensão. Se **Configurações** > **Geral** > **Mostrar a dock em** oferecer a atualização, clique em **Atualizar a extensão** e depois encerre a sessão e entre de novo.

Rodando como AppImage, o Pier cadastra a própria entrada de menu e os ícones em `~/.local/share` na primeira execução, então a dock aparece no menu de aplicativos sem nenhum ajuste manual.

## Configuração

O Pier guarda as configurações em `config.json`:

- **Windows:** `%APPDATA%\pier\config.json`
- **macOS:** `~/Library/Application Support/pier/config.json`
- **Linux:** `~/.config/pier/config.json`

Se você usava o Pier quando ele se chamava WinDock, o Pier importa as suas configurações, o token do GitHub e o início automático na primeira execução. No GNOME, reinstale a extensão pelas configurações do Pier.

## Solução de problemas

### Lentidão ao passar o mouse

Se a animação ao passar o mouse ficar lenta, experimente **Configurações > Aparência > Reduzir efeitos visuais**. A opção desativa o desfoque e as sombras decorativas da dock e dos ícones, mantendo os widgets e as animações. Ao desativá-la, o material escolhido é restaurado.

### A barra de tarefas ou o Dock não voltou

O Pier restaura a barra de tarefas nativa (ou o Dock do macOS) ao sair. No Windows, um pequeno processo auxiliar também restaura a barra de tarefas quando o Pier é encerrado à força, por exemplo pelo Gerenciador de Tarefas. Se ela continuar sumida (ou no macOS), rode o comando abaixo para restaurá-la:

```sh
%LOCALAPPDATA%\Pier\pier.exe --restore-taskbar                # Windows
/Applications/Pier.app/Contents/MacOS/pier --restore-taskbar   # macOS
```

No GNOME, o Dash volta sozinho.

## Contribuindo

Encontrou um bug ou tem uma ideia? [Abra uma issue](https://github.com/pier-dock/pier-dock/issues/new/choose). Para compilar o Pier a partir do código, rodar os testes ou enviar um pull request, veja o [CONTRIBUTING.pt-BR.md](../CONTRIBUTING.pt-BR.md).

## Licença

Copyright © 2026 Nathan Ferreira e Matheus Basilio.

O Pier é software livre: você pode redistribuí-lo e/ou modificá-lo sob os termos da [Licença Pública Geral GNU](../LICENSE), conforme publicada pela Free Software Foundation, na versão 3 da Licença ou (a seu critério) em qualquer versão posterior.

O Pier é distribuído na esperança de que seja útil, mas SEM NENHUMA GARANTIA, nem mesmo a garantia implícita de COMERCIALIZAÇÃO ou de ADEQUAÇÃO A UM DETERMINADO FIM. Consulte a Licença Pública Geral GNU para mais detalhes.
