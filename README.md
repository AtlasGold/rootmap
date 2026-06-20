# rootmap

**Linux Infrastructure Dependency Mapper**

O rootmap é uma ferramenta de linha de comando (CLI) desenvolvida em Rust para mapear dependências de infraestrutura Linux, combinando um armazenamento local relacional em SQLite com um banco de dados de grafos Neo4j. 

A ferramenta realiza:
1. Varredura automática de processos ativos, portas de rede em escuta, contêineres Docker e serviços Systemd.
2. Análise de impacto downstream: identifica quais componentes são afetados a jusante caso um serviço ou porta sofra indisponibilidade.
3. Caminho mais curto: rastreia a linha de dependência estrutural entre dois componentes de infraestrutura.
4. Análise heurística de causa raiz: calcula pontuações de probabilidade para identificar o provável culpado pela queda de um serviço ou porta com base nos fatos coletados.
5. Relatórios forenses: gera relatórios de incidentes no terminal ou exportados em formato Markdown.

O projeto serve como protótipo prático acadêmico para Trabalho de Conclusão de Curso (TCC) focado em correlação de incidentes e arquitetura de dados híbrida (relacional/grafos).

---

## Pré-requisitos

* **Linux**: ambiente de execução nativo (testado em distribuições Linux Mint, Ubuntu e Debian).
* **Docker** e **Docker Compose**: necessários para executar a instância do Neo4j.
* **Rust (opcional)**: necessário apenas se houver necessidade de recompilar o código fonte (`cargo build`).

---

## Instalação Automatizada

Para configurar as dependências de sistema e inicializar o Neo4j automaticamente em sistemas baseados em Debian, Ubuntu ou Linux Mint, execute o script de instalação:

```bash
sudo ./setup.sh
```

O script executa as seguintes etapas:
1. Atualiza os repositórios locais e instala `build-essential`, `pkg-config`, `libssl-dev` e `sqlite3`.
2. Instala o Docker e o Docker Compose se não estiverem presentes no sistema.
3. Adiciona o usuário atual ao grupo `docker` para permitir execução de comandos do Docker sem privilégios de superusuário.
4. Inicializa o contêiner do Neo4j.
5. Copia o binário compilado para `/usr/local/bin/rootmap-cli` facilitando a chamada global do comando.

> **Nota:** Após a execução do script, reinicie a sessão do terminal (ou execute logout/login) para que as permissões do grupo do Docker entrem em vigor.

---

## Manual de Execução

Após a execução do script de instalação, o utilitário `rootmap-cli` estará instalado globalmente.

### 1. Iniciar o Neo4j
Caso o serviço do Neo4j não esteja em execução, inicialize-o com o comando:
```bash
docker compose up -d neo4j
```
O console administrativo do Neo4j estará disponível em: `http://localhost:7474` (Usuário: `neo4j`, Senha: `rootmap123`).

### 2. Inicializar o Banco SQLite Local
Cria o banco de dados relacional local `rootmap.db` no diretório de execução:
```bash
rootmap-cli migrate
```

### 3. Executar Varredura (Scan)
Coleta os processos ativos, serviços, portas de rede e contêineres e persiste na base SQLite local:
```bash
rootmap-cli scan
```

### 4. Importar Relações de Dependência Lógica
Importa a cadeia lógica de relacionamento (ex: nginx depende de app, que depende de postgresql) a partir de um arquivo de configuração YAML:
```bash
rootmap-cli import -f rootmap/examples/inventory.yaml
```

### 5. Sincronizar Relacional com o Grafo
Envia e correlaciona as informações estruturadas da base SQLite local para o Neo4j:
```bash
rootmap-cli sync
```

### 6. Executar Análise de Impacto
Mapeia o efeito em cascata gerado pela queda de um componente:
```bash
rootmap-cli impact --node Service:postgresql
```

### 7. Rastrear Linha de Conexão (Caminho)
Identifica a cadeia de dependências diretas e indiretas ligando dois nós:
```bash
rootmap-cli path --from Service:nginx --to Service:postgresql
```

### 8. Analisar Causa Raiz de Incidentes
Executa a heurística de causa provável baseado no sintoma e no nó afetado:
```bash
rootmap-cli incident analyze --symptom "site fora do ar" --affected Service:nginx
```
Este comando exibirá o identificador único de incidente gerado (UUID).

### 9. Visualizar Relatório
Gera a saída detalhada do último incidente analisado:
```bash
rootmap-cli report --last
```

---

## Visualização no Neo4j Browser

Acesse o console em `http://localhost:7474` e utilize as seguintes queries Cypher para navegar pela topologia:

### Visualizar todos os nós e conexões (Limite de 100):
```cypher
MATCH (n)-[r]->(m) RETURN n, r, m LIMIT 100
```

### Filtrar cadeia de dependência específica entre Nginx e Postgres:
```cypher
MATCH path = (a:Service {id: 'nginx'})-[*..3]-(b:Service {id: 'postgresql'}) 
RETURN path
```

---

## Comandos Disponíveis na CLI

| Subcomando | Descrição |
|------------|-----------|
| `migrate` | Executa as migrations de estrutura no SQLite local |
| `scan` | Varre os metadados do host e salva localmente |
| `import -f <arquivo.yaml>` | Adiciona relações lógicas descritas em formato YAML |
| `sync` | Consolida e sincroniza os dados locais para o banco de grafos Neo4j |
| `impact --node <Type:id>` | Avalia o impacto downstream a partir do nó indicado |
| `path --from <Type:id> --to <Type:id>` | Retorna a menor rota de dependência funcional entre nós |
| `incident analyze --symptom "..." --affected <Type:id>` | Executa a heurística e calcula o scoring de causa provável |
| `report --last` | Formata o relatório de incidentes para a última execução |
| `report --incident <UUID>` | Recupera e exibe relatório de um incidente histórico do SQLite |

---

## Formato de Identificação de Nós (`Type:id`)

As consultas da CLI aceitam strings estruturadas identificando o tipo e o valor do componente:
* `Host:hostname` (Ex: `Host:reactor`)
* `Service:nome` (Ex: `Service:sshd`, `Service:nginx`, `Service:postgresql`)
* `Port:protocolo:numero` (Ex: `Port:tcp:22`, `Port:tcp:80`)
* `Container:nome` (Ex: `Container:rootmap-neo4j`)
* `Process:nome:pid` (Ex: `Process:systemd:1`)
