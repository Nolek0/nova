# NOVA pour iPhone

Ton IA privée, dans ta poche. **NOVA fonctionne seul sur l'iPhone**, et devient encore plus fort quand ton ordinateur est là.

- **Sans ordinateur** : Nova 1 répond instantanément (heure, date, dé, blagues…), les maths et les conversions sont faites avec les étapes, et **Apple Intelligence** répond au reste. NOVA lit aussi le texte des photos et des PDF. Tout se passe sur l'iPhone, rien ne part sur Internet.
- **Avec ton ordinateur** : tes discussions et tes modèles (Gemma…), **Nova Control**, et l'**écran de ton Mac à distance** (clics, clavier, défilement).

Il n'y a ni serveur, ni compte, ni abonnement : le téléphone parle directement à l'ordinateur. Il faut **iOS 26** ou plus récent (iPhone 11 et plus récents) ; Apple Intelligence demande un iPhone 15 Pro ou plus récent.

---

## Comment ça marche (en bref)

```
iPhone (app NOVA)  ──Wi-Fi de la maison ou Tailscale──▶  Ordinateur (NOVA ouvert)
```

- **À la maison** : le téléphone trouve l'ordinateur tout seul sur le Wi-Fi (comme une imprimante AirPrint).
- **Ailleurs** : installe **Tailscale** (gratuit pour un usage personnel) sur l'ordinateur et sur l'iPhone. Les deux appareils se voient alors comme s'ils étaient chez toi, avec une connexion chiffrée.
- **Première connexion** : NOVA affiche un **code à 6 chiffres** sur l'ordinateur et tu le tapes sur le téléphone. Ensuite, le téléphone reste connecté.

## 1. Activer le téléphone dans NOVA (sur l'ordinateur)

1. Ouvre **NOVA › Paramètres › Téléphone**.
2. Coche **« Utiliser NOVA depuis mon iPhone »**.
3. Un code à 6 chiffres s'affiche. Garde cette page ouverte.

Pour l'écran à distance et Nova Control, le Mac doit autoriser NOVA dans **Réglages Système › Confidentialité et sécurité** : **Accessibilité** et **Enregistrement de l'écran**. Ce sont les mêmes autorisations que Nova Control utilise déjà.

## 2. Installer l'app sur ton iPhone (gratuit)

Il n'y a pas besoin de l'App Store ni du compte développeur payant. Il te faut **Xcode** (gratuit, déjà installé sur ton Mac) et ton **identifiant Apple** habituel.

1. Branche l'iPhone au Mac avec un câble et touche **« Faire confiance »** sur l'iPhone.
2. Sur l'iPhone : **Réglages › Confidentialité et sécurité › Mode développeur** → active-le (l'iPhone redémarre).
3. Sur le Mac, ouvre `ios/NovaPhone.xcodeproj` (double-clic).
4. Dans Xcode : **Xcode › Réglages › Comptes** → **+** → ajoute ton identifiant Apple.
5. Clique sur **NovaPhone** (en haut à gauche) › onglet **Signing & Capabilities** › **Team** : choisis ton nom (**Personal Team**).
   - Si Xcode dit que l'identifiant `com.nolek0.nova.phone` est déjà pris, remplace-le par autre chose, par exemple `com.tonprenom.nova`.
6. En haut de la fenêtre, choisis ton iPhone, puis clique sur **▶** (Run).
7. La première fois, l'iPhone refuse d'ouvrir l'app. Va dans **Réglages › Général › VPN et gestion de l'appareil**, touche ton identifiant Apple puis **Faire confiance**.

> ⏳ **La limite des 7 jours.** Avec un compte Apple gratuit, l'app installée par Xcode s'arrête au bout de 7 jours. Il suffit de rebrancher l'iPhone et de recliquer sur ▶ (tes données ne sont pas perdues : elles sont sur l'ordinateur). Pour ne plus y penser, utilise **SideStore** (voir plus bas) : il renouvelle l'app tout seul.

