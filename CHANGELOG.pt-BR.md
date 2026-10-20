# Novidades

O que mudou em cada versão do Pier. O app mostra estas notas uma vez depois de se atualizar, então escreva para quem usa. Antes de criar a tag de uma release, adicione uma seção `## <versão>` aqui e no `CHANGELOG.md`.

## 0.11.0

### Claude Code na dock
O novo widget **Claude Agents** mostra suas sessões do Claude Code: rodando, pedindo permissão, com erro ou concluídas.
- Clique nele para ir à janela do terminal ou da IDE que abriu a sessão. Com mais de uma sessão, ele abre uma lista para escolher.
- Ele precisa dos hooks do Claude Code: em **Configurações** > **Widgets**, abra as opções do widget e selecione **Ativar** ao lado de **Hooks do Claude Code**. O Pier os adiciona em `~/.claude/settings.json`, e **Desativar** remove.

### Conheça o Clawd
O Clawd, mascote do Claude, anda pela dock enquanto uma sessão roda. Ele pula quando uma sessão pede permissão e descansa quando está tudo parado.
- **Mostrar na dock**, nas opções do widget, escolhe **Ícone**, **Ícone e Clawd** ou **Só o Clawd**.
- Clique no Clawd para abrir suas sessões, como no widget. Ele para de andar enquanto o cursor está sobre ele ou a lista de sessões está aberta.
- Ele se esconde junto com a dock quando **Ocultar automaticamente** está ligado.

### Correções
- Depois que a dock se escondia e voltava, o tooltip do último app ou widget usado podia continuar na tela, às vezes ao lado de outro.
- **Windows:** quando o sistema não consegue aplicar o material Mica, as Configurações e a janela de novidades agora usam o material sólido em vez de aparecerem transparentes.

## 0.10.0

### A dock em outro monitor
Com mais de um monitor, **Configurações** > **Geral** > **Mostrar a dock em** escolhe onde a dock fica:
- **Monitor principal**, como antes.
- Um monitor específico, pelo nome. Se ele for desconectado, a dock vai para o principal e volta quando ele for plugado de novo.
- **Seguir o cursor**: a dock vai para o monitor onde está o mouse.

Os popovers abrem no monitor da dock, e no Windows as janelas maximizadas desse monitor abrem espaço para ela. No macOS, com monitores de escalas diferentes (um Retina e um monitor externo), a dock fica no lugar certo e responde ao mouse em qualquer um deles.

> **GNOME:** para reservar espaço fora do monitor principal, e para seguir o cursor no Wayland, a extensão precisa ser atualizada. Em **Mostrar a dock em**, clique em **Atualizar a extensão** e depois saia e entre de novo na sessão.

### A dock entra em cena
Ao abrir o Pier e ao mudar de monitor, a dock desliza para dentro da tela em vez de aparecer de uma vez. Na troca de monitor, ela sai da tela antiga antes de entrar na nova, e fica à mostra por um instante mesmo com **Ocultar automaticamente** ligado. Com **Reduzir movimento** ligado no sistema, ela só esmaece.

### Atualizações automáticas
Quando há uma versão nova, o Pier se atualiza sozinho ao abrir, reinicia e mostra o que mudou. Para voltar a instalar à mão, desligue **Configurações** > **Geral** > **Atualizar automaticamente**. No Linux isso vale para o AppImage; no `.deb`, a atualização continua pelo menu da bandeja.

### Volume de cada app no macOS
O popover de som agora lista os apps que estão tocando áudio no macOS 14.2 ou mais novo, cada um com o próprio controle de volume e o botão de silenciar, como no Windows e no Linux.
- Na primeira vez que você mexe num controle, o macOS pede a permissão de **Gravação de Áudio do Sistema**: o Pier passa o áudio do app por ele para mudar o volume. Sem a permissão, o popover avisa e mostra um botão que abre os Ajustes.
- O Pier lembra o volume de cada app e o aplica de novo quando o app volta a tocar, inclusive depois de reabrir o Pier.

