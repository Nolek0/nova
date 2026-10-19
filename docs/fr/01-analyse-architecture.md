# NOVA AI — Phase 1 : analyse technique et architecture

Date : 19 octobre 2026. Tout ce qui est marqué **vérifié** a été testé sur ta machine pendant cette analyse.

## 1. Ce que j'ai mesuré sur ton Mac

| Élément | Résultat | Conséquence |
|---|---|---|
| Machine | MacBook Pro, Apple M5, 10 cœurs (4 perf. + 6 eff.), **16 Go RAM** | Bon pour l'IA locale, mais limite les gros modèles (≤ 8 Md de paramètres confortablement) |
| Système | macOS 27.0.1 | Accès aux frameworks Apple récents |
| Disque | 702 Go libres | Aucun problème de stockage |
| Swift | 6.3.3 (Command Line Tools) | **vérifié** : une app SwiftUI compile avec Swift Package Manager |
| Xcode | **absent** | Voir §4 : pas d'app iOS, pas de WeatherKit, packaging `.app` fait par script |
| Modèle Apple sur l'appareil (Foundation Models) | **disponible et testé** : réponse « Paris » générée localement | Un vrai LLM local, sans téléchargement |
| Ollama / llama.cpp | absents | Optionnels, à installer seulement si tu le décides |
| Python | 3.15.0rc2 (version préliminaire), sans SymPy | SymPy à installer dans un environnement isolé, après ton accord |
| SQLite | 3.54 système | Stockage local sans dépendance |
| Frameworks présents | FoundationModels, Speech, AVFoundation, Vision, NaturalLanguage, CoreLocation, Security | Voix, OCR, caméra, trousseau : tout est natif |

## 2. Architecture recommandée : native Swift / SwiftUI

**Pourquoi natif plutôt que Web embarqué (Electron/Tauri)** : la caméra de continuité, la dictée locale, le trousseau, Spotlight, les autorisations macOS et le modèle Apple local ne sont accessibles qu'en Swift. Une couche Web ajouterait Node, un pont JS↔natif et ~150 Mo, sans gain réel. Seule exception justifiée : un **service Python local** pour le calcul symbolique (SymPy), car rien d'équivalent n'existe en Swift.

```
┌────────────────────────── NovaApp (SwiftUI) ──────────────────────────┐
│ Barre latérale · Conversation · Saisie · Outils · Paramètres           │
└───────────────┬───────────────────────────────────────┬────────────────┘
                │                                       │
        ┌───────▼────────┐                     ┌────────▼────────┐
        │  NovaCore       │  routeur ─────────▶ │  NovaTools       │
        │  moteur de conv.│                     │  registre d'outils│
        │  + routage      │◀── résultats JSON ──│  (maths, météo,   │
        └───────┬────────┘                     │  fichiers, macOS…)│
                │                              └────────┬─────────┘
        ┌───────▼────────┐                     ┌────────▼─────────┐
        │  NovaModels     │                     │  NovaSystem       │
        │  Apple local    │                     │  permissions,     │
        │  Ollama (opt.)  │                     │  journal, trousseau│
        │  API distantes  │                     └────────┬─────────┘
        └────────────────┘                     ┌────────▼─────────┐
                                               │  NovaStorage      │
                                               │  SQLite + FTS5    │
                                               └──────────────────┘
        Service Python local (SymPy) ← appelé par l'outil Maths via JSON stdin/stdout
```

Chaque bloc est une cible Swift Package distincte : l'interface ne parle jamais directement aux modèles ou au système, tout passe par NovaCore.

### Moteurs d'IA

| Moteur | Statut | Rôle | Limites honnêtes |
|---|---|---|---|
| **Modèle Apple local** (Foundation Models) | prêt, vérifié | Conversation, résumés, appel d'outils, réponses structurées. Zéro téléchargement, hors ligne | Petit modèle (~3 Md de paramètres), contexte d'environ 4 000 jetons : il faudra résumer l'historique et découper les longs documents. Moins bon en raisonnement complexe et en code |
| **Ollama** (optionnel) | non installé | Modèles plus puissants (ex. Qwen, Llama, Gemma 7-8 Md ≈ 5 Go) et **vision locale** | Installation et téléchargement seulement après ta confirmation ; 16 Go de RAM limitent la taille |
| **API distantes** (Anthropic, OpenAI…) | à configurer | Raisonnement avancé, vision de haute qualité | Payant, données envoyées hors du Mac : désactivé par défaut, signalé à chaque requête, clés dans le Trousseau |

