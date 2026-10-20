#!/usr/bin/env python3
"""Données d'entraînement du modèle Nova Control (compréhension des tâches sur l'ordinateur).

Même principe que generate.py : des modèles de phrases écrits à la main, aucune autre IA.
Chaque phrase est une commande simple ; les tâches en plusieurs étapes (« ouvre Photoshop puis
enregistre l'image en PNG ») sont découpées par NOVA avant d'être analysées.

Sortie : control_train.tsv, control_test.tsv (intention <TAB> phrase <TAB> info=valeur;…)
"""
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import generate as G  # noqa: E402  (vocabulaire, politesse et bruit partagés)

APPS = G.APPS_TRAIN + ["Photoshop", "Lightroom", "Mail", "Notes", "Finder", "Safari", "Musique", "Spotify", "Word", "Pages", "Aperçu"]
APPS_TEST = G.APPS_TEST
SITES, SITES_TEST = G.SITES_TRAIN, G.SITES_TEST

FORMATS = ["PNG", "JPEG", "JPG", "PDF", "TIFF", "GIF", "HEIC", "WebP", "MP4", "TXT", "DOCX", "PSD", "SVG", "CSV"]
FOLDERS = ["Bureau", "Documents", "Téléchargements", "Images", "Desktop", "Musique", "Vidéos", "mon bureau", "le bureau", "mes documents"]
FOLDERS_TEST = ["Downloads", "Pictures", "mes images"]
NAMES = ["vacances", "rapport final", "photo1", "devis client", "logo", "affiche concert", "cv 2026", "facture mars", "plan", "essai",
         "projet nova", "capture", "dessin", "image finale", "export web", "notes de cours", "budget", "miniature youtube"]
NAMES_TEST = ["bannière", "brouillon", "version 2"]
MENUS = [["Fichier", "Exporter"], ["Fichier", "Enregistrer sous"], ["Édition", "Copier"], ["Image", "Taille de l'image"], ["Présentation", "Zoom avant"],
         ["Fichier", "Nouveau"], ["Format", "Police"], ["Fenêtre", "Réduire"], ["Fichier", "Imprimer"], ["Calque", "Nouveau", "Calque"],
         ["File", "Export"], ["Edit", "Paste"], ["Affichage", "Afficher la barre latérale"], ["Outils", "Préférences"], ["Aide", "Recherche"]]
MENUS_TEST = [["Filtre", "Flou"], ["Insertion", "Tableau"], ["View", "Zoom"]]
BUTTONS = ["Envoyer", "OK", "Annuler", "Enregistrer", "Nouveau message", "Suivant", "Valider", "Ouvrir", "Fermer", "Continuer", "Rechercher",
           "Télécharger", "Partager", "Ajouter", "Modifier", "Exporter", "Terminé", "Oui", "Non", "Créer", "Importer", "Lecture", "Paramètres"]
BUTTONS_TEST = ["Appliquer", "Confirmer", "Démarrer"]
TEXTS = ["bonjour à tous", "Courses", "réunion demain à 10h", "mon numéro de commande est 4521", "Cher Monsieur", "merci pour ton aide",
         "titre de la vidéo", "lait, pain, œufs", "hello world", "rendez-vous chez le dentiste", "Nova AI", "à bientôt", "liste des tâches",
         "le projet avance bien", "test 123"]
TEXTS_TEST = ["joyeux anniversaire", "compte rendu", "à demain"]
PEOPLE = ["paul@exemple.fr", "marie.dupont@gmail.com", "contact@nova.app", "prof@ecole.be", "lea@outlook.fr", "Paul", "maman", "Marie", "mon prof", "Léa"]
PEOPLE_TEST = ["tom@test.org", "Lucas", "jean.martin@free.fr"]
SUBJECTS = ["Retard", "Réunion", "Devis", "Question", "Photos", "Rendez-vous", "Projet", "Facture"]
MESSAGES = ["je serai en retard demain", "voici les photos", "merci pour hier", "on se voit à 18h", "le devis est prêt", "je ne pourrai pas venir",
            "peux-tu me rappeler", "bonne nouvelle, c'est validé", "je t'envoie le fichier ce soir"]
MESSAGES_TEST = ["le rendu est terminé", "j'arrive dans 10 minutes"]
QUERIES = G.WEB_QUERIES + ["meilleur ordinateur 2026", "horaires train Bruxelles Paris", "recette pâtes carbonara", "tuto Photoshop détourage"]


