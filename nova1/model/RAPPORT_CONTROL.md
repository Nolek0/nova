# Nova Control — rapport d'entraînement

Modèle de compréhension des commandes de NOVA, créé et entraîné de zéro (aucun poids extérieur).

| | |
|---|---|
| Paramètres | 1623519 |
| Taille du fichier | 1.9 Mo |
| Intentions | 30 |
| Exemples d'entraînement | 371460 |
| Durée d'entraînement | 877 s |
| Temps d'analyse moyen | 0.10 ms par phrase |

## Précision

| Jeu | Intention | Informations extraites |
|---|---|---|
| Entraînement (échantillon) | 99.5 % | 96.4 % |
| Test (vocabulaire jamais vu) | 98.0 % | 89.1 % |
| Phrases réalistes écrites à la main | 100.0 % | 75.8 % |

## Intentions

`autre`, `annuler`, `capture_ecran`, `cliquer`, `coller`, `copier`, `couper`, `creer_dossier`, `defiler`, `ecrire`, `enregistrer`, `envoyer_mail`, `envoyer_message`, `fermer_app`, `fermer_onglet`, `lecture_pause`, `menu`, `muet`, `nouveau_document`, `nouvel_onglet`, `nouvelle_note`, `ouvrir_app`, `ouvrir_site`, `piste_suivante`, `plein_ecran`, `recherche_web`, `rechercher_page`, `retablir`, `tout_selectionner`, `volume`

## Erreurs restantes (phrases réalistes)

- « envoie un mail à paul@exemple.fr pour dire que je serai en retard » : infos [("destinataire", "paul@exemple.fr"), ("message", "je serai en retard")] au lieu de [("destinataire", "paul@exemple.fr"), ("message", "que je serai en retard")]
- « crée une note Courses avec pain, lait, œufs » : infos [("contenu", "courses avec pain, lait"), ("texte", ", œufs")] au lieu de [("contenu", "pain, lait, œufs"), ("titre", "courses")]
- « écris bonjour tout le monde » : infos [("texte", "bonjour tout")] au lieu de [("texte", "bonjour tout le monde")]
- « dis à Léa sur insta que la soirée est annulée » : infos [("app", "insta"), ("destinataire", "léa"), ("message", "la")] au lieu de [("app", "insta"), ("destinataire", "léa"), ("message", "la soirée est annulée")]
- « écris à @lea.m sur insta : tu viens demain ? » : infos [("app", "insta"), ("destinataire", "@lea.m"), ("message", "tu viens demain")] au lieu de [("app", "insta"), ("destinataire", "@lea.m"), ("message", "tu viens demain ?")]
- « envoie un message sur outlook à paul@exemple.fr avec objet salut et message coucou » : infos [("destinataire", "paul@exemple.fr"), ("message", "salut et message coucou")] au lieu de [("app", "outlook"), ("destinataire", "paul@exemple.fr"), ("message", "coucou"), ("objet", "salut")]
- « Envoyer à Ferspi sur Discord salut ça va et demande lui si il a de l'argent à me prêter stp » : infos [("app", "discord"), ("destinataire", "ferspi"), ("message", "salut ça va et demande lui si il a de l'argent à me prêter")] au lieu de [("app", "discord"), ("destinataire", "ferspi"), ("message", "salut ça va et demande lui si il a de l'argent à me prêter stp")]
- « envoie à Léo sur insta coucou et dis lui que je passe samedi » : infos [("app", "insta"), ("destinataire", "léo"), ("message", "coucou et dis lui que je passe")] au lieu de [("app", "insta"), ("destinataire", "léo"), ("message", "coucou et dis lui que je passe samedi")]
