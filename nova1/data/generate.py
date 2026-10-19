#!/usr/bin/env python3
"""Générateur des données d'entraînement de Nova 1.

Toutes les phrases viennent de modèles de phrases écrits à la main ici : aucune autre IA
n'est utilisée. Le jeu de test utilise des applications, villes, sites et formulations
réservés (jamais vus à l'entraînement) pour mesurer la vraie capacité de généralisation.

Sortie (TSV) : intention <TAB> phrase <TAB> info=valeur;info=valeur
Usage : python3 generate.py   (écrit train.tsv et test.tsv à côté de ce fichier)
"""
import os
import random

HERE = os.path.dirname(os.path.abspath(__file__))

# ----------------------------------------------------------------------------- Vocabulaire

APPS_TRAIN = ["Spotify", "Safari", "Google Chrome", "Chrome", "Word", "Excel", "PowerPoint", "Notes", "Calendrier", "Messages", "Mail",
              "FaceTime", "Photos", "Musique", "Discord", "Notion", "Visual Studio Code", "VS Code", "Terminal", "Finder", "Calculatrice",
              "Plans", "Rappels", "Slack", "Zoom", "Teams", "Steam", "VLC", "Photoshop", "Premiere Pro", "FL Studio", "Minecraft", "WhatsApp",
              "Telegram", "Firefox", "Brave", "Obsidian", "Figma", "Blender", "Xcode", "Pages", "Keynote", "Numbers", "Aperçu", "TextEdit",
              "Réglages Système", "App Store", "Podcasts", "Horloge", "Bloc-notes", "Paint", "Explorateur de fichiers", "Edge", "Outlook",
              "Goodnotes", "Lightroom", "Illustrator", "Unity", "Android Studio", "Logic Pro", "GarageBand", "iMovie", "Raccourcis", "Contacts"]
APPS_TEST = ["Audacity", "GIMP", "Arc", "Signal", "Todoist", "Canva", "Postman", "Thunderbird", "OBS", "Krita", "Ableton Live", "Opera"]

SITES_TRAIN = ["youtube.com", "google.fr", "wikipedia.org", "lemonde.fr", "github.com", "amazon.fr", "netflix.com", "reddit.com",
               "leboncoin.fr", "openstreetmap.org", "x.com", "instagram.com", "twitch.tv", "deepl.com", "doctolib.fr", "laposte.fr",
               "youtube", "netflix", "google", "wikipedia", "github", "amazon", "reddit", "instagram", "twitch", "gmail"]
SITES_TEST = ["lequipe.fr", "allocine.fr", "spotify.com", "linkedin", "pinterest.com", "duckduckgo.com", "bbc.com", "tiktok"]

CITIES_TRAIN = ["Paris", "Lyon", "Marseille", "Lille", "Bordeaux", "Toulouse", "Nantes", "Nice", "Strasbourg", "Rennes", "Montpellier",
                "Bruxelles", "Liège", "Namur", "Mons", "Charleroi", "Gand", "Anvers", "Genève", "Lausanne", "Montréal", "Québec",
                "Londres", "Madrid", "Barcelone", "Rome", "Berlin", "New York", "Tokyo", "Dakar", "Casablanca", "Alger", "Tunis",
                "Abidjan", "Grenoble", "Dijon", "Reims", "Le Havre", "Saint-Étienne", "Clermont-Ferrand", "Brest", "Tours", "Metz",
                "Louvain-la-Neuve", "Ottignies", "Wavre", "Arlon", "Tournai", "Luxembourg", "Amsterdam", "Lisbonne", "Dubaï"]
CITIES_TEST = ["Nancy", "Ajaccio", "Bruges", "Ostende", "Bâle", "Zurich", "Sydney", "Le Caire", "Kinshasa", "Perpignan", "Annecy", "Vienne"]

NOTES = ["acheter du pain", "appeler maman demain", "réunion lundi à 10h", "idée : une appli de musique", "rendre le livre à la bibliothèque",
         "payer la facture d'électricité", "réviser le chapitre 3 de maths", "anniversaire de Léa samedi", "prendre rendez-vous chez le dentiste",
         "acheter des piles", "envoyer le devoir au prof", "ne pas oublier les clés", "liste de courses : lait, oeufs, farine",
         "tester le nouveau plugin", "finir la prod pour vendredi", "rappeler le garage", "regarder le film conseillé par Tom",
         "changer le mot de passe du wifi", "faire le plein", "sortir les poubelles mardi soir"]
