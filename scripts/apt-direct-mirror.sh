#!/bin/sh
# Os runners Ubuntu usam o mirror do Azure (azure.archive.ubuntu.com) via apt-mirrors.txt, e quando o
# DNS dele cai o apt trava até estourar o timeout, sem que Acquire::Retries ajude. Aponta direto
# para o archive oficial. Cobre o formato do 22.04 (sources.list) e o do 24.04 (ubuntu.sources).
set -e
for file in /etc/apt/sources.list /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources; do
  [ -f "$file" ] && sudo sed -i 's|mirror+file:/etc/apt/apt-mirrors.txt|http://archive.ubuntu.com/ubuntu/|' "$file"
done
exit 0