### Configurações
- **Importar** e **Exportar**, em **Geral**, levam os apps fixados, os widgets e a aparência para outra máquina num arquivo. A conta do GitHub e a agenda não vão junto.
- **Redefinir tudo** volta todas as configurações ao padrão, depois de pedir confirmação. A conta do GitHub e a agenda continuam conectadas.
- **Permissões**, em **Sistema**, resume o que o Pier tem do sistema. **Revisar** lista cada permissão, para que ela serve e um botão para concedê-la ou abrir os Ajustes. No macOS: Acessibilidade, Gravação de Tela, Automação, Localização e Bluetooth. No Windows: Localização.

### Correções
- A janela de novidades, que abre depois de uma atualização, agora aparece na frente das outras janelas em vez de ficar atrás delas.

## 0.9.0

### Um lançador de apps novo
O lançador tem um campo de busca maior e agora separa seus apps em duas partes.
- **Fixados** mostra, em galeria, os apps que você fixa no lançador, na ordem em que foram fixados. Eles são separados dos fixados da dock.
- **Apps instalados** lista todo o resto por nome. A busca filtra os dois.
- **Ctrl+P** (**⌘P** no macOS) fixa ou desafixa o app selecionado. **Ctrl+K** (**⌘K**) ou o botão direito mostra mais ações, como **Fixar na dock**.
- As setas andam pela galeria e pela lista, e o **Enter** abre o app selecionado.

### Correções
- **Windows 11:** com o material vidro líquido, a dock não fica mais branca quando você passa o mouse por ela, e os cantos arredondados não mostram mais uma borda quadrada. O mesmo vale para os popovers e as prévias das janelas.

## 0.8.1

### Correções
- **Windows:** depois de passar o mouse pelos ícones da dock por um tempo, ela podia parar de responder e deixar o mouse chegar à janela de trás. Agora o Pier percebe quando isso acontece e faz a dock voltar a responder na hora.

## 0.8.0

### Escolha onde a dock fica
**Configurações** > **Aparência** > **Posição** coloca a dock na borda **Embaixo** (como antes), **Em cima**, **À esquerda** ou **À direita** da tela.
- Nas laterais a barra fica em pé, como uma coluna. Os widgets largos passam para o tamanho compacto, e as dicas e as prévias das janelas abrem para o lado.
- No Windows, no Linux com X11 e no GNOME, as janelas maximizadas abrem espaço para a dock em qualquer borda. No macOS a dock muda de lugar, mas o sistema continua não deixando os apps reservarem espaço na tela.

> **GNOME:** as bordas de cima e laterais precisam da versão nova da extensão do Pier. Abra as **Configurações**, reinstale a extensão e depois saia e entre de novo na sessão. Com a antiga, só a borda de baixo reserva espaço.

### Prévias das janelas melhores
Ao passar o mouse num app com mais de uma janela, a faixa de prévias agora combina com o resto da dock.
- Ela tem o mesmo vidro dos popovers.
- No macOS os cartões se destacam ao passar o mouse e mostram o botão de fechar.
- Cada cartão mostra uma imagem da janela no macOS (14 ou mais novo) e no Windows, e não só o ícone do app. No macOS isso precisa da permissão de **Gravação de Tela**, que o Pier pede na primeira vez que as prévias abrem. Janelas minimizadas continuam com o ícone do app.
- A faixa fecha assim que o mouse sai dela e do ícone do app, por exemplo ao passar para outro ícone.
- A dock não pisca mais quando a faixa abre ou fecha.

### Dispositivos Bluetooth pareados
O popover de Bluetooth agora lista todos os dispositivos pareados, e não só os conectados, em **Conectados** e **Pareados**.
- Clique num pareado para conectar, e use **Desconectar** num conectado.
- No Windows, o clique num dispositivo abre as configurações de Bluetooth, porque o Windows não deixa os apps conectarem fones e teclados.

### Novidades depois de atualizar
Depois que o Pier se atualiza, uma janela mostra o que mudou na versão nova, uma vez só. Abra de novo quando quiser em **Configurações** > **Geral** > **Ver novidades**.

### Correções
- **macOS:** o botão **Permitir** do popover de Wi-Fi agora mostra o pedido de Localização, e as redes por perto aparecem. Antes, o clique não fazia nada.
- Com **Ocultar automaticamente** ligado, a dock se esconde assim que o mouse passa dos ícones, e não bem acima deles, e espera um pouco menos para esconder.

