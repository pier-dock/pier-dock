# Novidades

O que mudou em cada versão do Pier. O app mostra estas notas uma vez depois de se atualizar, então escreva para quem usa. Antes de criar a tag de uma release, adicione uma seção `## <versão>` aqui e no `CHANGELOG.md`.

## 0.21.0

### Widget de layout do teclado
- **Windows:** o novo widget **Layout do teclado** mostra o código do idioma do layout ativo (ENG, POR…), como o da barra de tarefas do Windows. Clique nele para ver os layouts instalados e alternar entre eles. Ele vem desligado e fica oculto no macOS e no Linux.

### Correções
- **Windows:** se o Pier for encerrado à força, por exemplo pelo Gerenciador de Tarefas, a barra de tarefas agora volta sozinha. Antes ela ficava oculta até você rodar `pier.exe --restore-taskbar`.
- **Dock:** quando o cursor cai de uma vez sobre um ícone, como quando a dock sobe de volta debaixo dele, os vizinhos não crescem mais além do tamanho final para depois encolher.
- **Dock:** a barra continua centralizada, dentro do vidro por trás dela, enquanto a ampliação a alarga durante a animação de aparecer.

## 0.20.0

### Aparência e feedback redesenhados
- **Configurações:** a aba **Aparência** ganhou um novo layout. A pré-visualização ao vivo fica no topo, e os ajustes estão em três cards de mesma altura: **Estilo** (material, opacidade, efeitos reduzidos, tamanho dos ícones e cor de destaque), **Dock** (posição, largura e altura do painel) e **Comportamento** (ocultar automaticamente, ampliação e apps abertos). Tudo cabe na janela de configurações padrão sem rolar. Os textos explicativos saíram, e os avisos sobre sistema sem vidro e sobre **Nunca** ocultar a dock continuam. A pré-visualização agora mostra a posição da dock e a ampliação, e o modo de ocultar **Com app em uso** passou a se chamar **Inteligente**.
- **Reportar um problema:** o diálogo tem um novo visual, com um aviso de que o relato vira uma issue pública no GitHub e um botão **Cancelar**. Depois do envio, uma confirmação mostra a issue criada, com o repositório e o número, e um botão para abri-la. Quando o GitHub abre no navegador em vez disso, a confirmação avisa, e **Enviar outro** começa um novo relato.

## 0.19.0

### Pokémon na sua dock
- **Agentes de IA:** o mascote que anda pela dock enquanto uma sessão roda agora pode ser um Pokémon no lugar do Clawd. Nas opções do widget **Agentes de IA**, escolha **Mascote** e depois o Pokémon. Ele ganha um ponto de status que acompanha a sessão principal, salta quando uma sessão pede permissão, o clique continua abrindo as sessões, e o ícone do widget na dock acompanha a sua escolha.
- **Widget Pokémon:** escolha até 10 Pokémon em vez de um. Todos andam pela borda da dock, cada um no seu ritmo, e o primeiro é o ícone na dock. As configurações listam os escolhidos, com um botão para remover cada um e um contador, e param de aceitar no 10. O Pokémon que você já tinha continua.

## 0.18.0

### Atualizações que esperam você
- **Atualizações:** o Pier não instala mais uma atualização e reinicia sozinho. Quando sai uma versão nova, aparece um ponto no lançador de apps (ou no botão Iniciar, se o lançador estiver oculto), e **Atualização para x.y.z disponível** entra nos menus da dock. Clique em qualquer um para abrir **Configurações** > **Geral** e depois em **Instalar e reiniciar**. O item da bandeja continua instalando na hora. **Procurar atualizações automaticamente** agora liga ou desliga as checagens em segundo plano.
- **macOS:** o Pier agora é assinado com um certificado fixo, então o macOS mantém as permissões de Acessibilidade, Gravação de Tela e áudio do sistema entre as atualizações. Depois desta atualização ele pede essas permissões uma última vez.

### Primeira abertura
- Numa instalação nova o Pier não esconde mais a barra de tarefas do Windows, o Dash do GNOME nem o Dock do macOS de cara. Ele abre as **Configurações** com um card de boas-vindas que pergunta antes.
- Enquanto a extensão do GNOME ou a Acessibilidade do macOS ainda precisa ser configurada, um ponto âmbar no lançador de apps abre as **Configurações**, onde o aviso diz o que fazer. Antes, só dava para ver abrindo as **Configurações**.

