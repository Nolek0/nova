# Nova Control — rapport d'entraînement

Modèle de compréhension des commandes de NOVA, créé et entraîné de zéro (aucun poids extérieur).

| | |
|---|---|
| Paramètres | 1623326 |
| Taille du fichier | 1.9 Mo |
| Intentions | 29 |
| Exemples d'entraînement | 24170 |
| Durée d'entraînement | 68 s |
| Temps d'analyse moyen | 0.06 ms par phrase |

## Précision

| Jeu | Intention | Informations extraites |
|---|---|---|
| Entraînement (échantillon) | 99.5 % | 97.0 % |
| Test (vocabulaire jamais vu) | 98.7 % | 86.6 % |
| Phrases réalistes écrites à la main | 100.0 % | 81.2 % |

## Intentions

`autre`, `annuler`, `capture_ecran`, `cliquer`, `coller`, `copier`, `couper`, `creer_dossier`, `defiler`, `ecrire`, `enregistrer`, `envoyer_mail`, `fermer_app`, `fermer_onglet`, `lecture_pause`, `menu`, `muet`, `nouveau_document`, `nouvel_onglet`, `nouvelle_note`, `ouvrir_app`, `ouvrir_site`, `piste_suivante`, `plein_ecran`, `recherche_web`, `rechercher_page`, `retablir`, `tout_selectionner`, `volume`

## Erreurs restantes (phrases réalistes)

- « envoie un mail à paul@exemple.fr pour dire que je serai en retard » : infos [("destinataire", "paul@exemple.fr"), ("message", "je serai en retard")] au lieu de [("destinataire", "paul@exemple.fr"), ("message", "que je serai en retard")]
- « crée une note Courses avec pain, lait, œufs » : infos [("contenu", "pain, lait"), ("texte", ", œufs"), ("titre", "courses")] au lieu de [("contenu", "pain, lait, œufs"), ("titre", "courses")]
- « écris bonjour tout le monde » : infos [("texte", "bonjour")] au lieu de [("texte", "bonjour tout le monde")]