NOTES_TEST = ["arroser les plantes", "commander des cartouches d'encre", "préparer l'exposé d'histoire", "réserver le train pour Lille"]

FILE_QUERIES = ["facture edf", "cv", "mon cv", "photos de vacances", "rapport de stage", "devoir de maths", "contrat de location",
                "relevé bancaire", "playlist", "projet fl studio", "exercice 2", "notes de cours", "ticket de caisse", "carte d'identité scannée",
                "présentation", "mémoire", "budget 2026", "devis", "recette crêpes", "partition piano"]
FILE_QUERIES_TEST = ["attestation de stage", "bulletin de notes", "plan de l'appartement", "mixage final"]

WEB_QUERIES = ["la tour Eiffel", "les trous noirs", "Napoléon", "la photosynthèse", "le prix du bitcoin", "les résultats du match",
               "la recette du tiramisu", "l'histoire de la Belgique", "les volcans", "Playboi Carti", "le fonctionnement d'un moteur",
               "la Révolution française", "l'intelligence artificielle", "les dinosaures", "Marie Curie"]
WEB_QUERIES_TEST = ["les pyramides d'Égypte", "Albert Einstein", "les éoliennes", "le Tour de France"]

TOPICS = ["la photosynthèse", "la Révolution française", "les trous noirs", "le théorème de Pythagore", "la relativité", "l'inflation",
          "le réchauffement climatique", "les impôts", "la mitose", "le droit économique", "la démocratie", "les fractions", "le blues",
          "les accords de guitare", "le mixage audio", "la programmation orientée objet", "les pointeurs en C", "la Seconde Guerre mondiale",
          "la tectonique des plaques", "le système immunitaire", "les intérêts composés", "la poésie de Baudelaire", "le rap français",
          "la mécanique quantique", "le cycle de l'eau", "les verbes irréguliers anglais", "la dérivée", "les nombres premiers",
          "la blockchain", "les réseaux de neurones", "la gravitation", "la Belgique", "le marketing", "la philosophie de Kant"]


# ----------------------------------------------------------------------------- Vocabulaire combinatoire
# Des valeurs composées au hasard obligent Nova 1 à repérer les informations grâce au
# contexte de la phrase (« ouvre ___ », « note : ___ ») plutôt qu'en apprenant des listes.

SYLL = ["ka", "lo", "mi", "zen", "tor", "vi", "ra", "nex", "bel", "qua", "fly", "dro", "sto", "pix", "lu", "mor", "an", "tel", "ri", "ver",
        "go", "su", "pra", "ny", "ox", "el", "cor", "da", "fen", "ix", "ul", "bra", "sky", "tri", "mo", "ze", "lia", "ron", "pa", "flo"]
APP_SUFFIX = ["", "", "", " Studio", " Pro", " Player", " Notes", " Chat", " Mail", " Desk", " Music", " Draw", " Editor", " Lite", " 2"]


def fake_name(rng, parts=(2, 3)):
    n = rng.randint(*parts)
    return "".join(rng.choice(SYLL) for _ in range(n)).capitalize()


def fake_app(rng):
    return fake_name(rng) + rng.choice(APP_SUFFIX)


def fake_city(rng):
    base = fake_name(rng)
    r = rng.random()
    if r < 0.2:
        return f"Saint-{base}"
    if r < 0.35:
        return f"{base}-sur-{rng.choice(['Mer', 'Loire', 'Meuse', 'Seine'])}"
    if r < 0.45:
        return f"{base} {rng.choice(['Ville', 'City'])}"
    return base


VERBS = ["acheter", "appeler", "envoyer", "réviser", "préparer", "finir", "commencer", "réparer", "nettoyer", "ranger", "payer", "réserver",
         "vérifier", "imprimer", "télécharger", "rendre", "récupérer", "laver", "contacter", "écrire", "lire", "regarder", "tester", "enregistrer",
         "mixer", "installer", "sauvegarder", "changer", "commander", "planifier", "organiser", "répondre à", "relancer", "corriger"]
