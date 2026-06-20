#!/bin/sh

# Script de Instalação Automatizada de Dependências para o rootmap
# Focado em sistemas Linux baseados em Debian, Ubuntu e Linux Mint.
# Deve ser executado com privilégios de root (sudo).

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # Sem cor

printf "${BLUE}=====================================================${NC}\n"
printf "${BLUE}          rootmap — Instalador de Dependências       ${NC}\n"
printf "${BLUE}=====================================================${NC}\n"

# 1. Verificar privilégios de root
if [ "$(id -u)" -ne 0 ]; then
    printf "${RED}Erro: Este script precisa ser executado como root.${NC}\n"
    printf "Por favor, execute: ${YELLOW}sudo ./setup.sh${NC}\n"
    exit 1
fi

# 2. Identificar a distribuição Linux
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
else
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
fi

printf "Identificando sistema operacional: ${GREEN}$OS${NC}\n"

# 3. Instalar pacotes de acordo com a distribuição
if [ "$OS" = "ubuntu" ] || [ "$OS" = "debian" ] || [ "$OS" = "linuxmint" ] || [ "$OS" = "pop" ]; then
    printf "\n${YELLOW}[1/4] Atualizando repositórios de pacotes (apt)...${NC}\n"
    apt-get update -y

    printf "\n${YELLOW}[2/4] Instalar pacotes essenciais do sistema...${NC}\n"
    # build-essential, pkg-config, libssl-dev -> para compilação do Rust se necessário
    # sqlite3 -> para manipulação manual do banco local
    # curl -> utilitário de rede
    apt-get install -y build-essential pkg-config libssl-dev sqlite3 curl

    # Instalar Docker se não existir
    if ! command -v docker >/dev/null 2>&1; then
        printf "\n${YELLOW}Instalando Docker Engine...${NC}\n"
        apt-get install -y docker.io
        systemctl enable --now docker
    else
        printf "${GREEN}✓ Docker já está instalado.${NC}\n"
    fi

    # Instalar Docker Compose se não existir
    if ! docker compose version >/dev/null 2>&1 && ! command -v docker-compose >/dev/null 2>&1; then
        printf "\n${YELLOW}Instalando Docker Compose...${NC}\n"
        apt-get install -y docker-compose-v2 || apt-get install -y docker-compose
    else
        printf "${GREEN}✓ Docker Compose já está instalado.${NC}\n"
    fi
else
    printf "${RED}Aviso: Distribuição '$OS' não é suportada automaticamente por este instalador.${NC}\n"
    printf "Por favor, instale manualmente os seguintes pacotes usando o gerenciador da sua distro:\n"
    printf " - docker e docker-compose (ou docker-compose-v2)\n"
    printf " - sqlite3\n"
    printf " - build-essential, pkg-config, openssl-devel (se for compilar código Rust)\n"
    exit 1
fi

# 4. Configurar permissões do Docker para o usuário
if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    printf "\n${YELLOW}[3/4] Configurando permissões do Docker para o usuário '$SUDO_USER'...${NC}\n"
    if ! getent group docker >/dev/null; then
        printf "Grupo 'docker' não encontrado. Criando grupo...\n"
        groupadd docker
    fi
    usermod -aG docker "$SUDO_USER"
    printf "${GREEN}✓ Usuário '$SUDO_USER' adicionado ao grupo docker.${NC}\n"
else
    printf "\n${YELLOW}[3/4] Pulando configuração de grupo do docker (executado diretamente como root)...${NC}\n"
fi

# 5. Inicializar o contêiner do Neo4j
printf "\n${YELLOW}[4/5] Inicializando banco de dados de grafos Neo4j...${NC}\n"
if [ -f "docker-compose.yml" ]; then
    if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
        # Subir como usuário comum para que os volumes criados em ./data/ pertençam a ele, não ao root
        sudo -u "$SUDO_USER" docker compose up -d neo4j
    else
        docker compose up -d neo4j
    fi
    printf "${GREEN}✓ Neo4j iniciado com sucesso no Docker!${NC}\n"
else
    printf "${RED}Erro: Arquivo docker-compose.yml não foi encontrado no diretório atual.${NC}\n"
    printf "Por favor, execute este script a partir da raiz do projeto rootmap.\n"
    exit 1
fi

# 6. Instalar o binário globalmente no PATH
printf "\n${YELLOW}[5/5] Instalando rootmap-cli globalmente no PATH do sistema (/usr/local/bin)...${NC}\n"
BIN_SOURCE=""
if [ -f "target/release/rootmap" ]; then
    BIN_SOURCE="target/release/rootmap"
elif [ -f "../rootmap-cli" ]; then
    BIN_SOURCE="../rootmap-cli"
elif [ -f "rootmap-cli" ]; then
    BIN_SOURCE="rootmap-cli"
fi

if [ -n "$BIN_SOURCE" ]; then
    cp "$BIN_SOURCE" /usr/local/bin/rootmap-cli
    chmod +x /usr/local/bin/rootmap-cli
    printf "${GREEN}✓ Binário instalado em /usr/local/bin/rootmap-cli${NC}\n"
else
    printf "${RED}Aviso: Não foi possível encontrar o binário compilado do rootmap para instalar globalmente.${NC}\n"
    printf "Compile primeiro usando 'cargo build --release' ou certifique-se de que o arquivo rootmap-cli existe.\n"
fi

printf "\n${GREEN}=====================================================${NC}\n"
printf "${GREEN}          Instalação Concluída com Sucesso!          ${NC}\n"
printf "${GREEN}=====================================================${NC}\n"
printf "\nVocê agora pode executar a ferramenta de qualquer pasta digitando: ${YELLOW}rootmap-cli <comando>${NC}\n"
printf "\n${YELLOW}ATENÇÃO: É necessário reiniciar a sessão do seu Linux (ou reiniciar o PC)${NC}\n"
printf "para que as novas permissões do Docker no grupo do seu usuário façam efeito.\n"
printf "Depois de reiniciar, você poderá rodar o docker sem usar sudo.\n\n"
