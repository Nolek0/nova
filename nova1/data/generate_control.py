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
    if rng.random() < 0.35:
        return g_envoyer_mail_app(rng, t)
    who = pick(rng, PEOPLE, PEOPLE_TEST, t)
    msg = pick(rng, MESSAGES, MESSAGES_TEST, t)
    sub = rng.choice(SUBJECTS)
    r = rng.random()
    if r < 0.4:
        f = rng.choice([f"envoie un mail à {who} pour dire {msg}", f"écris un e-mail à {who} : {msg}", f"mail à {who} {msg}",
                        f"envoie un email à {who} pour lui dire que {msg}", f"envoie à {who} un mail disant {msg}"])
        return G.polite(rng, f), [("destinataire", who), ("message", msg)]
    if r < 0.7:
        f = rng.choice([f"envoie un mail à {who} avec l'objet {sub} et le message {msg}", f"écris à {who}, objet {sub} : {msg}",
                        f"envoie un mail à {who} avec objet {sub} et message {msg}", f"envoie un mail à {who} objet {sub} message {msg}",
                        f"mail à {who}, objet : {sub}, message : {msg}", f"envoie un mail à {who} avec pour objet {sub} et pour contenu {msg}",
                        f"envoie un message sur outlook à {who} avec objet {sub} et message {msg}", f"écris un mail à {who} sujet {sub} texte {msg}",
                        f"envoie un mail à {who} dont l'objet est {sub} et le message {msg}"])
        return G.polite(rng, f), [("destinataire", who), ("objet", sub), ("message", msg)]
    f = rng.choice([f"envoie un mail à {who}", f"écris un mail à {who}", f"nouveau mail pour {who}", f"prépare un email pour {who}"])
    return G.polite(rng, f), [("destinataire", who)]


MAIL_APPS = ["Outlook", "Gmail", "Mail", "Apple Mail", "Spark", "Thunderbird"]
CHAT_APPS = ["Discord", "WhatsApp", "Instagram", "insta", "Messenger", "Slack", "Teams", "Telegram", "Messages", "Signal", "iMessage"]
CHAT_APPS_TEST = ["Snapchat", "Viber"]
CHAT_PEOPLE = ["Tom", "Léa", "maman", "papa", "Lucas", "Inès", "Paul Martin", "mon frère", "Sarah", "le groupe", "Yanis", "Kevin"]
CHAT_PEOPLE_TEST = ["Noah", "Camille"]


def g_envoyer_mail_app(rng, t):
    """Mails avec la messagerie précisée (« depuis Outlook »)."""
    who = pick(rng, PEOPLE, PEOPLE_TEST, t)
    msg = pick(rng, MESSAGES, MESSAGES_TEST, t)
    app = rng.choice(MAIL_APPS)
    f = rng.choice([f"envoie un mail à {who} depuis {app} pour lui dire que {msg}", f"avec {app}, envoie un mail à {who} : {msg}",
                    f"envoie un email via {app} à {who} en disant {msg}", f"écris un mail sur {app} à {who} pour dire {msg}",
                    f"utilise {app} pour envoyer un mail à {who} disant {msg}", f"envoie un mail à {who} sur {app} avec le message {msg}"])
    return G.polite(rng, f), [("destinataire", who), ("app", app), ("message", msg)]


FIRST = ["Tom", "Léa", "Lucas", "Inès", "Sarah", "Yanis", "Kevin", "Noah", "Camille", "Mamadou", "Ibrahim", "Fatou", "Chloé", "Hugo", "Amine",
         "Rayan", "Jade", "Louis", "Emma", "Nathan", "Lina", "Adam", "Mehdi", "Aya", "Théo", "Manon", "Enzo", "Zoé", "Karim", "Sofia", "Alex",
         "First", "Jean", "Marie", "Paul", "Anaïs", "Bilal", "Yasmine", "Moussa", "Clara", "Maxime", "Océane", "Samir", "Elif", "Diego", "Jules"]
