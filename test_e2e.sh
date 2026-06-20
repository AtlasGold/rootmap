#!/bin/sh

# Script de Teste End-to-End para o rootmap (Totalmente portável e compatível com POSIX/sh/bash)

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # Sem cor

printf "${BLUE}=====================================================${NC}\n"
printf "${BLUE}          rootmap — Iniciando Teste E2E (SQLite)     ${NC}\n"
printf "${BLUE}=====================================================${NC}\n"

BINARY="./target/debug/rootmap"

if [ ! -f "$BINARY" ]; then
    printf "${RED}Erro: Binário não encontrado em $BINARY. Por favor compile com cargo build.${NC}\n"
    exit 1
fi

# 1. Limpar banco SQLite antigo
printf "\n${YELLOW}[1/4] Limpando banco SQLite local...${NC}\n"
rm -f rootmap.db

# 2. Iniciar contêiner do Neo4j
printf "\n${YELLOW}[2/4] Iniciando Neo4j no Docker...${NC}\n"
docker compose down -v --remove-orphans || true
docker compose up -d neo4j

# 3. Aguardar Neo4j ficar pronto (Healthcheck)
printf "\n${YELLOW}[3/4] Aguardando inicialização do Neo4j...${NC}\n"
printf "Aguardando Neo4j ficar pronto..."
until [ "$(docker inspect --format='{{json .State.Health.Status}}' rootmap-neo4j 2>/dev/null)" = "\"healthy\"" ]; do
    printf "."
    sleep 2
done
printf " ${GREEN}Neo4j PRONTO!${NC}\n"

# 4. Rodar Migrations no SQLite
printf "\n${YELLOW}[4/4] Executando migrations de banco SQLite...${NC}\n"
$BINARY migrate

# 5. Executar Infra Scan local (Salva no SQLite)
printf "\n${YELLOW}[5/5] Executando scan de infraestrutura local...${NC}\n"
$BINARY scan

# 6. Importar dependências de exemplo e sincronizar
printf "\n${YELLOW}[6/6] Importando inventário YAML e sincronizando para o Neo4j...${NC}\n"
$BINARY import -f examples/inventory.yaml
$BINARY sync

# 7. Executar as análises e exibir os resultados
printf "\n${GREEN}=====================================================${NC}\n"
printf "${GREEN}      Validação das Funcionalidades da CLI           ${NC}\n"
printf "${GREEN}=====================================================${NC}\n"

printf "\n${BLUE}➔ 1. Executando Análise de Impacto (downstream de postgresql):${NC}\n"
$BINARY impact --node Service:postgresql

printf "\n${BLUE}➔ 2. Procurando Caminho mais Curto (nginx até postgresql):${NC}\n"
$BINARY path --from Service:nginx --to Service:postgresql

printf "\n${BLUE}➔ 3. Analisando Causa Raiz Heurística (sintoma: 'site fora do ar', afetado: nginx):${NC}\n"
$BINARY incident analyze --symptom "site fora do ar" --affected Service:nginx

printf "\n${BLUE}➔ 4. Gerando Relatório Markdown do Último Incidente Analisado:${NC}\n"
$BINARY report --last --format markdown

printf "\n${GREEN}=====================================================${NC}\n"
printf "${GREEN}          Teste E2E Concluído com Sucesso!          ${NC}\n"
printf "${GREEN}=====================================================${NC}\n"