def pick(rng, train, test, is_test):
    return rng.choice(test if (is_test and test) else train)


def maybe_app(rng, is_test, text, slots, p=0.25):
    """Ajoute parfois « dans Photoshop » pour préciser l'application visée."""
    if rng.random() < p:
        app = pick(rng, APPS, APPS_TEST, is_test)
        if rng.random() < 0.5:
            return f"dans {app}, {text}", slots + [("app", app)]
        return f"{text} dans {app}", slots + [("app", app)]
    return text, slots


def g_ouvrir_app(rng, t):
    app = pick(rng, APPS, APPS_TEST, t) if rng.random() < 0.7 else G.fake_app(rng)
    f = rng.choice(["ouvre {a}", "lance {a}", "démarre {a}", "va sur {a}", "ouvre l'application {a}", "mets {a}", "passe sur {a}",
                    "affiche {a}", "bascule sur {a}", "j'ai besoin de {a}", "ouvre-moi {a}", "va dans {a}"])
    return G.polite(rng, f.format(a=app)), [("app", app)]


def g_fermer_app(rng, t):
    app = pick(rng, APPS, APPS_TEST, t)
    f = rng.choice(["ferme {a}", "quitte {a}", "fermer {a}", "arrête {a}", "coupe {a}", "ferme l'application {a}"])
    return G.polite(rng, f.format(a=app)), [("app", app)]


def g_ouvrir_site(rng, t):
    s = pick(rng, SITES, SITES_TEST, t)
    f = rng.choice(["ouvre {s}", "va sur {s}", "ouvre le site {s}", "affiche la page {s}", "emmène-moi sur {s}", "navigue vers {s}"])
    return G.polite(rng, f.format(s=s)), [("site", s)]


def g_recherche_web(rng, t):
    q = rng.choice(QUERIES)
    f = rng.choice(["cherche {q} sur internet", "recherche {q} sur google", "google {q}", "fais une recherche sur {q}", "cherche {q} sur le web",
                    "tape {q} dans google", "lance une recherche : {q}"])
    return G.polite(rng, f.format(q=q)), [("requete", q)]


def g_nouvelle_note(rng, t):
    title = rng.choice(["Courses", "Idées", "Réunion", "À faire", "Projet", "Film à voir"])
    content = pick(rng, TEXTS, TEXTS_TEST, t)
    r = rng.random()
    if r < 0.4:
        return G.polite(rng, rng.choice([f"crée une note {title} avec {content}", f"fais une note « {title} » avec : {content}", f"nouvelle note {title} : {content}"])), [("titre", title), ("contenu", content)]
    if r < 0.7:
        return G.polite(rng, rng.choice([f"crée une note {content}", f"note {content}", f"écris une note : {content}", f"ajoute une note {content}"])), [("contenu", content)]
    return G.polite(rng, rng.choice([f"crée une note {title}", f"nouvelle note {title}", f"ouvre une nouvelle note {title}"])), [("titre", title)]


def g_envoyer_mail(rng, t):
    who = pick(rng, PEOPLE, PEOPLE_TEST, t)
    msg = pick(rng, MESSAGES, MESSAGES_TEST, t)
    sub = rng.choice(SUBJECTS)
    r = rng.random()
    if r < 0.4:
        f = rng.choice([f"envoie un mail à {who} pour dire {msg}", f"écris un e-mail à {who} : {msg}", f"mail à {who} {msg}",
                        f"envoie un email à {who} pour lui dire que {msg}", f"envoie à {who} un mail disant {msg}"])
        return G.polite(rng, f), [("destinataire", who), ("message", msg)]
    if r < 0.7:
        f = rng.choice([f"envoie un mail à {who} avec l'objet {sub} et le message {msg}", f"écris à {who}, objet {sub} : {msg}"])
        return G.polite(rng, f), [("destinataire", who), ("objet", sub), ("message", msg)]
    f = rng.choice([f"envoie un mail à {who}", f"écris un mail à {who}", f"nouveau mail pour {who}", f"prépare un email pour {who}"])
    return G.polite(rng, f), [("destinataire", who)]