WORDS = ["Pi", "Dark", "Kabylie", "Gamer", "Nova", "Shadow", "Wolf", "King", "Queen", "Pixel", "Ninja", "Frost", "Blaze", "Moon", "Star", "Lion",
         "Tiger", "Zen", "Neo", "Max", "Fox", "Bear", "Storm", "Ghost", "Kira", "Atlas", "Echo", "Rio", "Volt", "Sky"]
RELATIVES = ["maman", "papa", "mon frère", "ma sœur", "mon cousin", "ma cousine", "mon prof", "le groupe", "mon pote", "ma meuf", "mon reuf",
             "mamie", "papi", "mon coach", "la team", "le serveur", "mon chef", "ma voisine"]


def fake_who(rng):
    """Pseudos et noms variés : « First Pi », « JSKabylie1946 », « xX_Dark_Xx », « @lea.m », « Paul Martin »…"""
    r = rng.random()
    if r < 0.18:
        return rng.choice(FIRST)
    if r < 0.3:
        return rng.choice(RELATIVES)
    if r < 0.45:
        return f"{rng.choice(FIRST)} {rng.choice(WORDS + [w.upper() for w in WORDS[:5]] + ['Martin', 'Diallo', 'Benali', 'Dupont', 'Nguyen'])}"
    if r < 0.6:
        return "".join(rng.choice([str.upper, str.title, str.lower])(x) for x in [rng.choice(FIRST + WORDS)[:rng.randint(2, 6)], rng.choice(WORDS)]) + str(rng.choice([1, 7, 13, 22, 69, 99, 2008, 1946, 2012, 404, ""]))
    if r < 0.72:
        a, b = rng.choice(WORDS), rng.choice(WORDS)
        return rng.choice([f"xX_{a}_Xx", f"{a}_{b}", f"{a.lower()}.{b.lower()}", f"{a}{b}", f"the{a}", f"{a.lower()}_{rng.randint(1, 99)}"])
    if r < 0.85:
        return "@" + rng.choice([f"{rng.choice(FIRST).lower()}.{rng.choice('abcdefghijklm')}", f"{rng.choice(FIRST).lower()}_{rng.randint(1, 99)}", rng.choice(WORDS).lower() + rng.choice(WORDS).lower()])
    return rng.choice(FIRST).lower()


GREET = ["salut", "yo", "coucou", "bonjour", "hey", "wesh", "bonsoir", "hello", "slt", "cc", "bjr"]
MSG_PARTS = ["on se voit à {h}", "tu viens ce soir ?", "je suis en retard", "j'arrive dans {n} minutes", "t'es dispo {d} ?", "on joue ce soir ?",
             "merci pour {x}", "bien reçu", "je t'appelle {d}", "le match commence à {h}", "rdv devant {p}", "tu peux m'envoyer {x} ?",
             "c'est validé", "je ne pourrai pas venir {d}", "bon anniversaire", "joyeux anniversaire {who}", "bien joué", "on décale à {d}",
             "n'oublie pas {x}", "je t'envoie {x} ce soir", "t'as vu le message ?", "réponds-moi stp", "ça marche", "bonne nuit", "à {d}",
             "la réunion est à {h}", "le rendu est fini", "check tes mails", "appelle-moi quand tu peux", "je suis chez moi", "c'est noté"]
FILL = {"h": ["18h", "20h30", "midi", "14h", "9h"], "n": ["5", "10", "15", "20"], "d": ["demain", "ce soir", "samedi", "lundi", "tout à l'heure", "la semaine prochaine"],
        "x": ["hier", "les photos", "le fichier", "les notes", "le lien", "ton aide", "le cadeau", "les clés"], "p": ["la gare", "le ciné", "chez Tom", "l'école"],
        "who": ["frérot", "ma belle", "champion", "boss"]}