La compréhension d'images par le modèle Apple n'est pas confirmée à ce stade : je la testerai en phase 4. En attendant, une image ne sera envoyée qu'à un modèle qui la prend réellement en charge (Ollama vision ou fournisseur distant), et l'OCR local via le framework Vision fonctionnera dans tous les cas.

### Routage

Le routeur classe chaque demande avant d'appeler un modèle :

- expression ou équation simple → calculateur Swift local (instantané) ;
- calcul symbolique (dérivée, intégrale, limite, système) → service SymPy, puis vérification numérique, puis explication par le modèle ;
- PDF ou document → extraction de texte (PDFKit), découpage, résumé ;
- image → OCR local (Vision) et/ou modèle de vision compatible ;
- météo, devises → outil réseau avec source et heure affichées ;
- action macOS → outil système soumis aux autorisations ;
- reste → modèle conversationnel choisi.

Le modèle Apple sait appeler des outils (protocole `Tool`) : NOVA lui exposera le registre d'outils pour qu'il décide lui-même dans les cas ambigus.

## 3. Fonctionnalités : ce qui est faisable et comment

### Disponibles immédiatement, 100 % locales
- Conversations, historique, recherche plein texte, conversations épinglées (SQLite + FTS5).
- Modèle local Apple avec streaming.
- Calculatrice scientifique, conversions d'unités (API `Measurement` d'Apple), fractions, trigonométrie, statistiques, matrices, graphiques (Swift Charts).
- OCR de photos et documents (Vision, français pris en charge).
- Extraction de texte PDF (PDFKit).
- Dictée locale (`Speech`, reconnaissance sur l'appareil) et lecture vocale (`AVSpeechSynthesizer`).
- Notes, tâches, rappels internes à l'app, mémoire persistante séparée de l'historique.
- Ouvrir apps, fichiers, URL, copier dans le presse-papiers, infos système non sensibles (`NSWorkspace`, `NSPasteboard`).
- Recherche de fichiers dans des dossiers que tu autorises (Spotlight limité à ces dossiers).
- Lancement de Raccourcis macOS que tu as listés toi-même.

### Nécessitent une petite configuration
- **Calcul symbolique** : environnement Python isolé dans le dossier de l'app avec SymPy (~60 Mo). Remarque : ton Python est une version préliminaire 3.15 ; si SymPy y pose problème, je te le signalerai.
- **Météo** : Open-Meteo (gratuit, sans clé, données de services météo nationaux). WeatherKit d'Apple est écarté car il exige un compte développeur payant.
- **Devises** : Frankfurter (taux officiels de la BCE, gratuit, mis à jour chaque jour ouvré).
- **Recherche Web** : nécessite un fournisseur de recherche (la plupart demandent une clé) ; module optionnel en phase 4.
- **Ollama** pour des modèles plus gros et la vision locale.
- **Fournisseurs distants** : clés API à saisir par toi dans les paramètres.

### Caméra de l'iPhone : la bonne nouvelle

Apple fournit exactement le flux demandé avec la **Caméra de continuité** : une app Mac peut proposer « Prendre une photo » ou « Numériser des documents » ; macOS déclenche alors **l'appareil photo directement sur ton iPhone**, tu prends la photo, tu valides ou tu reprends sur l'iPhone, et l'image arrive dans l'app. C'est le mécanisme du menu « Importer depuis l'iPhone » du Finder, et il est accessible aux applications tierces (`NSServicesMenuRequestor`).

- Bouton caméra → menu : « Photo avec l'iPhone », « Numériser un document avec l'iPhone », « Webcam du Mac / iPhone en webcam » (`AVCaptureDevice`), « Choisir une image existante » (secours).
- Conditions : même compte Apple sur les deux appareils, Wi-Fi et Bluetooth activés, iPhone à proximité.
- Une app compagnon iOS n'est **pas nécessaire**, et ne pourrait de toute façon pas être compilée sans Xcode.
- Limite : je ne peux pas tester moi-même la prise de vue sur ton iPhone ; je validerai la partie Mac et tu confirmeras le flux complet.

### Impossibles ou déconseillés dans cet environnement
| Demande | Pourquoi | Remplacement |
|---|---|---|
| App compagnon iOS | Xcode et le SDK iOS absents | Caméra de continuité (meilleure de toute façon) |
| WeatherKit | Compte développeur Apple payant | Open-Meteo |
| Accès à ton calendrier, tes e-mails, Messages | Pas d'intégration configurée ; données sensibles | Calendrier et rappels locaux à NOVA ; EventKit plus tard, avec autorisation explicite |
| Isolation forte de l'exécution de code | Sans App Sandbox complète, macOS n'offre pas de bac à sable simple pour un processus enfant | Exécution dans un processus séparé : dossier temporaire, délai maximal, limites de ressources, confirmation avant chaque exécution, code toujours affiché avant |
| Accès libre au Terminal | Refusé par principe | Commandes préparées et affichées, exécutées seulement après ta validation |

## 4. Contraintes liées à l'absence de Xcode

Compiler sans Xcode fonctionne (**vérifié**), mais :

- l'app sera construite par `swift build` puis assemblée en `NovaAI.app` par un script (`Info.plist` avec les textes d'autorisation caméra/micro/reconnaissance vocale, icône `.icns` via `iconutil`, signature locale ad hoc). Sans ce bundle, macOS refuserait l'accès caméra et micro ;
- pas d'aperçus SwiftUI ni de catalogue d'assets : les couleurs et l'icône seront définies en code et en fichiers ;
- l'app ne sera pas notarisée : au premier lancement, macOS peut demander une confirmation (clic droit → Ouvrir). Je ne désactive pas Gatekeeper.

