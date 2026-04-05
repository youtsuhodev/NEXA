# NEXA (.nxa)

Langage expérimental et compilateur MVP en Rust (workspace multi-crates).

## Prérequis

Sur Windows, installez les **Build Tools for Visual Studio** (charge de travail « Développement Desktop en C++ ») pour que `link.exe` soit disponible, ou utilisez une cible `x86_64-pc-windows-gnu` si votre toolchain est configurée ainsi.

## Commandes

```bash
cargo build --workspace
cargo run -p nexa-cli -- run path/to/file.nxa
cargo run -p nexa-cli -- check path/to/file.nxa
```

Binaire : `nexa` (`crates/nexa-cli`).

## MVP actuel

- Lexer, parser, AST
- Vérificateur de types minimal (`Int`, `String`, `Void`, `print`, appels de fonctions)
- VM stack + bytecode, exécution via `nexa run`
- `nexa build` / `nexa fmt` : non implémentés

## Exemple

```nxa
fn main() {
    print("Hello NEXA");
}
```

Voir `crates/nexa-cli/tests/fixtures/`.