OBJECTS = ["le pain", "les courses", "maman", "le devoir de physique", "le rapport", "la voiture", "le vélo", "ma chambre", "la facture",
           "un billet de train", "les mails", "le dossier", "le contrat", "la prod", "le beat", "le morceau", "le chapitre 4", "les notes de cours",
           "le dentiste", "le médecin", "le loyer", "les plantes", "le chargeur", "les photos", "le projet", "le site web", "la présentation",
           "le colis", "les clés", "le frigo", "le linge", "l'ordinateur", "le serveur", "le portfolio", "le cadeau de Léa", "Thomas", "Sarah",
           "le prof de maths", "la banque", "l'assurance", "le plombier", "la playlist", "le mémoire", "le CV", "la lettre de motivation"]
COMPLEMENTS = ["", "", "", "demain", "ce soir", "lundi", "avant vendredi", "à 18h", "cette semaine", "pour le cours", "avant le week-end",
               "au plus vite", "dans la matinée", "après le sport", "à 10h30"]


def fake_content(rng):
    c = f"{rng.choice(VERBS)} {rng.choice(OBJECTS)}"
    comp = rng.choice(COMPLEMENTS)
    return f"{c} {comp}".strip()


NOUNS = ["facture", "rapport", "devoir", "photo", "contrat", "projet", "exercice", "présentation", "budget", "relevé", "cours", "mixage",
         "partition", "recette", "attestation", "planning", "inventaire", "script", "maquette", "export", "sample", "logo", "liste", "brouillon"]
QUALIF = ["", "", "final", "de stage", "2025", "2026", "de maths", "d'anglais", "edf", "proximus", "v2", "du client", "de juin", "perso",
          "de groupe", "définitif", "scanné", "complet", "de vacances", "du studio"]


def fake_query(rng):
    return f"{rng.choice(NOUNS)} {rng.choice(QUALIF)}".strip()


SUBJECTS = ["les trous noirs", "la mitose", "le jazz", "la Grèce antique", "les impôts belges", "le climat", "les volcans", "la monnaie",
            "les abeilles", "le cerveau", "les vaccins", "les satellites", "le football", "la chimie organique", "les langues", "Mozart",
            "la Lune", "les pyramides", "le cinéma", "la mode", "les élections", "le café", "les océans", "l'énergie solaire", "les robots",
            "le stoïcisme", "la Renaissance", "les algorithmes", "l'économie", "les requins", "le sommeil", "la musique classique",
            "la trap", "le mixage", "les accords", "la batterie", "le piano", "la guerre froide", "les mangas", "l'IA", "les jeux vidéo",
            "le marketing", "la comptabilité", "le droit du travail", "la géométrie", "les probabilités", "les fractions", "la grammaire"]


def fake_topic(rng):
    t = rng.choice(SUBJECTS + TOPICS)
    r = rng.random()
    if r < 0.15:
        return f"{t} en {rng.choice(['2024', '2025', '2026', 'Belgique', 'France', 'Europe'])}"
    if r < 0.25:
        return f"{rng.choice(['la coupe du monde', 'les jeux olympiques', 'le festival de Cannes', 'les élections', 'le salon de l auto'])} {rng.choice(['2026', '2028', 'de cette année'])}"
    return t


# ----------------------------------------------------------------------------- Durées

NUM_WORDS = {1: "une", 2: "deux", 3: "trois", 4: "quatre", 5: "cinq", 6: "six", 7: "sept", 8: "huit", 9: "neuf", 10: "dix", 12: "douze",
             15: "quinze", 20: "vingt", 25: "vingt-cinq", 30: "trente", 40: "quarante", 45: "quarante-cinq", 50: "cinquante"}


def duration(rng):
    kind = rng.random()
    if kind < 0.45:
        n = rng.choice([1, 2, 3, 5, 7, 10, 12, 15, 20, 25, 30, 40, 45, 50, 60, 90])
        unit = rng.choice(["minutes", "minutes", "min", "mn", "minute" if n == 1 else "minutes"])
        num = NUM_WORDS.get(n) if rng.random() < 0.3 and n in NUM_WORDS else str(n)
        if num == "une" and unit.startswith("minute"):
            unit = "minute"
        return f"{num} {unit}"
    if kind < 0.65:
        n = rng.choice([5, 10, 15, 20, 30, 45, 90])
        return f"{rng.choice([str(n), NUM_WORDS.get(n, str(n))])} {rng.choice(['secondes', 'sec', 's'])}"
    if kind < 0.85:
        n = rng.choice([1, 2, 3])
        return rng.choice([f"{n} heure{'s' if n > 1 else ''}", f"{NUM_WORDS[n] if n > 1 else 'une'} heure{'s' if n > 1 else ''}", f"{n}h", f"{n} h"])
    return rng.choice(["1h30", "1 heure et demie", "une heure et demie", "2h15", "1h05", "une demi-heure", "un quart d'heure", "trois quarts d'heure"])


