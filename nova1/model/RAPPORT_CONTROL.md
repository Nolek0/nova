# Nova Control — rapport d'entraînement

Modèle de compréhension des commandes de NOVA, créé et entraîné de zéro (aucun poids extérieur).

| | |
|---|---|
| Paramètres | 1623519 |
| Taille du fichier | 1.9 Mo |
| Intentions | 30 |
| Exemples d'entraînement | 371388 |
| Durée d'entraînement | 591 s |
| Temps d'analyse moyen | 0.09 ms par phrase |

## Précision

| Jeu | Intention | Informations extraites |
|---|---|---|
| Entraînement (échantillon) | 99.5 % | 96.7 % |
| Test (vocabulaire jamais vu) | 98.7 % | 88.9 % |
| Phrases réalistes écrites à la main | 100.0 % | 79.3 % |

## Intentions

`autre`, `annuler`, `capture_ecran`, `cliquer`, `coller`, `copier`, `couper`, `creer_dossier`, `defiler`, `ecrire`, `enregistrer`, `envoyer_mail`, `envoyer_message`, `fermer_app`, `fermer_onglet`, `lecture_pause`, `menu`, `muet`, `nouveau_document`, `nouvel_onglet`, `nouvelle_note`, `ouvrir_app`, `ouvrir_site`, `piste_suivante`, `plein_ecran`, `recherche_web`, `rechercher_page`, `retablir`, `tout_selectionner`, `volume`

## Erreurs restantes (phrases réalistes)

- « envoie un mail à paul@exemple.fr pour dire que je serai en retard » : infos [("destinataire", "paul@exemple.fr"), ("message", "je serai en retard")] au lieu de [("destinataire", "paul@exemple.fr"), ("message", "que je serai en retard")]
- « crée une note Courses avec pain, lait, œufs » : infos [("contenu", "courses avec pain"), ("texte", "lait, œufs")] au lieu de [("contenu", "pain, lait, œufs"), ("titre", "courses")]
- « écris bonjour tout le monde » : infos [("message", "tout"), ("texte", "bonjour")] au lieu de [("texte", "bonjour tout le monde")]
- « dis à Léa sur insta que la soirée est annulée » : infos [("app", "insta"), ("destinataire", "léa"), ("message", "la soirée")] au lieu de [("app", "insta"), ("destinataire", "léa"), ("message", "la soirée est annulée")]
- « écris à @lea.m sur insta : tu viens demain ? » : infos [("app", "insta"), ("destinataire", "@lea.m"), ("message", "tu viens demain")] au lieu de [("app", "insta"), ("destinataire", "@lea.m"), ("message", "tu viens demain ?")]
- « envoie un message sur outlook à paul@exemple.fr avec objet salut et message coucou » : infos [("destinataire", "paul@exemple.fr"), ("message", "coucou"), ("objet", "salut")] au lieu de [("app", "outlook"), ("destinataire", "paul@exemple.fr"), ("message", "coucou"), ("objet", "salut")]