### Correções
- **Configurações:** um arquivo de configuração ilegível não é mais trocado pelo padrão. Ele fica guardado como `config.json.corrupt-<hora>` e a gravação anterior é restaurada. As gravações são atômicas, então uma atualização ou um travamento no meio de uma não deixa o arquivo pela metade. Se as configurações não carregarem, a janela mostra o erro e tenta de novo em vez de ficar em branco, e uma configuração que não grava volta ao valor anterior.
- **Idioma:** os erros do sistema, como "sem interface Wi-Fi", seguem o idioma da interface em vez de virem sempre em português. O exemplo do endereço iCal, a descrição do widget de relógio e a descrição da extensão do GNOME também foram corrigidos.
- **Windows:** um app com o ID de pasta malformado podia quebrar a lista de apps. Agora ele é ignorado.

## 0.17.0

### Pokémon na sua dock
Um novo widget, **Pokémon**, no novo grupo **Enfeites**, mostra o ícone de time do Pokémon que você escolher, vindo da PokeAPI. Busque por nome ou número (#1 ao #809, os que têm ícone de time). Como o widget de sessões de IA, ele pode ficar na dock como ícone, andar pela borda dela, ou os dois. Ele olha para o lado em que anda, com os pés na borda, e funciona junto com o Clawd.

### Widgets
- **Configurações:** a galeria de widgets lista primeiro os que estão em uso. Ao ligar um, ele vai para o fim dos ativos, onde aparece na dock.
- **Configurações:** com **Só o Clawd** ou **Só andando**, o card do widget aparece como ligado, em vez de desligado.

### Correções
- **Som:** no Linux (PulseAudio/PipeWire), saídas que não estão conectadas, como portas HDMI sem nada ligado, não aparecem mais na lista, então não dá para escolher uma e ficar sem som.

## 0.16.0

### Correções
- **Atualizações:** no Linux, **Instalar e reiniciar** abre o Pier de novo depois de instalar a nova versão, em vez de só fechá-lo. No NixOS, uma atualização em AppImage reinicia pelo `appimage-run`.
- **Som:** o seletor de saída de cada app abre dentro do painel de som, então escolher um dispositivo não fecha mais o seletor nem o painel inteiro. Também funciona pelo teclado (setas, Enter e Esc).
- **Dock:** clicar em um app que não está fixado não embaralha mais os apps abertos. Eles ficam onde apareceram, e os apps abertos depois entram no fim.

## 0.15.0

### Contadores de notificação no Windows
No Windows, o contador vermelho no ícone do dock agora também conta as notificações do app na Central de Notificações, e o contador que apps da Store como o WhatsApp põem na barra de tarefas. Funciona para apps que não mostram a contagem no título da janela, como Discord e Steam. Quando os dois existem, o dock mostra o maior número.

### Correções
- **Windows:** apps da Store que não informam o ID na janela, como o WhatsApp, mostram o ícone real e são abertos do mesmo jeito que pelo Menu Iniciar.

## 0.14.0

### Modos de energia e áudio por app
- **Bateria:** o widget de bateria abre um painel com o perfil de energia ativo e os modos Economia, Equilibrado e Desempenho que o sistema suporta. Um modo que não consegue ser aplicado é informado em vez de aparecer como selecionado.
- **Som:** no Linux (PulseAudio/PipeWire), cada app que está tocando mostra o ícone e o seu próprio seletor de saída, para mandar um app para outro dispositivo sem mexer nos demais.

### Relatórios sem conta do GitHub
Agora dá para enviar um relatório de problema pelo Pier sem conectar uma conta do GitHub. Ele é aberto como uma issue para você.

### Configurações na dock
Enquanto **Configurações** ou **Novidades** estiver aberta, o Pier aparece como um app em execução: na dock e, no macOS, também no Dock do sistema e no Cmd+Tab. Clique nele para trazer a janela de volta, mesmo com outras janelas por cima.

### Dock mais fluida
Passar o mouse e ampliar os ícones ficou mais rápido e leve, e as atualizações dos widgets em segundo plano não redesenham mais a dock inteira. Em **Configurações** > **Aparência** há uma nova opção **Reduzir efeitos visuais**, que usa fundo sólido e tira as sombras decorativas, para máquinas que sofrem com o desfoque.

### Mais melhorias nos widgets
- **Relógio:** clicar na data abre um calendário do mês com mês anterior/próximo e **Hoje**, em vez das notificações.
- **Bluetooth:** dispositivos salvos continuam na lista com o Bluetooth desligado, e dá para ligar, conectar e desconectar pelo painel.
- **Rede, Som e Bluetooth:** o rodapé dos painéis não repete mais detalhes de conexão que confundiam.
- **Ícones da dock** agora seguem a arte do site, incluindo bateria, controles de mídia, volume, Wi-Fi e calendário.

### Correções
- **Windows:** com a dock sempre visível, apps em tela cheia e vídeos escondem a dock.
- **Agentes de IA:** o Antigravity e outros provedores rodando dentro do T3 Code são detectados.
- **Clawd:** o mascote fica em pé na dock em todas as bordas da tela, não só embaixo.
- **Dock:** o destaque ao passar o mouse num ícone ampliado fica dentro da barra.
- **Configurações do sistema:** o Bluetooth abre o app de configurações certo em cada desktop Linux, e uma ação que falha agora mostra um erro com opção de tentar de novo.

## 0.13.0

### Contador de notificações na dock
Os ícones dos apps agora mostram um selo vermelho com as notificações não lidas que os apps e sites colocam no início do título da janela, como "(3) WhatsApp" ou "[12] Caixa de entrada".
- A contagem soma as janelas do app e para em **99+**.
- O tooltip menciona as notificações, e leitores de tela leem a contagem junto com o estado do app.

### Um calendário mais claro
O popover do calendário agora tem um foco só: seu próximo evento.
- O cabeçalho mostra o dia da semana, a data e quantos eventos faltam.
- O próximo evento fica num cartão neutro com a cor do evento como barra de destaque, o status, o título, o horário e o local na mesma linha, e uma barra de progresso enquanto acontece. **Entrar** ocupa a largura toda, e **Copiar link** virou um botão de ícone.
- O resto do dia é uma linha do tempo com um trilho colorido, dividida em **Mais tarde hoje** e **Amanhã** (esmaecido). Um dia livre mostra um estado vazio centralizado.

### Correções
- **Agentes de IA:** no opencode V2, o plugin não carregava e as sessões do opencode nunca apareciam. Reinstale o plugin nas opções do widget e reinicie o opencode.
- **Agentes de IA:** sessões do Codex iniciadas por um app server, como o T3 Code, são detectadas automaticamente pelos eventos de ciclo de vida locais, sem hooks nem reinício. Hooks opcionais ainda adicionam o status de permissão.
- **Windows:** apps da Microsoft Store, como o WhatsApp, mostram o ícone real em vez do ícone genérico do Windows. Um ícone que falha ao carregar agora é tentado de novo em vez de ficar em cache para sempre.
- **Linux:** clicar num app no AppImage volta a funcionar no NixOS; os apps iniciados não herdam mais as bibliotecas empacotadas no AppImage.

## 0.12.0

### Um widget para todas as suas IAs de linha de comando
O widget **Claude Agents** virou **Agentes de IA** e agora mostra sessões do Claude Code, Codex, opencode e Gemini CLI: rodando, pedindo permissão, com erro ou concluídas.
- Cada CLI tem a própria integração em **Configurações** > **Widgets** > opções do widget. **Ativar** instala os hooks (ou o plugin do opencode) e **Desativar** remove.
- O Codex pede para confiar nos hooks do Pier com `/hooks`; o opencode precisa ser reiniciado para carregar o plugin.
- A lista mostra a qual CLI cada sessão pertence, e clicar numa linha continua levando ao terminal ou à IDE que a iniciou. Sua configuração atual do Claude Code continua funcionando, e **Reinstalar** a move para o hook compartilhado.
- O widget compartilha só o provedor, o status, a pasta do projeto e o horário — nunca o conteúdo da conversa.

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