ASK_PARTS = ["si il a de l'argent à me prêter", "s'il a de l'argent à me prêter", "s'il vient {d}", "si elle vient {d}", "s'il est dispo {d}",
             "comment il va", "comment elle va", "à quelle heure il arrive", "à quelle heure on mange", "où il est", "s'il peut m'appeler",
             "de m'appeler {d}", "s'il a vu {x}", "si elle a reçu {x}", "ce qu'il fait {d}", "quand il rentre", "s'il veut venir à {p}",
             "s'il a fini {x}", "s'il peut me ramener {x}", "pourquoi il répond pas", "combien ça coûte", "s'il joue {d}"]


def fake_msg(rng, short=False):
    if short or rng.random() < 0.3:
        g = rng.choice(GREET)
        return g if rng.random() < 0.6 else f"{g} {rng.choice(['ça va ?', 'tu fais quoi ?', 'bien ou quoi', 'mon pote', '!'])}"
    m = rng.choice(MSG_PARTS)
    for k, v in FILL.items():
        m = m.replace("{" + k + "}", rng.choice(v))
    if rng.random() < 0.3:
        m = f"{rng.choice(GREET)} {m}"
    return m


def g_envoyer_message(rng, t):
    if rng.random() < 0.08:
        return g_meme_message(rng, t)
    who = fake_who(rng) if not t else rng.choice(["Noah", "Camille", "Zéphyr 9", "@nina.k", "BigBoss77"])
    app = pick(rng, CHAT_APPS, CHAT_APPS_TEST, t) if rng.random() < 0.7 else None
    msg = fake_msg(rng)
    short = fake_msg(rng, short=True)
    sur = f" sur {app}" if app else ""
    slots_app = [("app", app)] if app else []
    forms = [
        (f"envoie un message à {who}{sur} : {msg}", msg), (f"envoie un message à {who}{sur} pour lui dire que {msg}", msg),
        (f"envoie un message à {who}{sur} en disant {msg}", msg), (f"envoie un message à {who}{sur} qui dit {msg}", msg),
        (f"envoie {short} à {who}{sur}", short), (f"envoie « {short} » à {who}{sur}", short), (f"dis {short} à {who}{sur}", short),
        (f"dis à {who}{sur} que {msg}", msg), (f"écris à {who}{sur} : {msg}", msg), (f"écris à {who}{sur} {msg}", msg),
        (f"envoie un message à {who}{sur} : {short}", short), (f"envoie un dm à {who}{sur} : {msg}", msg), (f"réponds {short} à {who}{sur}", short),
        (f"préviens {who}{sur} que {msg}", msg), (f"envoie à {who}{sur} le message {msg}", msg), (f"envoie un message a {who}{sur} : {short}", short),
        (f"envoi un msg à {who}{sur} : {msg}", msg), (f"send {short} to {who}{' on ' + app if app else ''}", short),
        (f"text {who}{' on ' + app if app else ''} : {msg}", msg),
    ]
    # Consignes indirectes (« demande-lui s'il… », « dis-lui que… ») : tout le contenu est le message ;
    # NOVA le réécrit ensuite au discours direct (app/src-tauri/src/tools/speech.rs).
    q = rng.choice(ASK_PARTS)
    for k, v in FILL.items():
        q = q.replace("{" + k + "}", rng.choice(v))
    g = rng.choice(GREET + ["salut ça va", "yo bien ou quoi", "coucou ça va", "wesh", "hey ça va"])
    lui = rng.choice(["demande lui", "demande-lui", "demandes lui"])
    dis = rng.choice(["dis lui", "dis-lui"])
    st = rng.choice(["stp", "svp", "s'il te plaît", ""])
    ask = f"{g} et {lui} {q} {st}".strip()
    tell = f"{g} et {dis} que {rng.choice(MSG_PARTS).replace('{d}', rng.choice(FILL['d'])).replace('{x}', rng.choice(FILL['x'])).replace('{h}', rng.choice(FILL['h'])).replace('{n}', rng.choice(FILL['n'])).replace('{p}', rng.choice(FILL['p'])).replace('{who}', rng.choice(FILL['who']))}"
    verb = rng.choice(["envoyer", "envoie", "envoi", "écris", "ecris", "écrire"])
    forms += [
        (f"{verb} à {who}{sur} {ask}", ask), (f"{verb} à {who}{sur} {tell}", tell), (f"{verb} un message à {who}{sur} {ask}", ask),
        (f"{verb} à {who}{sur} {msg}", msg), (f"{verb} à {who}{sur} {short}", short),
        (f"{lui} à {who}{sur} {q}".replace("demande lui à", "demande à").replace("demande-lui à", "demande à").replace("demandes lui à", "demande à"), q),
        (f"dis à {who}{sur} {short} et {lui} {q}", f"{short} et {lui} {q}"),
    ]
    text, m = rng.choice(forms)
    if rng.random() < 0.08:
        return G.polite(rng, f"envoie un message à {who}{sur}"), [("destinataire", who)] + slots_app
    return G.polite(rng, text), [("destinataire", who)] + slots_app + [("message", m)]