# ----------------------------------------------------------------------------- Modèles de phrases

POLITE_PRE = ["", "", "", "", "nova ", "hey nova ", "dis ", "stp ", "est-ce que tu peux ", "tu peux ", "peux-tu ", "pourrais-tu ",
              "j'aimerais que tu ", "je veux que tu ", "vas-y "]
POLITE_POST = ["", "", "", "", " stp", " s'il te plaît", " s'il te plait", " merci", " vite", " !", " ?", " please", " steuplé"]


def polite(rng, verb_phrase):
    pre = rng.choice(POLITE_PRE)
    return f"{pre}{verb_phrase}{rng.choice(POLITE_POST)}".strip()


def gen_minuteur(rng, test):
    d = duration(rng)
    forms = [
        f"mets un minuteur de {d}", f"minuteur {d}", f"lance un minuteur de {d}", f"un minuteur de {d}", f"timer {d}",
        f"mets un timer de {d}", f"compte à rebours de {d}", f"préviens-moi dans {d}", f"réveille-moi dans {d}",
        f"programme un minuteur pour {d}", f"démarre un chrono de {d}", f"sonne dans {d}", f"minuteur pour {d}",
        f"minuteur {d} pour {rng.choice(['les pâtes', 'le thé', 'le riz', 'les oeufs', 'la sieste', 'le four'])}",
        f"mets {d} de minuteur", f"timer de {d} stp", f"{d} de minuteur",
    ]
    if test:
        forms = [f"enclenche un minuteur de {d}", f"tu me mets {d} au minuteur", f"alerte-moi dans {d}", f"{d} de minuteur"]
    if rng.random() < 0.3 and not test:
        what = rng.choice(["sortir le gâteau", "retourner la viande", "prendre mes médicaments", "faire une pause", "relancer le rendu",
                           "appeler Paul", "éteindre le four", "aller chercher le linge"]) if rng.random() < 0.4 else fake_content(rng)
        t = rng.choice([f"rappelle-moi de {what} dans {d}", f"dans {d} rappelle-moi de {what}", f"préviens-moi dans {d} pour {what}"])
        return t, [("duree", d), ("contenu", what)]
    return polite(rng, rng.choice(forms)), [("duree", d)]


def gen_ouvrir_app(rng, test):
    app = rng.choice(APPS_TEST) if test else (rng.choice(APPS_TRAIN) if rng.random() < 0.55 else fake_app(rng))
    art = rng.choice(["", "", "", "l'application ", "l'app ", "l'appli ", "le logiciel "])
    verbs = ["ouvre", "lance", "démarre", "ouvre-moi", "lance-moi", "mets", "allume"] if not test else ["fais démarrer", "exécute", "ouvre donc"]
    v = rng.choice(verbs)
    form = rng.choice([f"{v} {art}{app}", f"{v} {art}{app}", f"je veux ouvrir {art}{app}", f"j'ai besoin de {app}", f"ouvrir {app}"])
    return polite(rng, form), [("app", app)]


def gen_ouvrir_site(rng, test):
    site = rng.choice(SITES_TEST if test else SITES_TRAIN)
    form = rng.choice([f"ouvre {site}", f"va sur {site}", f"ouvre le site {site}", f"affiche {site}", f"emmène-moi sur {site}",
                       f"je veux aller sur {site}", f"ouvre la page {site}", f"mets {site}"])
    return polite(rng, form), [("site", site)]


