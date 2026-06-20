# Arquitetura do rootmap

## Visão Geral

O rootmap implementa uma **arquitetura híbrida relacional–grafo** para o mapeamento e análise de dependências em infraestrutura Linux. A persistência local estruturada é feita em **SQLite**, enquanto as operações de travessia topológica utilizam o **Neo4j**.

```
┌─────────────────────────────────────────────────────┐
│                    rootmap CLI                       │
│  scan │ import │ sync │ impact │ path │ incident     │
│  ───┬──────┬────────┬─────────┬────────┬─────────────┘
│     │      │        │         │        │
│     ▼      ▼        │         │        │
│ ┌──────────────┐    │    ┌────┴────────┴──────────────┐
│ │  Coletores   │    │    │     Módulo de Análise      │
│ │  ─ linux     │    │    │  ─ impact (impacto)        │
│ │  ─ systemd   │    │    │  ─ path (caminho)          │
│ │  ─ docker    │    │    │  ─ incident (causa raiz)   │
│ │  ─ ports     │    │    └────────────┬───────────────┘
│ └──────┬───────┘    │                 │
│        │            │                 │
│        ▼            ▼                 ▼
│ ┌──────────────────────┐    ┌─────────────────────┐
│ │      SQLite          │───▶│      Neo4j          │
│ │(persistência local)  │    │  (grafo de deps)    │
│ │                      │    │                     │
│ │  ─ scan_runs         │    │  Nós: Host, Process,│
│ │  ─ hosts             │    │  Service, Container,│
│ │  ─ processes         │    │  Port, Incident     │
│ │  ─ systemd_services  │    │                     │
│ │  ─ docker_containers │    │  Relações:          │
│ │  ─ listening_ports   │    │  RUNS_*, LISTENS_ON,│
│ │  ─ dependencies      │    │  DEPENDS_ON,        │
│ │  ─ incidents         │    │  MANAGES_PROCESS,   │
│ │  ─ incident_findings │    │  AFFECTS, RELATED_TO│
│ └──────────────────────┘    └─────────────────────┘
```

## Por que SQLite?

O SQLite foi adotado como a camada de persistência relacional do protótipo por vários fatores técnicos:

1. **Portabilidade e Autossuficiência**: Por ser uma biblioteca *serverless* que armazena todo o banco de dados em um único arquivo local (`rootmap.db`), elimina a necessidade de gerenciar credenciais, portas e processos de um servidor relacional externo (como PostgreSQL ou MySQL).
2. **Armazenamento Estruturado**: Os dados operacionais coletados durante a varredura (processos, serviços, portas) têm schemas bem definidos e se beneficiam do modelo relacional clássico.
3. **Histórico Temporal e Integridade**: Chaves estrangeiras e relacionamentos garantem integridade referencial nativa entre execuções de scan (`scan_runs`), hosts e entidades coletadas, permitindo mapeamento histórico confiável.
4. **Desempenho e Recursos**: O consumo de memória RAM do SQLite é mínimo (frações de megabytes), permitindo que a CLI execute em ambientes de restrição de hardware sem overhead operacional.

## Por que Neo4j?

O Neo4j é utilizado como a base do motor de grafos topológico:

1. **Eficiência em Travessias Downstream**: Identificar impactos em cascata ("quais nós quebram se X cair?") é expresso como busca recursiva de nós no grafo, sendo mais simples do que consultas JOIN recursivas em SQL tradicional.
2. **Cálculo de Menor Caminho**: O cálculo de dependência funcional utiliza a função nativa `shortestPath` do Neo4j, fornecendo o trajeto ótimo de conexões físicas e lógicas de forma nativa.
3. **Expressividade das Relações**: Diferentes tipos de relacionamentos (ex: `LISTENS_ON`, `DEPENDS_ON`) carregam metadados como nível de confiança e origem, mapeando a infraestrutura de forma flexível.

## Fluxo de Dados

```
1. SCAN          2. STORE         3. SYNC          4. ANALYZE
┌─────────┐    ┌──────────┐    ┌──────────┐    ┌──────────────┐
│ Linux   │───▶│  SQLite  │───▶│  Neo4j   │───▶│ Impact/Path/ │
│ System  │    │          │    │          │    │ Incident     │
└─────────┘    └──────────┘    └──────────┘    └──────────────┘
                    ▲
                    │
              ┌──────────┐
              │  YAML    │
              │ (import) │
              └──────────┘
```

### 1. Varredura (`rootmap-cli scan`)
* Coleta de forma nativa metadados do SO host utilizando `sysinfo`, listagem de serviços do `systemd` e tabelas de conexões via socket (`ss`).
* O resultado é guardado sob um novo `scan_run` na base SQLite.

### 2. Importação (`rootmap-cli import`)
* O importador lê arquivos YAML que mapeiam dependências lógicas que não podem ser inferidas passivamente do sistema operacional.

### 3. Sincronização (`rootmap-cli sync`)
* Lê a base SQLite e populariza/atualiza o modelo topológico no Neo4j de forma idempotente usando queries `MERGE`.

### 4. Análise (`rootmap-cli impact`, `rootmap-cli path`, `rootmap-cli incident analyze`)
* Consome o motor de grafos Bolt no Neo4j para executar as lógicas de travessia topológica downstream, caminhos diretos e heurística de causa provável.

## Algoritmo de Scoring Heurístico

O subcomando `incident analyze` utiliza um algoritmo multicritério com pesos fixos para ranquear causas prováveis de falha:

| Critério | Peso Aditivo |
|----------|--------------|
| Condição Base (Ser nó upstream) | +0.30 |
| Adjacência Direta (depth=1) | +0.20 |
| Adjacência Indireta Curta (depth=2) | +0.12 |
| Adjacência Indireta Média (depth=3) | +0.08 |
| Status operacional com erro (failed/unhealthy) | +0.15 |
| Alto grau de dependentes downstream (>2) | +0.10 |
| Identificado como serviço crítico (ex: postgres, nginx) | +0.10 |
| Multiplicador de Confiança da relação | * Coeficiente de Confiança (0.0 a 1.0) |

## Limitações do Escopo do Protótipo

1. **Varredura Local**: O escopo da CLI limita-se a varreduras no host de execução, sem capacidade de varreduras remotas ativas nativas (ssh/agentes).
2. **Dependência Heurística**: A correlação automática de conexões baseia-se em portas e processos compartilhados, gerando hipóteses que requerem validação humana.
3. **Snapshot Estático**: Cada execução do `scan` gera um retrato temporal pontual. Não há daemon residente em background executando monitoramento contínuo em tempo real.
4. **Sincronização Full-Replace**: O processo de sincronização atual atualiza todos os nós correspondentes.