## 3. Connecter le téléphone (facultatif)

NOVA marche tout de suite sur l'iPhone. Pour y ajouter ton ordinateur :

1. Dans l'app, touche **Connecter mon ordinateur** (dans Réglages ou sur l'accueil), puis accepte **« Réseau local »** : c'est ce qui permet à l'app de trouver l'ordinateur.
2. Touche le nom de ton ordinateur.
3. Tape le **code à 6 chiffres** affiché dans NOVA sur l'ordinateur.
4. Accepte les **notifications**.

Si l'ordinateur n'apparaît pas, touche **« Saisir l'adresse »** et recopie l'adresse affichée dans NOVA (par exemple `192.168.1.20`, ou l'adresse Tailscale `100.x.y.z`).

## Ce que fait l'app

| Onglet | |
|---|---|
| ✨ **NOVA** | L'accueil avec l'orbe NOVA, des suggestions et tes discussions récentes. Si le Mac est joignable, NOVA répond avec ses modèles, et la discussion est enregistrée sur le Mac. Sinon, NOVA répond sur l'iPhone, et la discussion reste sur l'iPhone. Si le Mac décroche en cours de route, la question est reprise toute seule sur l'iPhone. Tu peux joindre une photo ou un PDF. |
| 🪄 **Nova Control** | Tu décris une tâche (« ouvre Notes et crée une note Courses ») et NOVA la réalise sur le Mac. Les actions sensibles attendent ton accord sur le téléphone. |
| 💻 **Écran** | Touche **Contrôler le Mac** : l'écran du Mac s'affiche en direct. Un toucher fait un clic, deux touchers un double-clic et un appui long un clic droit. Glisser fait défiler la page, ou déplace la souris selon le bouton en haut. Pince l'écran pour zoomer. En bas, tu as le clavier, les touches ⏎ ⌫ ⇥ Échap, les flèches, ⌘C ⌘V ⌘Z ⌘⇥ ⌘Espace, et lecture/pause. |
| ⚙️ **Réglages** | L'ordinateur connecté, où NOVA réfléchit (avec l'option « Toujours répondre sur l'iPhone »), le modèle du Mac, la réflexion approfondie et les notifications. |

**Économie de batterie.** Le Mac ne filme son écran que pendant une session « Contrôler le Mac ». Pendant la session :
- le Mac affiche **« L'iPhone contrôle le Mac »** dans son encoche ;
- l'iPhone affiche **« Vous contrôlez le Mac »** dans la Dynamic Island et sur l'écran verrouillé, avec la durée.

La session s'arrête avec le bouton **Arrêter**, ou dès que tu quittes l'onglet ou l'app. Côté Mac, elle s'arrête aussi toute seule si le téléphone ne donne plus de nouvelles pendant 20 secondes.

**Notifications** : Nova Control te prévient quand il a besoin de ton accord (tu peux **Autoriser** ou **Refuser** directement depuis la notification), quand il pose une question (tu peux **Répondre** depuis la notification) et quand une tâche est finie. NOVA te prévient aussi quand une réponse est prête.

> ℹ️ Apple réserve les notifications « app fermée » (push) au compte développeur payant (99 €/an). Ici, elles arrivent quand l'app est ouverte ou que tu viens de la quitter (iOS la laisse travailler encore quelques dizaines de secondes). Quand tu la rouvres, elle se reconnecte toute seule.

L'onglet Écran et Nova Control ne sont disponibles qu'avec un Mac. Avec NOVA sur Windows, le téléphone donne accès aux discussions.

## Partager l'app avec tes proches (gratuit)

Chaque proche a **son propre NOVA** sur **son ordinateur** et connecte **son iPhone** à **son ordinateur**. Personne n'a accès à l'ordinateur d'un autre sans le code affiché sur celui-ci.

Deux façons d'installer l'app sur leur iPhone :

**A. Avec ton Mac (le plus simple)** : branche leur iPhone sur ton Mac et suis l'étape 2. L'app est signée avec ton identifiant Apple gratuit, donc elle doit être renouvelée tous les 7 jours de la même façon. Un compte gratuit est limité à quelques appareils et à 3 apps par iPhone.

**B. Avec SideStore (sans câble ensuite, renouvellement automatique)** :
1. Sur ton Mac, fabrique le fichier d'installation : `./ios/build-ipa.sh` crée `ios/build/NOVA.ipa`.
2. Envoie `NOVA.ipa` à ton proche (AirDrop, iCloud Drive…).
3. Ton proche installe **SideStore** (gratuit, open source : [sidestore.io](https://sidestore.io)) en suivant leur guide. Il se sert de **son propre identifiant Apple**.
4. Dans SideStore : **+** → choisis `NOVA.ipa`.
5. SideStore renouvelle l'app tout seul tous les 7 jours, tant que l'iPhone a internet.

**AltStore** ([altstore.io](https://altstore.io)) fonctionne de la même façon, mais il faut garder un ordinateur avec AltServer allumé de temps en temps.

## Sécurité et vie privée

- L'option Téléphone est **désactivée par défaut**.
- Aucun service extérieur : tout passe directement entre ton téléphone et ton ordinateur.
- Le code à 6 chiffres expire au bout de 10 minutes ou après 5 essais ratés, et il change après chaque connexion.
- Le téléphone garde une clé secrète dans le **trousseau iOS**. Dans NOVA › Paramètres › Téléphone, **« Déconnecter les téléphones »** rend cette clé inutilisable.
- Nova Control n'écrit jamais dans un champ de mot de passe. Les actions sensibles demandent toujours ton accord, sauf si tu choisis « Tout accepter ».
- Sur le Wi-Fi de la maison, la connexion n'est pas chiffrée : n'active pas l'option sur un Wi-Fi public. Avec Tailscale, elle est chiffrée de bout en bout.

## Pour les développeurs

- App : SwiftUI et Liquid Glass, iOS 26+, `ios/NovaPhone/` (projet `ios/NovaPhone.xcodeproj`, aucune dépendance).
  - L'extension `ios/NovaLiveActivity/` gère la Dynamic Island.
  - Le code partagé avec l'app Mac est compilé directement depuis `apple/Sources/NovaTools` (maths, conversions, lecture des PDF et photos) et `apple/Sources/NovaModels` (modèle Apple).
  - Nova 1 tourne sur l'iPhone : `ios/NovaPhone/OnDevice/Nova1.swift` est une traduction en Swift de l'inférence de `nova1/`, qui lit le même fichier `nova1/model/nova1.bin`. Elle donne les mêmes résultats que la version Rust.
- Côté ordinateur : `app/src-tauri/src/remote.rs`. C'est un petit serveur HTTP sur le port **47860**, annoncé par Bonjour sous le type `_nova._tcp`.
  - `POST /api/pair {pin, device}` → `{key}`
  - Ensuite, chaque requête envoie l'en-tête `X-Nova-Key`.
  - `POST /api/invoke {cmd, args}` : les mêmes commandes que l'interface desktop, sur une liste autorisée.
  - `POST /api/stream {cmd, args}` : une ligne JSON par événement (`send_message`, `regenerate`, `control_start`).
  - `GET /api/events` : accords, questions et fins de tâche, pour les notifications.
  - `POST /api/control/start` et `POST /api/control/stop` : la session « Contrôler le Mac ».
  - `POST /api/screen` : renvoie un JPEG, avec le cadre de l'écran dans l'en-tête `X-Screen`. Ne répond que pendant une session.
  - `POST /api/input` : clic, glisser, défilement, frappe, raccourci.
- Simulateur : `xcodebuild -project ios/NovaPhone.xcodeproj -scheme NovaPhone -sdk iphonesimulator build`. Le simulateur joint NOVA sur le même Mac à l'adresse `localhost`.