def gen_meteo(rng, test):
    city = rng.choice(CITIES_TEST) if test else (rng.choice(CITIES_TRAIN) if rng.random() < 0.6 else fake_city(rng))
    with_city = [f"météo à {city}", f"quel temps fait-il à {city}", f"il fait quel temps à {city}", f"la météo de {city}",
                 f"météo {city}", f"est-ce qu'il pleut à {city}", f"il va pleuvoir à {city} ?", f"la température à {city}",
                 f"quelle est la météo à {city} demain", f"prévisions météo pour {city}", f"il fait froid à {city} ?", f"meteo a {city}"]
    no_city = ["quel temps fait-il", "la météo", "météo", "il fait beau aujourd'hui ?", "est-ce qu'il va pleuvoir", "météo du jour",
               "il fait combien dehors", "quelle température il fait", "faut-il prendre un parapluie"]
    if test:
        with_city = [f"donne-moi le temps qu'il fait à {city}", f"{city} météo", f"ça caille à {city} ?", f"bulletin météo {city}"]
        no_city = ["je dois sortir un manteau ?", "le ciel est comment aujourd'hui"]
    if rng.random() < 0.75:
        return polite(rng, rng.choice(with_city)), [("ville", city)]
    return polite(rng, rng.choice(no_city)), []


def gen_note(rng, test):
    c = rng.choice(NOTES_TEST) if test else (rng.choice(NOTES) if rng.random() < 0.3 else fake_content(rng))
    form = rng.choice([f"note : {c}", f"crée une note {c}", f"prends une note : {c}", f"note que je dois {c}" if not c[0].isupper() else f"note {c}",
                       f"ajoute une note : {c}", f"écris une note : {c}", f"nouvelle note {c}", f"mémo : {c}", f"note ça : {c}",
                       f"crée une note {c}", f"fais une note {c}", f"note-moi {c}", f"ajoute dans mes notes {c}", f"pense à noter {c}", f"note {c}"])
    if test:
        form = rng.choice([f"garde en note : {c}", f"inscris dans mes notes : {c}"])
    return form, [("contenu", c)]


def gen_chercher_fichier(rng, test):
    q = rng.choice(FILE_QUERIES_TEST) if test else (rng.choice(FILE_QUERIES) if rng.random() < 0.4 else fake_query(rng))
    form = rng.choice([f"cherche le fichier {q}", f"trouve le fichier {q}", f"où est mon fichier {q}", f"retrouve le document {q}",
                       f"cherche {q} dans mes fichiers", f"trouve mes fichiers {q}", f"recherche le pdf {q}", f"où j'ai mis le fichier {q}"])
    if test:
        form = rng.choice([f"localise le fichier {q}", f"tu trouves où est {q} dans mes documents ?"])
    return polite(rng, form), [("requete", q)]


def gen_recherche_web(rng, test):
    q = rng.choice(WEB_QUERIES_TEST) if test else (rng.choice(WEB_QUERIES) if rng.random() < 0.3 else fake_topic(rng))
    form = rng.choice([f"cherche sur internet {q}", f"recherche sur le web {q}", f"cherche sur wikipédia {q}", f"regarde sur internet {q}",
                       f"fais une recherche sur {q}", f"google {q}", f"cherche en ligne {q}"])
    return polite(rng, form), [("requete", q)]


def gen_calcul(rng, test):
    a, b = rng.randint(2, 99), rng.randint(2, 30)
    forms = [f"combien font {a} fois {b}", f"calcule {a} + {b}", f"{a} * {b}", f"{a} plus {b}", f"combien fait {a} moins {b}",
             f"{a} divisé par {b}", f"racine carrée de {a * a}", f"résous {b}x + {a} = 0", f"calcule {a}^2", f"{a}x{b}",
             f"combien vaut {a} au carré", f"dérivée de x^{b}", f"résous x^2 - {b} = 0", f"calcul {a}/{b}", f"{a}+{b}*{b}"]
    if test:
        forms = [f"ça fait combien {a} multiplié par {b}", f"tu peux calculer {a} - {b}", f"{a} puissance {b % 5 + 2}"]
    return polite(rng, rng.choice(forms)), []


def gen_conversion(rng, test):
    n = rng.choice([1, 2, 5, 10, 12, 20, 42, 100, 250, 1000])
    pair = rng.choice([("km", "miles"), ("miles", "km"), ("°F", "°C"), ("degrés Celsius", "Fahrenheit"), ("kg", "livres"), ("livres", "kg"),
                       ("mètres", "pieds"), ("litres", "gallons"), ("km/h", "m/s"), ("Go", "Mo"), ("heures", "minutes"), ("cm", "pouces")])
    form = rng.choice([f"{n} {pair[0]} en {pair[1]}", f"convertis {n} {pair[0]} en {pair[1]}", f"combien font {n} {pair[0]} en {pair[1]}",
                       f"{n} {pair[0]} ça fait combien en {pair[1]}", f"conversion {n} {pair[0]} en {pair[1]}"])
    return polite(rng, form), []