## 0.7.0

### Suporte ao KDE Plasma
O Pier agora roda no KDE Plasma sem nenhuma configuração manual.
- **Plasma no X11:** a dock lista, foca, minimiza e fecha janelas, reserva o espaço na tela, e o material de vidro desfoca com o KWin. Não há extensão para instalar.
- **Plasma no Wayland:** a dock lista as janelas dos apps X11. As configurações explicam que as janelas nativas do Wayland ainda não aparecem.
- **AppImage:** na primeira abertura, o Pier se adiciona ao menu de apps com o ícone, então não é mais preciso escrever a entrada do menu à mão. Uma entrada que já existe nunca é alterada.

### Prévia das janelas ao passar o mouse
Passe o mouse num app com mais de uma janela para ver, acima da dock, um cartão por janela, com o título e um botão de fechar. Clique num cartão para focar aquela janela.
- No Linux com X11, cada cartão mostra a miniatura da janela. No Windows, no macOS e no GNOME, por enquanto o cartão mostra o ícone do app.
- Apps com uma janela só continuam abrindo e escondendo a janela com um clique.

### Popover de volume
O clique no widget de volume abre um popover com:
- o volume geral e o botão de silenciar;
- as saídas de áudio, para trocar com um clique;
- o volume de cada app que está tocando (Windows e Linux).

Rolar e clicar com o botão direito no widget continuam ajustando e silenciando o volume.

### Popover de Wi-Fi
O clique no widget de rede abre um popover com a rede atual e as redes por perto.
- Conecte numa rede salva ou aberta com um clique, ou digite a senha de uma rede nova na própria linha.
- Desconecte, e ligue ou desligue o Wi-Fi quando o sistema permite.
- No macOS 14+ e no Windows 11 24H2, o sistema só mostra os nomes das redes para apps com acesso à Localização. O popover pede esse acesso quando precisa.

### Lançador de apps
Um novo item **Lançador de apps** na dock abre um painel com todos os apps instalados.
- Digite para buscar (maiúsculas e acentos não importam), use as setas para andar e **Enter** para abrir.
- Clique com o botão direito num app para fixá-lo na dock.
- Ele vem desligado. Ligue em **Configurações** > **Apps** > **Mostrar o lançador de apps**.

### Widget de uso de IA com o Codex CLI
O widget de uso do Claude Code agora é **Uso de IA** e também lê o **Codex CLI**. Escolha a IA em **Configurações** > **Widgets** > **Uso de IA**, ou deixe em **Automático**, que mostra a IA usada por último.
- O painel mostra uma barra para cada janela de limite que a IA informa, e a dica diz de qual IA vêm os números.
- Tudo é lido de arquivos locais. Nada vai pela rede, e só os números de uso são lidos, nunca as conversas.
- O widget que você já tinha e as configurações dele continuam valendo.

### Widget de mostrar a área de trabalho
Um novo widget **Área de trabalho** minimiza todas as janelas com um clique e traz de volta no clique seguinte, como o botão do canto da barra de tarefas do Windows. Funciona no Windows, no macOS, no Linux com X11, no GNOME e no KDE Plasma. Ligue em **Configurações** > **Widgets**.

> **GNOME:** este widget precisa da versão nova da extensão do Pier. Abra as **Configurações**, reinstale a extensão e depois saia e entre de novo na sessão.

### Altura do painel
**Configurações** > **Aparência** > **Altura do painel** define a altura da barra: **Automática** (como antes) ou uma altura em pixels. O espaço que a dock reserva na tela acompanha a nova altura, e os ícones continuam centralizados na barra.

### Formato número no widget de contribuições
O widget de contribuições do GitHub ganhou o formato **Número**, que mostra só o total do ano, sem o gráfico.

### Reportar um problema pelas configurações
**Configurações** > **Geral** agora tem **Reportar um problema** e **Sugerir um recurso**.
- O relato de bug inclui os dados do sistema e, se você quiser, os logs recentes do Pier, que dá para revisar e editar antes de enviar.
- Com uma conta do GitHub conectada ao Pier, a issue é criada direto. Sem ela, o formulário abre no navegador já preenchido.
- O Pier agora guarda um arquivo de log, o que facilita encontrar problemas.
