# Status do Protótipo rootmap para o TCC

## Funcionalidades Entregues

1. **Varredura Local de Infraestrutura Linux**
   * Coleta de hostname, sistema operacional e versão do kernel.
   * Listagem de processos ativos em execução no host.
   * Filtro ativo de processos prioritários (ex: nginx, postgres, docker, sshd).
   * Detecção de estado de serviços gerenciados pelo Systemd (via systemctl).
   * Inventário de contêineres ativos executados no Docker (via API local).
   * Mapeamento de portas TCP/UDP em estado de escuta (listening).

2. **Camada de Persistência Relacional Local (SQLite)**
   * Estruturação de dados contendo 9 tabelas relacionais com chaves primárias e integridade referencial: scan_runs, hosts, processes, systemd_services, docker_containers, listening_ports, dependencies, incidents e incident_findings.
   * Registro histórico de coletas locais.
   * Suporte para persistência de dependências declarativas (YAML) e operacionais detectadas.

3. **Sincronização Topológica para Grafo (Neo4j)**
   * Geração automatizada de nós representativos: Host, Process, Service, Container, Port.
   * Construção de relacionamentos direcionados: RUNS_PROCESS, RUNS_SERVICE, RUNS_CONTAINER, LISTENS_ON, DEPENDS_ON, MANAGES_PROCESS.
   * Aplicação automática de constraints de unicidade de identificação.

4. **Motor de Análise de Impacto**
   * Travessia topológica recursiva a jusante (downstream) para determinação de impacto em cascata.
   * Formatação de saída estruturada em árvore de texto ASCII e tabelas organizadas.

5. **Cálculo de Caminhos de Dependência**
   * Determinação do menor trajeto relacional entre componentes utilizando algoritmo nativo shortestPath via driver Bolt.

6. **Mapeamento Heurístico de Incidentes**
   * Algoritmo de scoring aditivo para ranqueamento de hipóteses de causa raiz (upstream).
   * Geração de relatórios com as principais causas candidatas e seus respectivos pesos de probabilidade.

7. **Configuração e Instalação Simplificada**
   * Script automatizado bash (setup.sh) que instala as dependências necessárias, configura o grupo docker e instala o binário de execução globalmente.

---

## Conexão Teórica com o TCC

O protótipo serve como validação experimental do modelo híbrido relacional-grafo proposto no TCC:

* **SQLite** (modelo relacional): Demonstra eficácia no armazenamento estruturado de registros brutos, permitindo auditorias temporais, consistência ACID local e facilidade de deploy (banco local portátil armazenado em um único arquivo de banco de dados).
* **Neo4j** (modelo em grafo): Valida a eficiência na execução de travessias profundas e consultas relacionais complexas (caminho crítico e árvore de colapso downstream), minimizando o custo computacional que seria gerado por múltiplas operações recursivas em SQL convencional.

---

## Atividades Pendentes para Conclusão do Trabalho Acadêmico

### Execução de Ensaios e Coleta de Métricas
* Executar simulações em ambiente controlado (ex: derrubar o serviço do PostgreSQL ou Nginx e validar se a CLI mapeia corretamente a topologia colapsada).
* Avaliar o tempo médio de identificação e diagnóstico de causa raiz por operadores humanos em comparação com o uso do rootmap.
* Medir a acurácia (precisão e revocação) do mapeamento de dependências.
* Avaliar a latência de consultas topológicas no Neo4j com o crescimento do grafo de conexões.

### Consolidação e Documentação de Resultados
* Organizar tabelas e gráficos comparativos baseados nas rodadas de teste locais.
* Capturar evidências operacionais (logs e representações gráficas geradas no console do Neo4j Browser) para ilustrar o capítulo de discussões do TCC.
* Documentar a especificação do hardware e versões do sistema operacional do laboratório de testes.

---

## Recomendações de Integridade Científica

A integridade dos dados e da metodologia do protótipo é fundamental para a validação da pesquisa académica. É recomendado:
* Evitar aproximações arbitrárias nos tempos de identificação de falha; utilizar logs do sistema e timestamps do SQLite para mensurar o tempo exato.
* Evitar extrapolações no potencial da heurística; documentar de forma clara que o cálculo de causa raiz é baseado em scoring determinístico, não em machine learning preditivo.
* Mapear de forma transparente as restrições da ferramenta, tais como a dependência do arquivo compose inicial e o escopo local de escaneamento.

---

## Roteiro Rápido de Testes para Validação do TCC

1. Configurar infraestrutura local:
   ```bash
   sudo ./setup.sh
   ```

2. Aplicar estrutura do banco de dados relacional:
   ```bash
   rootmap-cli migrate
   ```

3. Executar escaneamento de ativos:
   ```bash
   rootmap-cli scan
   ```

4. Alimentar matriz de dependências de teste:
   ```bash
   rootmap-cli import -f rootmap/examples/inventory.yaml
   ```

5. Executar sincronização topológica:
   ```bash
   rootmap-cli sync
   ```

6. Executar diagnósticos de teste:
   ```bash
   rootmap-cli impact --node Service:postgresql
   rootmap-cli path --from Service:nginx --to Service:postgresql
   rootmap-cli incident analyze --symptom "indisponibilidade no sistema" --affected Service:nginx
   ```