def simple(forms_train, forms_test=None):
    def gen(rng, test):
        forms = forms_test if (test and forms_test) else forms_train
        return polite(rng, rng.choice(forms)) if rng.random() < 0.3 else rng.choice(forms), []
    return gen


def gen_lancer_de(rng, test):
    if rng.random() < 0.4:
        f = str(rng.choice([4, 6, 8, 10, 12, 20, 100]))
        return polite(rng, rng.choice([f"lance un dé à {f} faces", f"jette un dé de {f}", f"dé {f} faces", f"lance un d{f}"])), [("faces", f)]
    return polite(rng, rng.choice(["lance un dé", "jette un dé", "lance les dés", "un coup de dé", "tire un dé"])), []


GENERATORS = {
    "salutation": simple(["bonjour", "salut", "coucou", "hello", "hey", "yo", "bonsoir", "wesh", "salut nova", "bonjour nova", "slt", "bjr",
                          "re", "hello nova", "hey salut", "bonjour à toi", "salut toi", "cc", "hi", "bonjour bonjour"],
                         ["bien le bonjour", "salutations", "hé ho", "coucou toi"]),
    "comment_ca_va": simple(["ça va ?", "comment tu vas", "tu vas bien ?", "quoi de neuf", "la forme ?", "comment ça va", "salut ça va",
                             "bonjour, comment allez-vous ?", "ça roule ?", "tu vas bien nova ?", "comment vas-tu", "cv ?", "ca va",
                             "bien ou quoi", "ça dit quoi", "tu vas bien aujourd'hui ?", "comment ça se passe pour toi", "ça va bien ?", "et toi ça va ?",
                             "la pêche ?", "tu tiens le coup ?"],
                            ["tu te portes bien ?", "comment se passe ta journée", "tout va bien de ton côté ?"]),
    "remerciement": simple(["merci", "merci beaucoup", "merci bien", "c'est gentil", "top merci", "parfait merci", "super, merci !",
                            "merci nova", "mercii", "thanks", "trop bien merci", "nickel merci", "je te remercie", "mrc", "merci pour ton aide"],
                           ["merci infiniment", "c'est cool de ta part", "grand merci"]),
    "au_revoir": simple(["au revoir", "bye", "à plus", "bonne nuit", "à demain", "ciao", "salut, à plus tard", "a+", "bonne soirée",
                         "je te laisse", "on se reparle plus tard", "bye bye", "à la prochaine", "tchao", "bonne journée"],
                        ["je file", "à tout à l'heure", "allez, à bientôt"]),
    "identite": simple(["qui es-tu", "t'es qui", "comment tu t'appelles", "tu es une ia ?", "qui t'a créé", "c'est quoi ton nom",
                        "tu es quoi", "présente-toi", "tu es un robot ?", "quel est ton nom", "tu es chatgpt ?", "tu es qui toi",
                        "qui t'a fabriqué", "t'es une vraie personne ?"],
                       ["dis-moi qui tu es", "tu viens d'où", "quel modèle es-tu"]),
    "capacites": simple(["que sais-tu faire", "tu peux faire quoi", "aide", "quelles sont tes fonctions", "à quoi tu sers",
                         "qu'est-ce que tu sais faire", "liste tes fonctionnalités", "comment tu peux m'aider", "tu sers à quoi",
                         "montre-moi ce que tu sais faire", "help"],
                        ["c'est quoi tes compétences", "en quoi peux-tu m'être utile"]),
    "heure": simple(["quelle heure est-il", "il est quelle heure", "l'heure stp", "t'as l'heure", "quelle heure", "donne-moi l'heure",
                     "il est quelle heure là", "c'est quelle heure", "heure actuelle", "dis-moi l'heure"],
                    ["tu as l'heure s'il te plaît", "on est à quelle heure"]),
    "date": simple(["on est quel jour", "quelle est la date", "c'est quoi la date aujourd'hui", "on est le combien", "quel jour sommes-nous",
                    "la date d'aujourd'hui", "on est quel jour aujourd'hui", "c'est quel jour", "quelle date on est"],
                   ["rappelle-moi la date du jour", "aujourd'hui c'est quel jour"]),
    "annuler_minuteur": simple(["arrête le minuteur", "annule le minuteur", "stop le timer", "supprime le minuteur", "stop minuteur",
                                "annule le timer", "coupe le minuteur", "plus besoin du minuteur", "arrête le compte à rebours", "annule mon timer",
                                "annule mon minuteur", "stoppe le minuteur", "éteins le minuteur", "arrête le timer"],
                               ["désactive le minuteur", "enlève le timer"]),
    "infos_systeme": simple(["combien de ram j'ai", "c'est quoi mon processeur", "infos système", "quelle version de macos j'ai",
                             "infos sur mon ordinateur", "config de mon pc", "caractéristiques de mon mac", "combien de mémoire a mon ordi",
                             "quel processeur j'ai", "informations système"],
                            ["détails techniques de ma machine", "mon ordi a combien de ram"]),
    "blague": simple(["raconte une blague", "fais-moi rire", "t'as une blague ?", "dis une blague", "une blague stp", "raconte-moi une blague",
                      "fais-moi une blague", "blague", "tu connais une blague ?", "une petite blague"],
                     ["sors-moi une blague", "j'ai besoin de rire"]),
    "pile_ou_face": simple(["pile ou face", "lance une pièce", "pile ou face ?", "tire à pile ou face", "joue à pile ou face",
                            "fais un pile ou face", "lance une pièce de monnaie"],
                           ["fais tourner une pièce", "pile face"]),
    "minuteur": gen_minuteur,
    "ouvrir_app": gen_ouvrir_app,
    "ouvrir_site": gen_ouvrir_site,
    "meteo": gen_meteo,
    "note": gen_note,
    "chercher_fichier": gen_chercher_fichier,
    "recherche_web": gen_recherche_web,
    "calcul": gen_calcul,
    "conversion": gen_conversion,
    "lancer_de": gen_lancer_de,
}

