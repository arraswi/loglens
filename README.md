# LogLens

CLI em Rust para resumir arquivos de log JSONL. Lê uma linha por vez, conta níveis e mensagens frequentes, aceita filtro por severidade e continua mesmo quando encontra JSON inválido.

## Recursos

- Processamento em fluxo para não carregar o arquivo inteiro na memória.
- Contagem de registros por nível.
- Filtro opcional por trace, debug, info, warn ou error.
- Mensagens mais frequentes.
- Intervalo de timestamps RFC 3339.
- Relatório em texto ou JSON.
- Linhas inválidas contabilizadas sem interromper a análise.

## Requisitos

- Rust 1.85 ou superior.

## Executar

    cargo run -- sample-logs.ndjson
    cargo run -- sample-logs.ndjson --level error
    cargo run -- sample-logs.ndjson --top 3 --json

Uma linha de log deve ser um objeto JSON. O programa procura timestamp ou time, level ou severity e message ou msg.

## Exemplo de linha

    {"timestamp":"2026-03-02T09:15:00Z","level":"INFO","message":"worker started"}

## Tecnologias

Rust · clap · serde · serde_json · chrono

## Próximos passos

- Exportar CSV para análise posterior.
- Filtrar por intervalo de tempo.
- Adicionar suporte a formatos de log comuns.
- Gerar uma sugestão de alertas a partir de picos de erro.
