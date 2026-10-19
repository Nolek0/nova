# NOVA AI

Assistant personnel local pour **macOS** et **Windows**, gratuit et sans compte. NOVA discute, calcule, lit tes documents, consulte la météo, cherche dans tes dossiers et agit sur ton ordinateur, toujours avec ton accord. L'IA tourne sur ta machine.

## Ce que fait NOVA aujourd'hui

| Domaine | Fonctions | Où ça tourne |
|---|---|---|
| Conversation | Réponses diffusées en direct, Markdown, code coloré, copier, régénérer, modifier et renvoyer, historique, recherche plein texte | Local |
| IA | llama.cpp intégré + modèles libres à télécharger d'un clic (Gemma 3, Qwen3, gpt-oss), Ollama s'il est installé, modèle Apple sur Mac | Local |
| Mode Internet | Bouton « Internet » : réponses appuyées sur Wikipédia, sources citées | Internet (Wikipédia) |
| Maths | Calculatrice, équations (étapes et vérification), systèmes, dérivées, graphiques, statistiques, probabilités, matrices, conversions d'unités | Local |
| Météo | Actuelle, 24 h, 7 jours ; hors ligne, dernière donnée affichée comme ancienne | Internet (Open-Meteo) |
| Fichiers | Recherche par nom et contenu, limitée aux dossiers que tu autorises | Local |
| Actions | Ouvrir une app, un site, un fichier ; créer une note. Toujours affichées puis exécutées seulement après « Autoriser » | Local |
| Documents et images | PDF et textes lus localement ; images vues par les modèles de vision (Gemma 3), OCR Apple sur Mac | Local |
| Caméra | Mac : photo avec l'iPhone (Caméra de continuité) ; partout : webcam | Local |
| Voix | Lecture des réponses (voix du système) ; dictée locale sur Mac | Local |
| Données | Mémoire choisie par toi, export JSON, suppression, journal d'activité | Local |
| Mini-chat (Mac) | Icône NOVA dans la barre des menus : un clic ouvre une petite fenêtre flottante où l'on pose ses questions sans quitter l'app en cours. « Ouvrir dans NOVA » reprend la conversation dans la grande fenêtre | Local |
| Téléchargements | Uniquement sur ton clic, avec barre de progression (vitesse, temps restant, annulation) et reprise automatique si la connexion coupe | Internet |

## Installer et lancer

### macOS
```bash
./scripts/build-mac.sh
```
Prérequis : Command Line Tools (`xcode-select --install`) et Rust (rustup.rs). L'app est créée dans `app/src-tauri/target/release/bundle/macos/NOVA.app`, et l'installeur dans `…/bundle/dmg/`.

Au premier lancement, ouvre la page **Modèles** : installe le moteur (12 Mo), puis un modèle. NOVA recommande celui qui convient à la mémoire de ta machine.

### Windows
La version Windows se compile gratuitement sur GitHub. Le fichier [.github/workflows/build.yml](.github/workflows/build.yml) fait tout :
1. Crée un dépôt GitHub (gratuit) et envoies-y ce dossier.
2. Onglet **Actions** › **Build Windows** › **Run workflow**.
3. Télécharge l'installeur `NOVA-Windows` dans les « Artifacts » de l'exécution.

Sur un PC Windows avec Rust installé, tu peux aussi compiler directement : `cd app/src-tauri` puis `cargo install tauri-cli` puis `cargo tauri build`.

## Architecture

```
NovaAI/
├── app/                  Application multiplateforme (Tauri 2)
│   ├── src-tauri/        Cœur en Rust
│   │   └── src/
│   │       ├── chat.rs       routage outil/modèle, contexte, flux de réponse
│   │       ├── db.rs         SQLite + recherche plein texte
│   │       ├── llm/          llama.cpp (installation, modèles, serveur), Ollama, pont Apple
│   │       ├── math/         calcul, équations, dérivées, unités, statistiques
│   │       └── tools/        météo, fichiers, actions, Wikipédia, pièces jointes, système
│   └── ui/               Interface (HTML/CSS/JS sans outil de build)
├── apple/                Module Swift pour macOS (modèle Apple, Caméra de continuité, OCR, dictée)
├── scripts/              build-mac.sh, build-apple-helper.sh
└── docs/                 Analyse et architecture
```

Le cœur Rust contient toute la logique. L'interface ne fait qu'afficher et appeler ses commandes. Les fonctions propres à Apple passent par un petit programme Swift embarqué dans l'app Mac (`nova-apple-helper`).

## Sécurité et confidentialité

- Tout est stocké localement : `~/Library/Application Support/local.nova.ai` sur Mac, `%APPDATA%\local.nova.ai` sur Windows.
- Rien ne part sur Internet sans action de ta part. Les réponses qui ont utilisé Internet portent un badge.
- Aucune commande système n'est exécutée par l'IA. Les actions possibles sont limitées et toujours confirmées.
- L'accès aux fichiers est limité aux dossiers autorisés.
- Le contenu des documents et des pages Web est transmis au modèle comme une donnée, jamais comme une instruction.

## Tests

```bash
cd app/src-tauri && cargo test --lib
```
Le module Apple a ses propres tests : `cd apple && ./scripts/test.sh`.

## Limites connues

- La version Windows est compilée par GitHub, mais elle n'a pas encore été testée sur un vrai PC.
- La dictée n'existe que sur Mac pour l'instant (sur Windows : Whisper, prévu).
- La photo avec l'iPhone repose sur la Caméra de continuité d'Apple : même identifiant Apple, Wi-Fi et Bluetooth activés.
- Pas encore d'intégrales ni de limites symboliques.
- Le modèle Apple est petit et ses garde-fous sont stricts. Pour des réponses de qualité, utilise un modèle llama.cpp.