Installer Xcode (gratuit, ~15 Go) n'est pas nécessaire pour avancer. Ce ne serait utile que pour une future app iOS.

## 5. Sécurité et autorisations (modèle prévu)

Chaque outil déclare un **niveau de risque** :

| Niveau | Exemples | Comportement |
|---|---|---|
| Libre | calcul, conversion, recherche dans l'historique | exécuté directement, journalisé |
| Réseau | météo, devises, recherche Web, API distante | badge « quitte le Mac » visible, désactivable globalement |
| Confirmation | renommer ou créer un fichier, ouvrir une app, exécuter du code, lancer un Raccourci | fenêtre de confirmation qui montre l'action exacte |
| Fort | supprimer (corbeille uniquement), commandes système | double confirmation, jamais proposé de façon autonome |

Autres règles : accès fichiers limité aux dossiers choisis (signets sécurisés), journal d'activité consultable, contenu des documents et des pages Web traité comme des données et jamais comme des ordres, micro et caméra actifs uniquement sur action explicite.

## 6. Structure du projet prévue

```
NovaAI/
├── Package.swift
├── Sources/
│   ├── NovaApp/        interface SwiftUI (vues, thème, navigation)
│   ├── NovaCore/       conversations, routeur, protocoles
│   ├── NovaModels/     Apple local, Ollama, API distantes
│   ├── NovaTools/      registre + outils (maths, sciences, météo, fichiers…)
│   ├── NovaSystem/     autorisations, journal, trousseau, intégrations macOS
│   └── NovaStorage/    SQLite, migrations, export
├── Services/python/    moteur SymPy (JSON en entrée/sortie)
├── Resources/          Info.plist, icône
├── Tests/              tests unitaires par module
├── scripts/            build-app.sh, setup-python.sh
└── docs/               architecture, guide d'ajout d'un module
```

Seule dépendance externe prévue : aucune côté Swift (tout est natif). Côté Python : SymPy et mpmath.

## 7. Plan de développement priorisé

1. **Noyau** (phase 3) : fenêtre et thème, barre latérale réductible, conversations persistantes, streaming avec le modèle Apple, Markdown et code, copier/régénérer/modifier, pièces jointes (fichiers, images, glisser-déposer, presse-papiers), recherche dans l'historique, calculatrice et conversions, script de build `.app`.
2. **Outils** : page Outils, calcul symbolique SymPy avec étapes, graphiques, sciences, horloge mondiale, chronomètre, minuteur.
3. **Réseau** : météo Open-Meteo, devises, indicateurs « en ligne / hors ligne », dernière mise à jour.
4. **Apple** : caméra de continuité, OCR, dictée et lecture vocale, recherche Spotlight dans les dossiers autorisés, actions macOS avec confirmation.
5. **Modèles supplémentaires** : Ollama (détection, état, téléchargement sur confirmation), fournisseurs distants avec trousseau.
6. **Productivité et mémoire** : notes, tâches, calendrier local, mémoire persistante, export et suppression des données.
7. **Tests et livraison** : tests unitaires, vérification hors ligne, documentation.

## 8. Décisions qui t'appartiennent

- **Emplacement du projet** : où conserver le code de façon durable.
- **SymPy** : installer un environnement Python isolé (~60 Mo) pour le calcul symbolique.
- **Ollama** : plus tard, si tu veux des modèles plus puissants ou la vision locale (téléchargements de plusieurs Go).
