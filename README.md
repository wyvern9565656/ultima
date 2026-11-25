# Ultima

Hub local para os seus repositórios: lista os apps, baixa releases, confere o SHA-256,
extrai o pacote, compila do código com log ao vivo e mantém tudo instalado em um só lugar.

Feito em Rust com `egui`/`eframe` — um binário nativo, sem Electron nem navegador.

## Rodar

```bash
cargo run --release
```

O binário fica em `target/release/ultima`.

Requisitos para compilar: Rust recente (edition 2021), OpenSSL, e as bibliotecas de
janela do sistema (`libxkbcommon`, Wayland ou X11). No Arch:

```bash
sudo pacman -S base-devel openssl libxkbcommon wayland
```

## Repositórios

Os quatro repositórios iniciais já vêm configurados (`wyvern9565656/Nexus`,
`nexus-code`, `ultima-launcher`, `ultima-pulse`). Em **Configurações** (engrenagem no
rodapé da barra lateral) dá para:

- informar um token do GitHub (aumenta o limite de 60 para 5.000 requisições/hora);
- trocar a pasta de instalação (padrão `~/.local/share/ultima/`);
- adicionar outros repositórios no formato `dono/repo`.

O token também pode vir da variável de ambiente `GITHUB_TOKEN`.

## O que ele faz

- **Biblioteca** — cards dos repositórios com busca, filtro por família e ações
  Instalar / Atualizar / Abrir / Compilar.
- **Verificar atualizações** — consulta a API do GitHub, ordena releases e marca o que
  está desatualizado.
- **Instalar release** — escolhe o asset Linux (AppImage, `.tar.gz`, `.zip`, `.deb`),
  baixa com progresso, confere o digest da API ou o arquivo `SHA256SUMS`, extrai e
  localiza o executável.
- **Compilar do código** — clona o branch (raso), detecta o projeto e roda o build com
  log em tempo real:
  - Cargo → `cargo build --release`
  - Python → empacota o projeto com um launcher `.sh`
  - Node → `npm ci`/`npm install` + `npm run build` + launcher
- **Descobrir** — seus outros repositórios no GitHub, prontos para adicionar.
- **Compilações** — histórico dos jobs desta sessão.

## Onde fica o quê

```
~/.local/share/ultima/
├── state.json        # repositórios, token e instalações
├── apps/<slug>/<versão>/   # apps instalados
├── src/<slug>/       # código clonado para compilar
└── cache/            # downloads em andamento
```

Cada app instalado ganha um registro com canal (`release` ou `source`), versão e
executável. Remover a pasta do app desinstala; o estado se ajusta na próxima abertura.

## Testes

```bash
cargo test
```

Cobrem comparação de versões, escolha de asset, leitura de checksums, extração de
`.zip`/`.tar.gz`, descoberta de binário, detecção de sistemas de build e persistência
do estado.
