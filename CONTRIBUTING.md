# Contribuir a ApexDTL

## Principios

Los cambios deben conservar determinismo, aritmética comprobada y atomicidad. No
se aceptan dependencias de reloj de pared, orden de hash maps, red externa o
aleatoriedad no inyectada en el núcleo.

## Flujo de trabajo

1. Crea una rama desde `main`.
2. Limita el cambio a un dominio coherente.
3. Añade tests de éxito, rechazo y no mutación tras error.
4. Actualiza documentación si cambia una interfaz o parámetro.
5. Ejecuta el pipeline local completo.
6. Abre un pull request con contexto, riesgo y evidencia.

```bash
bun install --frozen-lockfile
bun run ci
```

En PowerShell:

```powershell
./scripts/ci.ps1
```

## Convenciones

- Código, comentarios, nombres y mensajes técnicos: inglés.
- Documentación de producto: español.
- Rust formateado con `cargo fmt`.
- JavaScript formateado con Prettier.
- Ningún `unwrap` o `expect` sobre datos externos.
- Los importes usan `Amount`; los porcentajes normalizados usan `Bps`.
- Toda suma, resta, multiplicación o incremento de nonce debe ser comprobado.
- Las estructuras comprometidas por firma o digest requieren dominio versionado.

## Cambios de estado

Una nueva transición debe documentar:

- quién la autoriza;
- qué payload se firma;
- qué replay protection consume;
- qué valores modifica;
- qué invariantes verifica;
- qué evidencia produce.

## Pull requests

El cuerpo debe incluir objetivo, superficie afectada, compatibilidad, tests y
riesgo operativo. Los cambios de formato no deben mezclarse con cambios de
semántica salvo que sean inseparables.

## Releases

Las versiones siguen SemVer. Una release requiere que `Cargo.toml`,
`package.json`, `src/lib.rs`, el tag `vX.Y.Z` y la rama `production` estén
alineados con el mismo commit validado.
