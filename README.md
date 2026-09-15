# TechFix ERP Engine

Protótipo de ERP para produtos, estoque e vendas, com núcleo WebAssembly executado no navegador e um backend gRPC independente em Rust. O repositório explora contratos Protobuf, persistência chave-valor e controles de autenticação em uma aplicação de domínio transacional.

## Componentes

```text
src/lib.rs                 núcleo WASM com estado no localStorage
index.html                 interface web do protótipo
backend/proto/erp.proto    contrato gRPC
backend/src/services/      autenticação, produtos, vendas, estoque e dashboard
backend/src/store/         trait de persistência e implementação Sled
backend/src/validation.rs  validação das entradas
```

O frontend WASM mantém produtos, vendas e movimentações no `localStorage` do navegador. O backend implementa um modelo semelhante via gRPC e Sled. São dois fluxos de execução presentes no mesmo repositório; o código atual não demonstra sincronização entre o estado local do navegador e o backend.

## Backend

O contrato em `backend/proto/erp.proto` define serviços para:

- login e renovação de tokens;
- criação, consulta, atualização e exclusão de produtos;
- registro e listagem de vendas;
- reposição e histórico de estoque;
- indicadores agregados e health check.

A camada de serviços depende do trait `Store`, e `SledStore` implementa produtos, vendas, estoque, sessões e auditoria. Essa fronteira permite trocar o mecanismo de persistência sem alterar o contrato gRPC, desde que uma nova implementação cubra o trait.

## Decisões técnicas

- Tonic e Protobuf tornam o contrato explícito e geram os tipos de transporte no build.
- Sled mantém a implantação local sem servidor de banco, com a implicação de armazenamento vinculado ao filesystem da instância.
- senhas são verificadas com Argon2 e sessões usam JWT com registro de revogação no store.
- AES-GCM está disponível para criptografia autenticada de campos, mas a presença do módulo não comprova que todo dado persistido esteja cifrado.
- operações mutáveis registram eventos de auditoria; o tratamento atual ignora falhas ao gravar esses eventos para não interromper a operação principal.
- o backend semeia dados de demonstração ao iniciar, comportamento que deve ser revisto em uma implantação real.

## Execução local

### Frontend WASM

Requisitos: Rust, `wasm-pack` e Python 3.

```bash
wasm-pack build --target web --out-dir pkg
python serve.py
```

A interface fica disponível em `http://localhost:8080`.

### Backend gRPC

Requisitos: Rust, Cargo e o compilador Protocol Buffers (`protoc`), incluindo seus tipos padrão. `backend/build.rs` gera o código de transporte a partir do contrato Protobuf.

Configure `JWT_SECRET` e `JWT_REFRESH_SECRET` no ambiente com segredos aleatórios de pelo menos 32 bytes. Configure também `AES_KEY` com exatamente 32 bytes UTF-8: a aplicação usa o texto diretamente, sem decodificar hexadecimal ou Base64. Preserve esses valores entre reinicializações.

```bash
cd backend
cargo run --release --bin server
```

O servidor escuta em `0.0.0.0:50051` por padrão. `PORT` ou `GRPC_PORT` alteram a porta; `SLED_PATH` altera o diretório de dados.

## Verificação

```bash
cargo test
cd backend
cargo test
```

Os testes do backend cobrem autenticação, criptografia, validação, configuração e operações do store. O repositório não inclui artefatos suficientes para sustentar promessas universais de latência, throughput ou conformidade regulatória; qualquer avaliação desse tipo exige ambiente, carga e auditoria reproduzíveis.

## Limites atuais

- o processo expõe gRPC puro; não há gateway HTTP/REST implementado;
- variáveis de TLS, CORS, rate limit e porta gRPC-Web existem na configuração, mas não estão conectadas ao `Server` em `backend/src/main.rs`;
- a instância de `CorsLayer` criada no backend não é aplicada ao servidor;
- armazenamento Sled local exige volume persistente para sobreviver a recriações do container;
- o serviço `AuditService` aparece no Protobuf, mas não é registrado na composição atual do servidor;
- este é um protótipo técnico, sem evidência de uso em produção ou certificação de segurança;
- o repositório não contém um arquivo de licença.
