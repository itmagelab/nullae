# 0ae

[![CI - Security & Tests](https://github.com/itmagelab/nullae/actions/workflows/rust.yml/badge.svg)](https://github.com/itmagelab/nullae/actions/workflows/rust.yml)

## Examples of usage

```
curl "http://localhost:8500/v1/kv/0ae/index/0ae/index/address/127.0.0.1" | jq -r '.[0].Value' | base64 -d
```

Ping to API:

    curl -X POST --header 'Content-Type: application/json' -d '{"url":"http://127.0.0.1:8080/"}' -v https://api.lab.0ae.ru/api/v1/short
