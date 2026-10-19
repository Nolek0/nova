# Nova 1 — rapport d'entraînement

Modèle de compréhension des commandes de NOVA, créé et entraîné de zéro (aucun poids extérieur).

| | |
|---|---|
| Paramètres | 1621002 |
| Taille du fichier | 1.9 Mo |
| Intentions | 25 |
| Exemples d'entraînement | 30404 |
| Durée d'entraînement | 74 s |
| Temps d'analyse moyen | 0.07 ms par phrase |

## Précision

| Jeu | Intention | Informations extraites |
|---|---|---|
| Entraînement (échantillon) | 100.0 % | 98.4 % |
| Test (vocabulaire jamais vu) | 91.3 % | 76.9 % |
| Phrases réalistes écrites à la main | 97.3 % | 100.0 % |

## Intentions

`autre`, `annuler_minuteur`, `au_revoir`, `blague`, `calcul`, `capacites`, `chercher_fichier`, `comment_ca_va`, `conversion`, `date`, `dessin`, `heure`, `identite`, `infos_systeme`, `lancer_de`, `meteo`, `minuteur`, `note`, `ouvrir_app`, `ouvrir_site`, `pile_ou_face`, `recherche_web`, `remerciement`, `resume`, `salutation`

## Erreurs restantes (phrases réalistes)

- « résume la révolution française en 5 lignes » → resume (63 %) au lieu de autre
- « combien d'habitants en belgique » → conversion (32 %) au lieu de autre
