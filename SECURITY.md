# Politica de proteccion

Apex DTL esta disenado como un protocolo seguro de liquidacion determinista. Su
modelo combina autorizaciones criptograficas, serializacion canonica, control de
nonces, identificadores derivados y comprobaciones contables de suministro.

## Alcance

Este documento cubre:

- Apertura de intents.
- Liquidacion de intents.
- Validacion de identidades publicas.
- Firmas Ed25519 por dominio.
- Nonces de pagador y beneficiario.
- Rutas de ejecucion vinculadas al intent.
- Derivacion de IDs, transacciones y digests.
- Conservacion de suministro del ledger.
- Tests y pipeline de validacion del repositorio.

No cubre:

- Custodia de claves privadas fuera del proceso.
- Infraestructura cloud.
- Integraciones on-chain externas.
- Persistencia distribuida.
- Gestion de usuarios o permisos del sistema operativo.

## Modelo de confianza

El protocolo asume que:

- Las claves privadas se generan y almacenan fuera del ledger.
- Cada cuenta registrada corresponde a una identidad publica consistente.
- Los participantes firman solo payloads revisados por su cliente o integrador.
- Los integradores ejecutan builds reproducibles con lockfiles actualizados.
- Los reportes JSON se usan como salida de verificacion, no como fuente externa
  de autoridad.

## Controles del protocolo

### Dominios de firma

La apertura de intents y la liquidacion usan dominios de firma separados. Esta
separacion evita reutilizar una autorizacion en una accion con significado
distinto.

### Rutas vinculadas al intent

El digest de ruta forma parte de la vista firmada por el pagador. Cualquier
cambio posterior en solver, receptores, importes, credito, lane o quote nonce
produce una vista de autorizacion distinta.

### Nonces independientes

El pagador usa nonces de apertura y el beneficiario usa nonces de liquidacion.
El ledger valida el valor esperado antes de aceptar una transicion.

### Identificadores deterministas

Los IDs de intents, transacciones y estados se derivan con dominios explicitos y
datos canonicos. Esto facilita reproducibilidad y auditoria de cambios de
estado.

### Conservacion de suministro

El ledger confirma mutaciones sobre una copia candidata y solo persiste cambios
si la suma de saldos liquidos mas valor bloqueado coincide con el suministro
total registrado.

### Deteccion de duplicados

Cada transaccion aceptada queda registrada en el conjunto de transacciones
observadas. Las transacciones repetidas se rechazan antes de modificar el
estado.

## Practicas operativas

Antes de integrar cambios, ejecutar:

```bash
bun run ci
```

El pipeline local y de GitHub Actions comprueba formato, build, tests, Clippy y
contratos de integracion.

Para una validacion rapida:

```bash
bun run test:all
```

## Gestion de dependencias

El repositorio mantiene `Cargo.lock` y `bun.lock` para ejecuciones
reproducibles. Dependabot revisa periodicamente:

- Cargo.
- Bun.
- GitHub Actions.

Las actualizaciones deben validarse con el pipeline completo antes de ser
integradas.

## Reporte privado

Los hallazgos tecnicos deben comunicarse de forma privada al mantenedor del
proyecto. Un reporte util incluye:

- Version o commit analizado.
- Sistema operativo.
- Versiones de Rust y Bun.
- Pasos de reproduccion.
- Impacto tecnico estimado.
- Propuesta de mitigacion, si aplica.

No deben publicarse detalles operativos antes de que el mantenedor haya podido
revisar y coordinar la respuesta.

## Criterios de prioridad

- Critica: liquidaciones no autorizadas o ruptura de invariantes contables.
- Alta: bypass de firmas, reutilizacion de autorizaciones o corrupcion
  persistente del estado.
- Media: denegacion de servicio local, rechazo incorrecto de flujos validos o
  inconsistencias reproducibles en escenarios soportados.
- Baja: problemas de mensajes, configuracion, documentacion o tooling sin
  impacto directo en la liquidacion.

## Limitaciones

Apex DTL no implementa custodia de claves, consenso distribuido, persistencia
tolerante a fallos ni integracion directa con redes publicas. Para uso en
produccion se requiere hardening del entorno, gestion formal de claves,
monitorizacion y revision independiente del despliegue.