# Phrases « autre » : questions générales, demandes de rédaction… et pièges qui ressemblent à des commandes.
TRAPS = ["je suis stressé pour mon exam demain", "qui est Cristiano Ronaldo", "qui était Napoléon", "qui a gagné la coupe du monde 2018",
         "le temps qu'il faut pour cuire un oeuf", "combien de temps pour apprendre le piano", "j'ai pas le temps aujourd'hui",
         "ma note de français est mauvaise", "comment noter un exercice", "comment organiser mes fichiers", "c'est quoi un dossier zip",
         "à quelle heure ouvre la poste en général", "pourquoi les journées sont plus courtes en hiver", "comment on fait une bonne prod",
         "comment ouvrir un fichier zip", "le minuteur de ma cuisine est cassé", "je cherche un cadeau pour ma mère", "trouve-moi une idée de nom",
         "j'ai mal à la tête", "comment devenir riche", "c'est quoi la météo spatiale", "explique les fuseaux horaires",
         "écris un message pour souhaiter bon anniversaire", "aide-moi à trouver un titre pour mon morceau", "tu préfères le chat ou le chien",
         "les applications les plus utilisées au monde", "l'histoire de Google", "quel est le meilleur navigateur", "conseille-moi un film",
         "comment ouvrir un compte bancaire", "le temps passe vite", "j'ai eu une bonne note en maths", "combien de temps dure un match de foot",
         "ouvre ton esprit", "comment lancer une entreprise", "la météo influence-t-elle l'humeur ?", "explique-moi comment marche un minuteur",
         "j'ai perdu mes fichiers, que faire", "pourquoi le ciel est bleu", "écris une chanson sur la pluie", "c'est quoi un fichier pdf",
         "quelle heure est-il à Tokyo", "comment calculer une moyenne", "raconte-moi l'histoire de Spotify", "comment fonctionne internet",
         "aide-moi à écrire un mail à mon prof", "je me sens triste aujourd'hui", "donne-moi des conseils pour dormir",
         "quelle est la différence entre Word et Pages", "comment devenir développeur", "traduis bonjour en anglais",
         "résume ce texte", "corrige mes fautes", "écris une fonction python qui trie une liste", "c'est quoi la recette des crêpes",
         "que penses-tu de l'IA", "comment apprendre la guitare", "propose-moi un programme de sport", "parle-moi de toi en détail"]
TRAPS_TEST = ["comment ouvrir une bouteille de vin", "le temps de cuisson des pâtes", "note de synthèse sur la crise de 1929",
              "explique le calcul différentiel", "comment chercher un emploi", "l'application de la loi en France"]

