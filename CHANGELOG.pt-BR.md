# Novidades

O que mudou em cada versão do Pier. O app mostra estas notas uma vez depois de se atualizar, então escreva para quem usa. Antes de criar a tag de uma release, adicione uma seção `## <versão>` aqui e no `CHANGELOG.md`.

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