def g_enregistrer(rng, t):
    fmt = rng.choice(FORMATS)
    folder = pick(rng, FOLDERS, FOLDERS_TEST, t)
    name = pick(rng, NAMES, NAMES_TEST, t)
    what = rng.choice(["l'image", "le fichier", "le document", "ça", "le projet", "la photo", "la vidéo", "le dessin", ""])
    verb = rng.choice(["enregistre", "sauvegarde", "exporte", "enregistrer", "sauve", "save"])
    parts = [verb] + ([what] if what else [])
    slots = []
    order = rng.sample(["fmt", "folder", "name"], k=rng.randint(0, 3))
    for o in order:
        if o == "fmt":
            parts.append(rng.choice([f"en {fmt}", f"au format {fmt}", f"en format {fmt}"]))
            slots.append(("format", fmt))
        elif o == "folder":
            parts.append(rng.choice([f"sur le {folder}" if folder in ("Bureau", "Desktop") else f"dans {folder}", f"dans {folder}"]))
            slots.append(("dossier", folder))
        else:
            parts.append(rng.choice([f"sous le nom {name}", f"sous {name}", f"avec le nom {name}", f"nommé {name}"]))
            slots.append(("nom", name))
    text, slots = maybe_app(rng, t, " ".join(parts), slots)
    return G.polite(rng, text), slots


def g_menu(rng, t):
    path = pick(rng, MENUS, MENUS_TEST, t)
    sep = rng.choice([" puis ", " > ", " › ", " / ", ", "])
    chain = sep.join(path)
    f = rng.choice(["clique sur le menu {c}", "va dans le menu {c}", "menu {c}", "ouvre le menu {c}", "choisis {c} dans les menus", "va dans {c}",
                    "sélectionne {c} dans le menu"])
    return G.polite(rng, f.format(c=chain)), [("chemin", chain)]


def g_cliquer(rng, t):
    b = pick(rng, BUTTONS, BUTTONS_TEST, t)
    f = rng.choice(["clique sur {b}", "appuie sur {b}", "clique sur le bouton {b}", "appuie sur le bouton « {b} »", "clic sur {b}", "presse {b}",
                    "sélectionne {b}", "coche {b}", "clique {b}", "touche {b}"])
    text, slots = maybe_app(rng, t, f.format(b=b), [("element", b)], 0.15)
    return G.polite(rng, text), slots


def g_ecrire(rng, t):
    x = pick(rng, TEXTS, TEXTS_TEST, t)
    f = rng.choice(["écris {x}", "tape {x}", "saisis {x}", "écris « {x} »", "tape le texte {x}", "rédige {x}", "entre {x}", "marque {x}", "écrit {x}"])
    return G.polite(rng, f.format(x=x)), [("texte", x)]


def g_creer_dossier(rng, t):
    name = pick(rng, NAMES, NAMES_TEST, t)
    folder = pick(rng, FOLDERS, FOLDERS_TEST, t)
    if rng.random() < 0.5:
        f = rng.choice([f"crée un dossier {name} sur le Bureau", f"crée un dossier {name} dans {folder}", f"nouveau dossier {name} dans {folder}"])
        return G.polite(rng, f), [("nom", name), ("dossier", "Bureau" if "sur le Bureau" in f else folder)]
    return G.polite(rng, rng.choice([f"crée un dossier {name}", f"nouveau dossier {name}", f"fais un dossier appelé {name}"])), [("nom", name)]


def g_volume(rng, t):
    lvl = str(rng.choice([0, 10, 20, 25, 30, 40, 50, 60, 70, 75, 80, 90, 100]))
    f = rng.choice([f"mets le volume à {lvl}", f"volume {lvl}", f"règle le son à {lvl} %", f"baisse le volume à {lvl}", f"monte le son à {lvl}"])
    return G.polite(rng, f), [("niveau", lvl)]


def g_rechercher_page(rng, t):
    x = rng.choice(["prix", "contact", "Nova", "horaires", "conclusion", "chapitre 2", "email"])
    f = rng.choice([f"cherche {x} dans la page", f"trouve {x} sur la page", f"recherche le mot {x} dans le document", f"cmd f {x}"])
    return G.polite(rng, f), [("texte", x)]


def g_defiler(rng, t):
    d = rng.choice(["bas", "haut"])
    f = rng.choice([f"descends", f"fais défiler vers le {d}", f"scroll vers le {d}", f"remonte", f"va en {d} de la page", f"défile vers le {d}"])
    if f in ("descends",):
        d = "bas"
    if f in ("remonte",):
        d = "haut"
    return G.polite(rng, f), [("direction", d)]


