#!/bin/bash
if [ -n "$1" ]; then
    targetIp="$1"
else
    targetIp="192.168.1.213"
fi

fileToSend="/home/f4613569/personal/projects/jakson_todo/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk"

fileName="$(basename "$fileToSend")"

if [ ! -f "$fileToSend" ]; then
    echo "Erro: Arquivo não encontrado: $fileToSend"
    exit 1
fi

echo "Removendo arquivo anterior ($fileName) no dispositivo $targetIp..."
curl -s "http://$targetIp:8080/api/files/delete" \
  -H 'Content-Type: application/json' \
  -H "Origin: http://$targetIp:8080" \
  -H "Referer: http://$targetIp:8080/files/Download/" \
  --data-raw "{\"path\":\"Download\",\"names\":[\"$fileName\"]}" \
  --insecure

echo "Enviando $fileName para $targetIp..."
curl --progress-bar \
  "http://$targetIp:8080/api/files/upload?path=Download&name=$fileName&overwrite=1" \
  -H "Origin: http://$targetIp:8080" \
  -H "Referer: http://$targetIp:8080/files/Download/" \
  -H "Content-Type:" \
  --data-binary "@$fileToSend" \
  --insecure

echo ""
echo "Concluído!"