AUTRE_FORMS = ["comment on fait {t}", "j'ai une question sur {t}", "je suis nul en {t}", "tu connais {t} ?", "qu'est-ce que tu penses de {t}",
               "raconte-moi l'histoire de {t}", "écris un texte sur {t}", "donne-moi des conseils pour {t}", "qui a inventé {t}",
               "c'est quoi le lien entre {t} et l'école", "aide-moi pour {t}", "est-ce que {t} c'est difficile", "{t} ou pas ?",
               "explique-moi {t}", "c'est quoi {t}", "parle-moi de {t}", "résume {t}", "donne-moi un exemple sur {t}",
               "j'ai un exposé sur {t}, tu peux m'aider", "fais un plan sur {t}", "quelle est l'origine de {t}", "pourquoi {t} est important",
               "écris un poème sur {t}", "donne-moi 3 idées sur {t}", "je comprends pas {t}", "quiz sur {t}", "{t} expliqué simplement",
               "qu'est-ce que {t}", "définition de {t}", "comment on étudie {t}"]
AUTRE_FORMS_TEST = ["vulgarise {t}", "fais-moi une fiche de révision sur {t}", "je bloque sur {t}"]


def gen_autre(rng, test):
    if rng.random() < 0.35:
        return rng.choice(TRAPS_TEST if test else TRAPS), []
    t = rng.choice(TOPICS)
    return polite(rng, rng.choice(AUTRE_FORMS_TEST if test else AUTRE_FORMS).format(t=t)), []


GENERATORS["autre"] = gen_autre

COUNTS = {"autre": 9000, "comment_ca_va": 700, "annuler_minuteur": 600, "identite": 600, "minuteur": 2200, "ouvrir_app": 2200, "meteo": 1800, "note": 1300, "ouvrir_site": 1200, "chercher_fichier": 1100,
          "recherche_web": 900, "calcul": 1100, "conversion": 900, "lancer_de": 500}
DEFAULT_COUNT = 450

# ----------------------------------------------------------------------------- Bruit réaliste

ACCENTS = str.maketrans("éèêàâùûîïôç", "eeeaauuiioc")


def noisy(text, slots, rng):
    """Fautes de frappe, absence d'accents, majuscules — sans abîmer les valeurs à repérer."""
    protected = {w.lower() for _, v in slots for w in v.split()}
    words = text.split(" ")
    out = []
    for w in words:
        if w.lower() not in protected and len(w) > 3 and rng.random() < 0.08:
            i = rng.randrange(len(w) - 1)
            op = rng.random()
            if op < 0.4:
                w = w[:i] + w[i + 1:]  # lettre oubliée
            elif op < 0.8:
                w = w[:i] + w[i + 1] + w[i] + w[i + 2:]  # lettres inversées
            else:
                w = w[:i] + w[i] + w[i:]  # lettre doublée
        out.append(w)
    text = " ".join(out)
    if rng.random() < 0.2:
        text = text.translate(ACCENTS)
        slots = [(k, v.translate(ACCENTS)) for k, v in slots]
    r = rng.random()
    if r < 0.15:
        text = text[:1].upper() + text[1:]
    elif r < 0.18:
        text = text.upper()
        slots = [(k, v.upper()) for k, v in slots]
    return text, slots


def build(seed, test):
    rng = random.Random(seed)
    rows = []
    for intent, gen in GENERATORS.items():
        n = COUNTS.get(intent, DEFAULT_COUNT)
        if test:
            n = max(60, n // 6)
        for _ in range(n):
            text, slots = gen(rng, test)
            text, slots = noisy(text, slots, rng)
            rows.append((intent, text, slots))
    rng.shuffle(rows)
    return rows


def write(path, rows):
    with open(path, "w", encoding="utf-8") as f:
        for intent, text, slots in rows:
            s = ";".join(f"{k}={v}" for k, v in slots)
            f.write(f"{intent}\t{text}\t{s}\n")


if __name__ == "__main__":
    train = build(2026, False)
    test = build(1019, True)
    write(os.path.join(HERE, "train.tsv"), train)
    write(os.path.join(HERE, "test.tsv"), test)
    print(f"train : {len(train)} exemples · test : {len(test)} exemples · {len(GENERATORS)} intentions")