def g_meme_message(rng, t):
    """« et le même sur Discord à X » : on reprend le message précédent."""
    who = fake_who(rng)
    app = rng.choice(CHAT_APPS)
    f = rng.choice([f"le même sur {app} à {who}", f"pareil sur {app} pour {who}", f"envoie la même chose à {who} sur {app}",
                    f"fais pareil sur {app} avec {who}", f"idem sur {app} à {who}", f"la même chose sur {app} pour {who}",
                    f"envoie le même message à {who} sur {app}", f"refais pareil à {who} sur {app}"])
    return G.polite(rng, f), [("destinataire", who), ("app", app)]


def typo(rng, word):
    """Faute de frappe sur un nom d'app (« outlok », « discrod », « chrom »)."""
    if len(word) < 4:
        return word
    i = rng.randrange(1, len(word) - 1)
    op = rng.random()
    if op < 0.4:
        return word[:i] + word[i + 1:]
    if op < 0.8:
        return word[:i] + word[i + 1] + word[i] + word[i + 2:]
    return word[:i] + word[i] + word[i:]


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
    "ecrire": g_ecrire, "creer_dossier": g_creer_dossier, "envoyer_message": g_envoyer_message, "volume": g_volume, "rechercher_page": g_rechercher_page, "defiler": g_defiler,
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

COUNTS = {"enregistrer": 2600, "envoyer_mail": 2600, "envoyer_message": 6000, "ouvrir_app": 1800, "menu": 1500, "cliquer": 1500, "ecrire": 1200, "nouvelle_note": 1200,
          "autre": 1600, "recherche_web": 900, "ouvrir_site": 900, "creer_dossier": 900, "fermer_app": 700, "volume": 600}
DEFAULT = 450


def build(seed, test):
    rng = random.Random(seed)
    rows = []
    for intent, gen in GENERATORS.items():
        n = COUNTS.get(intent, DEFAULT)
        if test:
            n = min(n, 400)
        if test:
            n = max(60, n // 6)
        for _ in range(n):
            text, slots = gen(rng, test)
            # Fautes sur les noms d'applications : le modèle doit les repérer quand même.
            if not test and rng.random() < 0.12:
                for k, v in list(slots):
                    if k == "app" and v in text:
                        w = typo(rng, v)
                        text = text.replace(v, w, 1)
                        slots = [(kk, w if (kk == k and vv == v) else vv) for kk, vv in slots]
            text, slots = G.noisy(text, slots, rng)
            rows.append((intent, text, slots))
    rng.shuffle(rows)
    return rows


if __name__ == "__main__":
    # Facteur de taille : python3 generate_control.py 12  → environ 12 fois plus de phrases.
    SCALE = int(sys.argv[1]) if len(sys.argv) > 1 else 1
    COUNTS = {k: v * SCALE for k, v in COUNTS.items()}
    DEFAULT = DEFAULT * SCALE
    train, test = build(77, False), build(78, True)
    G.write(os.path.join(HERE, "control_train.tsv"), train)
    G.write(os.path.join(HERE, "control_test.tsv"), test)
    print(f"train : {len(train)} · test : {len(test)} · {len(GENERATORS)} intentions")
