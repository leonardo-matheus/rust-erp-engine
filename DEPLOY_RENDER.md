# Deploy na Render.com (Docker)

## Configuração no Render

1. Acesse o painel da Render e crie um novo **Web Service**
2. Selecione **Deploy from a Git repository**
3. Configurações:

| Campo | Valor |
|-------|-------|
| **Runtime** | Docker |
| **Dockerfile Path** | `Dockerfile` |
| **Build Context** | `.` |
| **Instance Type** | Starter ou superior (o plano Free não suporta persistent disk) |

## Variáveis de Ambiente

Configure as seguintes variáveis de ambiente no painel da Render:

| Variável | Descrição | Exemplo |
|----------|-----------|---------|
| `DATABASE_URL` | URL de conexão com o banco MySQL remoto | `mysql://user:pass@host:3306/dbname` |
| `JWT_SECRET` | Chave secreta para assinatura JWT (mínimo 32 caracteres) | *(gerar chave segura)* |
| `JWT_REFRESH_SECRET` | Chave secreta para tokens de refresh (mínimo 32 caracteres) | *(gerar chave segura)* |
| `AES_KEY` | Chave AES-256 para criptografia em repouso (exatamente 32 bytes em hex) | *(gerar chave segura)* |
| `RUST_LOG` | Nível de log | `info` |

**NÃO configure** a variável `PORT`. Ela é injetada automaticamente pela Render e o backend a utiliza para definir a porta de escuta. Configurá-la manualmente pode causar conflitos.

## Sobre o Health Check

O backend é um servidor **gRPC puro** (tonic). O Render Web Service espera uma resposta HTTP 200 em uma rota de health check, mas este servidor não expõe endpoints HTTP/1.1.

**Solução:** No painel da Render, na seção **Health Check Path**, deixe em branco para **desabilitar** o health check. Alternativamente, você pode configurar um health check gRPC se a Render suportar na sua versão atual.

O servidor já possui um health check gRPC implementado no serviço `HealthService` (proto `erp.proto`).

## Sobre o Storage (Sled)

O backend usa o engine **Sled** (B+Tree lock-free, pure Rust) como armazenamento local. Sled grava dados em disco no diretório configurado via `SLED_PATH` (padrão: `./techfix_data`).

**No plano Free da Render, não há persistent disk.** Isso significa que os dados locais do Sled são perdidos a cada reinicialização do container. Para produção, utilize o banco MySQL remoto via `DATABASE_URL`.

## Sobre o MySQL Remoto

O backend pode se conectar a um MySQL externo. O host do banco **não pode ser `localhost`** dentro do container Render — ele precisa ser um endereço acessível externamente.

Exemplo de `DATABASE_URL`:
```
mysql://usuario:senha@h64.servidorhh.com:3306/nome_banco
```

**Requisitos:**
- O servidor MySQL deve aceitar conexões remotas (verifique as regras de firewall/grupo de segurança)
- O host deve ser um domínio público ou IP público (não `localhost` nem `127.0.0.1`)
- O banco de dados e usuário devem existir previamente

## Build e Deploy

### Local (Docker)
```bash
docker build -t rust-erp-engine-render .
docker run -p 50051:10000 \
  -e DATABASE_URL="mysql://..." \
  -e JWT_SECRET="sua-chave-aqui" \
  -e JWT_REFRESH_SECRET="sua-chave-refresh-aqui" \
  -e AES_KEY="sua-chave-aes-32-bytes-hex" \
  rust-erp-engine-render
```

### Local (cargo)
```bash
cd backend
cargo run --release
```
O servidor local escuta na porta 50051 (ou na porta definida em `PORT` / `GRPC_PORT`).

## Notas Técnicas

- O binário gerado se chama `server`
- A porta padrão local é `50051` (gRPC)
- Em produção (Render), a porta é definida pela variável `PORT`
- O Dockerfile usa build multi-stage para manter a imagem final leve (~80MB)
- A imagem final é baseada em `debian:bookworm-slim`
- Protobuf é necessário apenas no build (stage builder), não na imagem final
