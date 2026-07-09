# Apex DTL

Apex DTL es un protocolo seguro de liquidacion determinista escrito en Rust. Su
objetivo es modelar flujos de pago diferido entre pagadores, beneficiarios,
solvers e integradores mediante intents firmados, rutas de ejecucion y
liquidaciones verificables.

El proyecto proporciona un nucleo compacto para simular y validar liquidaciones
Web3 con salidas JSON reproducibles, digests de estado estables y controles de
integridad contable en cada transicion relevante.

## Caracteristicas principales

- Ledger determinista con digest de estado reproducible.
- Identidades publicas derivadas de claves Ed25519.
- Intents firmados por el pagador.
- Rutas de ejecucion vinculadas criptograficamente al intent.
- Solicitudes de liquidacion firmadas por el beneficiario.
- Nonces independientes para apertura y liquidacion.
- Deteccion de transacciones duplicadas.
- Conservacion de suministro despues de cada mutacion critica.
- Escenarios CLI para flujos directos, enrutados, batch y snapshot.
- Tests de integracion en Rust y JavaScript/Bun.

## Arquitectura

El codigo esta separado por dominios:

- `src/amount`: importes y calculos en basis points.
- `src/codec`: serializacion canonica para firmas, digests e IDs.
- `src/crypto`: identidades, firmas Ed25519 y verificacion.
- `src/error`: errores del dominio Apex.
- `src/ids`: identificadores de cuentas, activos, intents, transacciones y
  digests.
- `src/ledger`: cuentas, diario contable, intents bloqueados y transiciones de
  estado.
- `src/order`: politicas de intent, rutas de ejecucion y liquidaciones firmadas.
- `src/runtime`: CLI y escenarios reproducibles.

## Flujo del protocolo

1. El ledger se inicializa con un `network_id`, un activo nativo y cuentas
   registradas.
2. El pagador crea un intent con beneficiario, activo, importe, nonce y politica
   de ejecucion.
3. La ruta seleccionada se resume en un digest y queda incluida en la vista
   firmada por el pagador.
4. El ledger abre el intent, debita el importe al pagador y bloquea el valor.
5. El beneficiario firma una solicitud de liquidacion con el digest de ruta
   observado.
6. El ledger verifica firmas, nonces, rutas, red, activo y estado del intent.
7. La liquidacion acredita al beneficiario y a las partes de ruta segun el plan
   registrado.
8. El ledger confirma la transicion solo si se conserva el suministro total.

## Escenarios disponibles

El binario acepta un argumento opcional. Si no se indica ninguno, ejecuta
`routed`.

```bash
cargo run --locked -- direct
cargo run --locked -- routed
cargo run --locked -- batch
cargo run --locked -- snapshot
```

Cada escenario devuelve un JSON con:

- `scenario`: nombre del escenario.
- `network_id`: red simulada.
- `asset`: activo liquidado.
- `intent_id`: identificador del intent cuando aplica.
- `open_tx`: transaccion de apertura cuando aplica.
- `settlement_tx`: transaccion de liquidacion cuando aplica.
- `balances`: saldos finales por rol.
- `total_supply`: suministro total registrado.
- `state_digest`: digest final del ledger.
- `conservation_ok`: resultado de la comprobacion contable.

## Requisitos

- Rust `1.96.0`.
- Bun `1.3.0` o superior.
- Node.js `24` recomendado para tooling auxiliar.

## Instalacion

```bash
bun install --frozen-lockfile
cargo build --all-targets --locked
```

## Comandos de desarrollo

```bash
bun run test        # tests JavaScript/Bun
bun run test:rust   # tests Rust
bun run test:all    # tests Rust y JavaScript
bun run fmt         # formato JavaScript
bun run fmt:check   # verificacion de formato JavaScript
bun run build       # verificacion de sintaxis JavaScript
bun run ci          # pipeline local completo
```

Tambien se incluyen scripts POSIX:

```bash
bash scripts/tests.sh
bash scripts/ci.sh
```

En Windows, se recomienda ejecutar los scripts con Git Bash. Desde PowerShell,
la ruta mas directa es `bun run ci`.

## Calidad

El pipeline comprueba:

- Formato Rust.
- Build Rust.
- Tests Rust.
- Clippy con warnings como errores.
- Formato JavaScript.
- Sintaxis JavaScript.
- Tests JavaScript/Bun.

Dependabot esta configurado para revisar dependencias de Cargo, Bun y GitHub
Actions.

## Estado

Apex DTL es una implementacion de referencia para entornos controlados,
integraciones internas y validacion de flujos de liquidacion. Cualquier uso en
produccion debe pasar por revision de arquitectura, hardening operativo y
validacion independiente.
