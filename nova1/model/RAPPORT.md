# Nova 1 — rapport d'entraînement

Modèle de compréhension des commandes de NOVA, créé et entraîné de zéro (aucun poids extérieur).

| | |
|---|---|
| Paramètres | 1621002 |
| Taille du fichier | 1.9 Mo |
| Intentions | 25 |
| Exemples d'entraînement | 30404 |
| Durée d'entraînement | 81 s |
| Temps d'analyse moyen | 0.08 ms par phrase |

## Précision

| Jeu | Intention | Informations extraites |
|---|---|---|
| Entraînement (échantillon) | 99.9 % | 99.0 % |
| Test (vocabulaire jamais vu) | 90.9 % | 76.1 % |
| Phrases réalistes écrites à la main | 98.9 % | 90.6 % |

## Intentions

`autre`, `annuler_minuteur`, `au_revoir`, `blague`, `calcul`, `capacites`, `chercher_fichier`, `comment_ca_va`, `conversion`, `date`, `dessin`, `heure`, `identite`, `infos_systeme`, `lancer_de`, `meteo`, `minuteur`, `note`, `ouvrir_app`, `ouvrir_site`, `pile_ou_face`, `recherche_web`, `remerciement`, `resume`, `salutation`

## Erreurs restantes (phrases réalistes)

- « combien d'habitants en belgique » → conversion (19 %) au lieu de autre
- « tu peux me dessiner un chat qui dort » : infos [("sujet", "un chat")] au lieu de [("sujet", "un chat qui dort")]
- « fais un dessin d'une maison au bord de la mer » : infos [("contenu", "de la"), ("sujet", "mer"), ("sujet", "une maison au bord")] au lieu de [("sujet", "une maison au bord de la mer")]
- « draw a duck » : infos [] au lieu de [("sujet", "a duck")]
