# API e integración

## Superficie pública

La crate exporta tipos de dominio desde `apex_dtl`. Las integraciones deben
evitar construir bytes firmados manualmente y usar los constructores y métodos
de firma expuestos.

## Inicialización del ledger

```rust
use apex_dtl::{Amount, ApexLedger, ApexResult, AssetId, KeyPair};

fn initialize() -> ApexResult<ApexLedger> {
    let treasury = KeyPair::from_seed([7_u8; 32]);
    let mut ledger = ApexLedger::new(77, AssetId::native());
    ledger.register_account(treasury.public_identity())?;
    ledger.credit_genesis(
        treasury.public_identity().account,
        Amount::new(1_000_000)?,
    )?;
    ledger.set_epoch(1)?;
    ledger.verify_conservation()?;
    Ok(ledger)
}
```

`credit_genesis` incrementa saldo y suministro en la misma transición. Debe
usarse únicamente durante la construcción del estado inicial acordado.

## Apertura de intent

El cliente construye política, ruta y términos; después firma la vista completa.

```rust
use apex_dtl::{
    Amount, Bps, Digest, IntentPolicy, IntentTerms, RoutePlan, SignedIntent,
};

let venue = Digest::from_parts("venue", &[b"primary"]);
let policy = IntentPolicy::new(venue, Bps::new(100)?, 4, 10, 40);
let route = RoutePlan::direct(solver_account, venue, 12, 9);
let terms = IntentTerms::new(
    ledger.network_id(),
    payer.public_identity().account,
    beneficiary.public_identity().account,
    ledger.asset(),
    Amount::new(250_000)?,
    ledger.intent_nonce(payer.public_identity().account)?,
    policy,
    Digest::from_parts("client-order", &[b"2026-08-14-0001"]),
)?;
let signed = SignedIntent::sign(terms, route, &payer)?;
let open_tx = ledger.open_intent(&signed)?;
```

El integrador debe conservar `intent_id`, `open_tx`, digest de ruta, época y
versión. Reintentar el mismo payload después de una respuesta confirmada produce
un rechazo por nonce o replay.

## Confirmación y liquidación

La confirmación pertenece al beneficiario y referencia el digest de la ruta
observada. Antes de firmar, el cliente debe mostrar red, activo, intent y digest
al firmante.

```rust
use apex_dtl::{SettlementRequest, SignedSettlement};

let request = SettlementRequest::new(
    ledger.network_id(),
    terms.intent_id,
    beneficiary.public_identity().account,
    ledger.settlement_nonce(
        beneficiary.public_identity().account,
    )?,
    route.route_digest()?,
);
let signed_settlement = SignedSettlement::sign(request, &beneficiary)?;
let settlement_tx = ledger.settle_intent(&signed_settlement)?;
```

Tras éxito se recomienda guardar `settlement_tx`, `state_digest` y checkpoint.
Tras error no se debe asumir mutación parcial.

## Riesgo

```rust
use apex_dtl::{
    Amount, Bps, CorridorLimits, PortfolioExposure, RiskEngine, RiskSignals,
    RiskWeights,
};

let decision = RiskEngine::new(RiskWeights::default())?.assess(
    Amount::new(500_000)?,
    PortfolioExposure {
        corridor_open: Amount::new(2_000_000)?,
        portfolio_open: Amount::new(10_000_000)?,
    },
    RiskSignals {
        finality_bps: 1_500,
        liquidity_bps: 3_000,
        counterparty_bps: 2_500,
        operational_bps: 1_000,
    },
    CorridorLimits {
        max_principal: Amount::new(1_000_000)?,
        max_concentration_bps: Bps::new(3_000)?,
        min_collateral_bps: 12_000,
    },
)?;
```

`accepted` resume los límites cuantitativos; `reasons` permite registrar las
causas de rechazo sin inferirlas a partir de strings de error.

## Precio

`PricingEngine::quote` devuelve componentes e identificador reproducible. El
resultado económico no modifica el ledger por sí mismo; el adaptador decide qué
parámetros aprobados incorpora a una orden.

```bash
cargo run --locked -- quote | jq .
```

## Controles de flujo

`FlowGuard` recibe una configuración validada. Cada `FlowRequest` contiene ID,
tipo, sujeto, importe, época y aprobadores. Las aprobaciones duplicadas se
deduplican y solo cuentan identidades autorizadas.

Los registros aceptados pueden exportarse con `records()`. La colección conserva
el payload y el modo vigente al autorizarlo.

## Gobierno

El ciclo normal es:

```text
submit -> approve... -> queue -> timelock -> execute
                            \-> cancel (guardian)
```

Todas las operaciones reciben una época explícita. El caller debe sincronizarla
con la época operativa del ledger y rechazar entradas retrasadas antes de invocar
el gobierno.

## Checkpoints

```rust
use apex_dtl::StateCheckpoint;

let checkpoint = StateCheckpoint::build(&ledger, 1, None)?;
checkpoint.verify(&ledger)?;
let next = StateCheckpoint::build(
    &ledger,
    2,
    Some(checkpoint.checkpoint_digest),
)?;
```

`sequence` empieza en uno. La persistencia debe impedir secuencias repetidas o
saltos; el tipo verifica contenido, mientras que la política de continuidad
pertenece al coordinador de checkpoints.

## CLI

| Comando | Salida | Uso |
| --- | --- | --- |
| `direct` | `ScenarioReport` | Ruta directa completa |
| `routed` | `ScenarioReport` | Ruta con componentes |
| `batch` | reporte agregado | Varias liquidaciones |
| `snapshot` | estado inicial | Baseline determinista |
| `quote` | `EconomicQuote` | Desglose económico |
| `checkpoint` | checkpoint + conteos | Evidencia de estado |
| `version` | versión | Diagnóstico de despliegue |

Un comando desconocido termina con código no cero y no produce un reporte
parcial válido.

## Tratamiento de errores

Las APIs mutables devuelven `ApexResult<T>`. Conviene clasificar variantes, no
comparar el texto del mensaje. Categorías habituales:

- autenticación o autorización;
- política de red, activo, venue o época;
- nonce, replay o estado terminal;
- saldo o aritmética;
- configuración de riesgo, gobierno o flujo;
- conservación y checkpoint.

Los logs no deben incluir seeds, claves privadas ni payloads sin minimización.