SIMPLE = {
    "copier": ["copie", "copie ça", "copie le texte", "copier", "fais cmd c", "copie la sélection"],
    "coller": ["colle", "colle ça", "coller", "colle le texte", "fais cmd v", "colle ici"],
    "couper": ["coupe", "coupe la sélection", "couper le texte", "cmd x"],
    "annuler": ["annule", "annule la dernière action", "fais cmd z", "reviens en arrière", "undo", "annuler"],
    "retablir": ["rétablis", "refais", "redo", "rétablir la dernière action"],
    "tout_selectionner": ["sélectionne tout", "tout sélectionner", "cmd a", "sélectionne tout le texte"],
    "nouvel_onglet": ["ouvre un nouvel onglet", "nouvel onglet", "crée un onglet", "ajoute un onglet"],
    "fermer_onglet": ["ferme l'onglet", "ferme cet onglet", "fermer l'onglet actuel", "ferme la page"],
    "nouveau_document": ["crée un nouveau document", "nouveau fichier", "nouveau document", "fais un nouveau projet", "ouvre un nouveau fichier"],
    "capture_ecran": ["fais une capture d'écran", "capture l'écran", "screenshot", "prends une capture d'écran", "fais un screen"],
    "plein_ecran": ["mets en plein écran", "plein écran", "passe en plein écran", "agrandis la fenêtre en plein écran"],
    "muet": ["coupe le son", "mets en muet", "mute", "silence"],
    "lecture_pause": ["mets pause", "pause la musique", "relance la musique", "lecture", "play", "mets la musique en pause", "reprends la lecture"],
    "piste_suivante": ["musique suivante", "passe à la chanson suivante", "next", "piste suivante", "change de musique"],
}

GENERATORS = {
    "ouvrir_app": g_ouvrir_app, "fermer_app": g_fermer_app, "ouvrir_site": g_ouvrir_site, "recherche_web": g_recherche_web,
    "nouvelle_note": g_nouvelle_note, "envoyer_mail": g_envoyer_mail, "enregistrer": g_enregistrer, "menu": g_menu, "cliquer": g_cliquer,
    "ecrire": g_ecrire, "creer_dossier": g_creer_dossier, "volume": g_volume, "rechercher_page": g_rechercher_page, "defiler": g_defiler,
}
for k, forms in SIMPLE.items():
    def mk(forms):
        def gen(rng, t):
            text, slots = maybe_app(rng, t, rng.choice(forms), [], 0.15)
            return G.polite(rng, text), slots
        return gen
    GENERATORS[k] = mk(forms)

AUTRE = ["qui est le président de la France", "explique-moi la photosynthèse", "quelle heure est-il", "raconte une blague", "c'est quoi un PNG",
         "comment enregistrer une image en PNG", "comment ouvrir Photoshop", "pourquoi mon Mac est lent", "écris-moi un poème sur la mer",
         "résume ce texte", "traduis bonjour en anglais", "calcule 12 fois 7", "quel temps fait-il", "aide-moi à réviser", "tu peux m'expliquer les menus",
         "je n'aime pas Photoshop", "Photoshop est cher ?", "le mail c'est quoi", "merci", "salut", "bonne nuit"]
GENERATORS["autre"] = lambda rng, t: (rng.choice(AUTRE), [])

COUNTS = {"enregistrer": 2600, "envoyer_mail": 1800, "ouvrir_app": 1800, "menu": 1500, "cliquer": 1500, "ecrire": 1200, "nouvelle_note": 1200,
          "autre": 1600, "recherche_web": 900, "ouvrir_site": 900, "creer_dossier": 900, "fermer_app": 700, "volume": 600}
DEFAULT = 450


def build(seed, test):
    rng = random.Random(seed)
    rows = []
    for intent, gen in GENERATORS.items():
        n = COUNTS.get(intent, DEFAULT)
        if test:
            n = max(60, n // 6)
        for _ in range(n):
            text, slots = gen(rng, test)
            text, slots = G.noisy(text, slots, rng)
            rows.append((intent, text, slots))
    rng.shuffle(rows)
    return rows


if __name__ == "__main__":
    train, test = build(77, False), build(78, True)
    G.write(os.path.join(HERE, "control_train.tsv"), train)
    G.write(os.path.join(HERE, "control_test.tsv"), test)
    print(f"train : {len(train)} · test : {len(test)} · {len(GENERATORS)} intentions")
